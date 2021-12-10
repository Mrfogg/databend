// Copyright 2021 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::BTreeSet;
use std::sync::Arc;

use async_raft::config::Config;
use async_raft::Raft;
use async_raft::RaftMetrics;
use async_raft::SnapshotPolicy;
use common_base::tokio;
use common_base::tokio::sync::watch;
use common_base::tokio::sync::Mutex;
use common_base::tokio::sync::RwLockReadGuard;
use common_base::tokio::task::JoinHandle;
use common_exception::prelude::ErrorCode;
use common_exception::prelude::ToErrorCode;
use common_meta_api::MetaApi;
use common_meta_raft_store::config::RaftConfig;
use common_meta_raft_store::state_machine::AppliedState;
use common_meta_raft_store::state_machine::StateMachine;
use common_meta_raft_store::state_machine::TableLookupKey;
use common_meta_raft_store::state_machine::TableLookupValue;
use common_meta_types::Cmd;
use common_meta_types::ListTableReq;
use common_meta_types::LogEntry;
use common_meta_types::Node;
use common_meta_types::NodeId;
use common_meta_types::SeqV;
use common_meta_types::TableInfo;
use common_meta_types::TableMeta;
use common_tracing::tracing;
use common_tracing::tracing::Instrument;

use crate::errors::ConnectionError;
use crate::errors::ForwardToLeader;
use crate::errors::MetaError;
use crate::meta_service::message::AdminRequest;
use crate::meta_service::message::AdminResponse;
use crate::meta_service::meta_leader::MetaLeader;
use crate::meta_service::AdminRequestInner;
use crate::meta_service::JoinRequest;
use crate::meta_service::MetaServiceImpl;
use crate::meta_service::Network;
use crate::proto::meta_service_client::MetaServiceClient;
use crate::proto::meta_service_server::MetaServiceServer;
use crate::store::MetaRaftStore;
use crate::Opened;

// MetaRaft is a impl of the generic Raft handling meta data R/W.
pub type MetaRaft = Raft<LogEntry, AppliedState, Network, MetaRaftStore>;

// MetaNode is the container of meta data related components and threads, such as storage, the raft node and a raft-state monitor.
pub struct MetaNode {
    // metrics subscribes raft state changes. The most important field is the leader node id, to which all write operations should be forward.
    pub metrics_rx: watch::Receiver<RaftMetrics>,
    pub sto: Arc<MetaRaftStore>,
    pub raft: MetaRaft,
    pub running_tx: watch::Sender<()>,
    pub running_rx: watch::Receiver<()>,
    pub join_handles: Mutex<Vec<JoinHandle<common_exception::Result<()>>>>,
}

impl Opened for MetaNode {
    fn is_opened(&self) -> bool {
        self.sto.is_opened()
    }
}

pub struct MetaNodeBuilder {
    node_id: Option<NodeId>,
    raft_config: Option<Config>,
    sto: Option<Arc<MetaRaftStore>>,
    monitor_metrics: bool,
    addr: Option<String>,
}

impl MetaNodeBuilder {
    pub async fn build(mut self) -> common_exception::Result<Arc<MetaNode>> {
        let node_id = self
            .node_id
            .ok_or_else(|| ErrorCode::InvalidConfig("node_id is not set"))?;

        let config = self
            .raft_config
            .take()
            .ok_or_else(|| ErrorCode::InvalidConfig("config is not set"))?;

        let sto = self
            .sto
            .take()
            .ok_or_else(|| ErrorCode::InvalidConfig("sto is not set"))?;

        let net = Network::new(sto.clone());

        let raft = MetaRaft::new(node_id, Arc::new(config), Arc::new(net), sto.clone());
        let metrics_rx = raft.metrics();

        let (tx, rx) = watch::channel::<()>(());

        let mn = Arc::new(MetaNode {
            metrics_rx: metrics_rx.clone(),
            sto: sto.clone(),
            raft,
            running_tx: tx,
            running_rx: rx,
            join_handles: Mutex::new(Vec::new()),
        });

        if self.monitor_metrics {
            tracing::info!("about to subscribe raft metrics");
            MetaNode::subscribe_metrics(mn.clone(), metrics_rx).await;
        }

        let addr = if let Some(a) = self.addr.take() {
            a
        } else {
            sto.get_node_addr(&node_id).await?
        };
        tracing::info!("about to start raft grpc on {}", addr);

        MetaNode::start_grpc(mn.clone(), &addr).await?;

        Ok(mn)
    }

    pub fn node_id(mut self, node_id: NodeId) -> Self {
        self.node_id = Some(node_id);
        self
    }
    pub fn sto(mut self, sto: Arc<MetaRaftStore>) -> Self {
        self.sto = Some(sto);
        self
    }
    pub fn addr(mut self, a: String) -> Self {
        self.addr = Some(a);
        self
    }
    pub fn monitor_metrics(mut self, b: bool) -> Self {
        self.monitor_metrics = b;
        self
    }
}

impl MetaNode {
    pub fn builder(config: &RaftConfig) -> MetaNodeBuilder {
        let raft_config = MetaNode::new_raft_config(config);

        MetaNodeBuilder {
            node_id: None,
            raft_config: Some(raft_config),
            sto: None,
            monitor_metrics: true,
            addr: None,
        }
    }

    pub fn new_raft_config(config: &RaftConfig) -> Config {
        // TODO(xp): configure cluster name.

        let hb = config.heartbeat_interval;

        Config::build("foo_cluster".into())
            .heartbeat_interval(hb)
            // Choose a rational value for election timeout.
            .election_timeout_min(hb * 8)
            .election_timeout_max(hb * 12)
            .install_snapshot_timeout(config.install_snapshot_timeout)
            .snapshot_policy(SnapshotPolicy::LogsSinceLast(
                config.snapshot_logs_since_last,
            ))
            .validate()
            .expect("building raft Config from databend-metasrv config")
    }

    /// Start the grpc service for raft communication and meta operation API.
    #[tracing::instrument(level = "info", skip(mn))]
    pub async fn start_grpc(mn: Arc<MetaNode>, addr: &str) -> common_exception::Result<()> {
        let mut rx = mn.running_rx.clone();

        let meta_srv_impl = MetaServiceImpl::create(mn.clone());
        let meta_srv = MetaServiceServer::new(meta_srv_impl);

        let addr_str = addr.to_string();
        let addr = addr.parse::<std::net::SocketAddr>()?;
        let node_id = mn.sto.id;

        let srv = tonic::transport::Server::builder().add_service(meta_srv);

        let h = tokio::spawn(async move {
            srv.serve_with_shutdown(addr, async move {
                let _ = rx.changed().await;
                tracing::info!(
                    "signal received, shutting down: id={} {} ",
                    node_id,
                    addr_str
                );
            })
            .await
            .map_err_to_code(ErrorCode::MetaServiceError, || "fail to serve")?;

            Ok::<(), common_exception::ErrorCode>(())
        });

        let mut jh = mn.join_handles.lock().await;
        jh.push(h);
        Ok(())
    }

    /// Open or create a metasrv node.
    /// Optionally boot a single node cluster.
    /// 1. If `open` is `Some`, try to open an existent one.
    /// 2. If `create` is `Some`, try to create an one in non-voter mode.
    /// 3. If `init_cluster` is `Some` and it is just created, try to initialize a single-node cluster.
    ///
    /// TODO(xp): `init_cluster`: pass in a Map<id, address> to initialize the cluster.
    #[tracing::instrument(level = "info", skip(config), fields(config_id=config.config_id.as_str()))]
    pub async fn open_create_boot(
        config: &RaftConfig,
        open: Option<()>,
        create: Option<()>,
        init_cluster: Option<Vec<String>>,
    ) -> common_exception::Result<Arc<MetaNode>> {
        let mut config = config.clone();

        // Always disable fsync on mac.
        // Because there are some integration tests running on mac VM.
        //
        // On mac File::sync_all() takes 10 ms ~ 30 ms, 500 ms at worst, which very likely to fail a test.
        if cfg!(target_os = "macos") {
            tracing::warn!("Disabled fsync for meta data tests. fsync on mac is quite slow");
            config.no_sync = true;
        }

        let sto = MetaRaftStore::open_create(&config, open, create).await?;
        let is_open = sto.is_opened();
        let sto = Arc::new(sto);

        let mut builder = MetaNode::builder(&config).sto(sto.clone());

        if is_open {
            // read id from sto, read listening addr from sto
            builder = builder.node_id(sto.id);
        } else {
            // read id from config, read listening addr from config.
            builder = builder.node_id(config.id).addr(config.raft_api_addr());
        }

        let mn = builder.build().await?;

        tracing::info!("MetaNode started: {:?}", config);

        if !is_open {
            if let Some(_addrs) = init_cluster {
                mn.init_cluster(config.raft_api_addr()).await?;
            }
        }

        Ok(mn)
    }

    #[tracing::instrument(level = "info", skip(self))]
    pub async fn stop(&self) -> common_exception::Result<i32> {
        // TODO(xp): need to be reentrant.

        let mut rx = self.raft.metrics();

        self.raft
            .shutdown()
            .await
            .map_err_to_code(ErrorCode::MetaServiceError, || "fail to stop raft")?;
        self.running_tx.send(()).unwrap();

        // wait for raft to close the metrics tx
        loop {
            let r = rx.changed().await;
            if r.is_err() {
                break;
            }
            tracing::info!("waiting for raft to shutdown, metrics: {:?}", rx.borrow());
        }
        tracing::info!("shutdown raft");

        // raft counts 1
        let mut joined = 1;
        for j in self.join_handles.lock().await.iter_mut() {
            let _rst = j
                .await
                .map_err_to_code(ErrorCode::MetaServiceError, || "fail to join")?;
            joined += 1;
        }

        tracing::info!("shutdown: id={}", self.sto.id);
        Ok(joined)
    }

    // spawn a monitor to watch raft state changes such as leader changes,
    // and manually add non-voter to cluster so that non-voter receives raft logs.
    pub async fn subscribe_metrics(mn: Arc<Self>, mut metrics_rx: watch::Receiver<RaftMetrics>) {
        //TODO: return a handle for join
        // TODO: every state change triggers add_non_voter!!!
        let mut running_rx = mn.running_rx.clone();
        let mut jh = mn.join_handles.lock().await;

        // TODO: reduce dependency: it does not need all of the fields in MetaNode
        let mn = mn.clone();

        let span = tracing::span!(tracing::Level::INFO, "watch-metrics");

        let h = tokio::task::spawn(
            {
                async move {
                    loop {
                        let changed = tokio::select! {
                            _ = running_rx.changed() => {
                               return Ok::<(), common_exception::ErrorCode>(());
                            }
                            changed = metrics_rx.changed() => {
                                changed
                            }
                        };
                        if changed.is_ok() {
                            let mm = metrics_rx.borrow().clone();
                            if let Some(cur) = mm.current_leader {
                                if cur == mn.sto.id {
                                    // TODO: check result
                                    let _rst = mn.add_configured_non_voters().await;

                                    if _rst.is_err() {
                                        tracing::info!(
                                            "fail to add non-voter: my id={}, rst:{:?}",
                                            mn.sto.id,
                                            _rst
                                        );
                                    }
                                }
                            }
                        } else {
                            // shutting down
                            break;
                        }
                    }

                    Ok::<(), common_exception::ErrorCode>(())
                }
            }
            .instrument(span),
        );
        jh.push(h);
    }

    /// Start MetaNode in either `boot`, `single`, `join` or `open` mode,
    /// according to config.
    #[tracing::instrument(level = "info", skip(config))]
    pub async fn start(config: &RaftConfig) -> Result<Arc<MetaNode>, ErrorCode> {
        tracing::info!(?config, "start()");
        let mn = Self::do_start(config).await?;
        tracing::info!("Done starting MetaNode: {:?}", config);
        Ok(mn)
    }

    async fn do_start(conf: &RaftConfig) -> Result<Arc<MetaNode>, ErrorCode> {
        // TODO(xp): remove boot mode
        if conf.boot {
            let mn = Self::open_create_boot(conf, None, Some(()), Some(vec![])).await?;
            return Ok(mn);
        }

        if conf.single {
            let mn = MetaNode::open_create_boot(conf, Some(()), Some(()), Some(vec![])).await?;
            return Ok(mn);
        }

        if !conf.join.is_empty() {
            // Bring up a new node, join an cluster
            let mn = MetaNode::open_create_boot(conf, Some(()), Some(()), None).await?;

            let addrs = &conf.join;
            for addr in addrs {
                let mut client = MetaServiceClient::connect(format!("http://{}", addr))
                    .await
                    .map_err(|e| ErrorCode::CannotConnectNode(e.to_string()))?;

                let admin_req = AdminRequest {
                    forward_to_leader: true,
                    req: AdminRequestInner::Join(JoinRequest {
                        node_id: conf.id,
                        address: conf.raft_api_addr(),
                    }),
                };

                let _res = client.forward(admin_req.clone()).await?;
                // TODO: retry
                // break;
            }

            return Ok(mn);
        }

        // open mode
        let mn = MetaNode::open_create_boot(conf, Some(()), None, None).await?;
        Ok(mn)
    }

    /// Boot up the first node to create a cluster.
    /// For every cluster this func should be called exactly once.
    #[tracing::instrument(level = "info", skip(config), fields(config_id=config.config_id.as_str()))]
    pub async fn boot(config: &RaftConfig) -> common_exception::Result<Arc<MetaNode>> {
        // 1. Bring a node up as non voter, start the grpc service for raft communication.
        // 2. Initialize itself as leader, because it is the only one in the new cluster.
        // 3. Add itself to the cluster storage by committing an `add-node` log so that the cluster members(only this node) is persisted.

        let mn = Self::open_create_boot(config, None, Some(()), Some(vec![])).await?;

        Ok(mn)
    }

    // Initialized a single node cluster by:
    // - Initializing raft membership.
    // - Adding current node into the meta data.
    #[tracing::instrument(level = "info", skip(self))]
    pub async fn init_cluster(&self, addr: String) -> common_exception::Result<()> {
        let node_id = self.sto.id;

        let mut cluster_node_ids = BTreeSet::new();
        cluster_node_ids.insert(node_id);

        let rst = self
            .raft
            .initialize(cluster_node_ids)
            .await
            .map_err(|x| ErrorCode::MetaServiceError(format!("{:?}", x)))?;

        tracing::info!("initialized cluster, rst: {:?}", rst);

        self.add_node(node_id, addr).await?;

        Ok(())
    }

    /// When a leader is established, it is the leader's responsibility to setup replication from itself to non-voters, AKA learners.
    /// async-raft does not persist the node set of non-voters, thus we need to do it manually.
    /// This fn should be called once a node found it becomes leader.
    #[tracing::instrument(level = "info", skip(self))]
    pub async fn add_configured_non_voters(&self) -> common_exception::Result<()> {
        // TODO after leader established, add non-voter through apis
        let node_ids = self.sto.list_non_voters().await;
        for i in node_ids.iter() {
            let x = self.raft.add_non_voter(*i).await;

            tracing::info!("add_non_voter result: {:?}", x);
            if x.is_ok() {
                tracing::info!("non-voter is added: {}", i);
            } else {
                tracing::info!("non-voter already exist: {}", i);
            }
        }
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn get_node(&self, node_id: &NodeId) -> common_exception::Result<Option<Node>> {
        // inconsistent get: from local state machine

        let sm = self.sto.state_machine.read().await;
        sm.get_node(node_id)
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn get_nodes(&self) -> common_exception::Result<Vec<Node>> {
        // inconsistent get: from local state machine
        let sm = self.sto.state_machine.read().await;
        sm.get_nodes()
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn get_voters(&self) -> common_exception::Result<Vec<Node>> {
        // inconsistent get: from local state machine
        self.sto.get_voters().await
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn get_non_voters(&self) -> common_exception::Result<Vec<Node>> {
        // inconsistent get: from local state machine
        self.sto.get_non_voters().await
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn handle_admin_req(&self, req: AdminRequest) -> Result<AdminResponse, MetaError> {
        let forward = req.forward_to_leader;

        let l = self.as_leader().await;
        let res = match l {
            Ok(l) => l.handle_admin_req(req.clone()).await,
            Err(e) => Err(MetaError::ForwardToLeader(e)),
        };

        let e = match res {
            Ok(x) => return Ok(x),
            Err(e) => e,
        };

        let e = match e {
            MetaError::ForwardToLeader(e) => e,
            _ => return Err(e),
        };

        if !forward {
            return Err(MetaError::ForwardToLeader(e));
        }

        let leader_id = e.leader.ok_or(MetaError::ForwardToLeader(e))?;

        let mut r2 = req.clone();
        // Avoid infinite forward
        r2.set_forward(false);

        let res: AdminResponse = self.forward(&leader_id, r2).await?;

        Ok(res)
    }

    /// Return a MetaLeader if `self` believes it is the leader.
    ///
    /// Otherwise it returns the leader in a ForwardToLeader error.
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn as_leader(&self) -> Result<MetaLeader<'_>, ForwardToLeader> {
        let curr_leader = self.get_leader().await;
        if curr_leader == self.sto.id {
            return Ok(MetaLeader::new(self));
        }
        Err(ForwardToLeader {
            leader: Some(curr_leader),
        })
    }

    /// Add a new node into this cluster.
    /// The node info is committed with raft, thus it must be called on an initialized node.
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn add_node(
        &self,
        node_id: NodeId,
        addr: String,
    ) -> common_exception::Result<AppliedState> {
        // TODO: use txid?
        let resp = self
            .write(LogEntry {
                txid: None,
                cmd: Cmd::AddNode {
                    node_id,
                    node: Node {
                        name: "".to_string(),
                        address: addr,
                    },
                },
            })
            .await?;
        Ok(resp)
    }

    pub async fn get_state_machine(&self) -> RwLockReadGuard<'_, StateMachine> {
        self.sto.state_machine.read().await
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn lookup_table_id(
        &self,
        db_id: u64,
        name: &str,
    ) -> Result<Option<SeqV<TableLookupValue>>, ErrorCode> {
        // inconsistent get: from local state machine

        let sm = self.sto.state_machine.read().await;
        sm.table_lookup().get(
            &(TableLookupKey {
                database_id: db_id,
                table_name: name.to_string(),
            }),
        )
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn list_tables(&self, req: ListTableReq) -> Result<Vec<Arc<TableInfo>>, ErrorCode> {
        // inconsistent get: from local state machine

        let sm = self.sto.state_machine.read().await;
        sm.list_tables(req).await
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn get_table_by_id(&self, tid: &u64) -> Result<Option<SeqV<TableMeta>>, ErrorCode> {
        // inconsistent get: from local state machine

        let sm = self.sto.state_machine.read().await;
        sm.get_table_meta_by_id(tid)
    }

    /// Submit a write request to the known leader. Returns the response after applying the request.
    #[tracing::instrument(level = "info", skip(self))]
    pub async fn write(&self, req: LogEntry) -> common_exception::Result<AppliedState> {
        let res = self
            .handle_admin_req(AdminRequest {
                forward_to_leader: true,
                req: AdminRequestInner::Write(req.clone()),
            })
            .await?;

        let res: AppliedState = res.try_into().expect("expect AppliedState");

        Ok(res)
    }

    /// Try to get the leader from the latest metrics of the local raft node.
    /// If leader is absent, wait for an metrics update in which a leader is set.
    #[tracing::instrument(level = "info", skip(self))]
    pub async fn get_leader(&self) -> NodeId {
        // fast path: there is a known leader

        if let Some(l) = self.metrics_rx.borrow().current_leader {
            return l;
        }

        // slow path: wait loop

        // Need to clone before calling changed() on it.
        // Otherwise other thread waiting on changed() may not receive the change event.
        let mut rx = self.metrics_rx.clone();

        loop {
            // NOTE:
            // The metrics may have already changed before we cloning it.
            // Thus we need to re-check the cloned rx.
            if let Some(l) = rx.borrow().current_leader {
                return l;
            }

            let changed = rx.changed().await;
            if changed.is_err() {
                tracing::info!("raft metrics tx closed");
                return 0;
            }
        }
    }

    #[tracing::instrument(level = "info", skip(self))]
    pub async fn forward(
        &self,
        node_id: &NodeId,
        req: AdminRequest,
    ) -> Result<AdminResponse, MetaError> {
        let addr = self
            .sto
            .get_node_addr(node_id)
            .await
            .map_err(|e| MetaError::UnknownError(e.to_string()))?;

        let mut client = MetaServiceClient::connect(format!("http://{}", addr))
            .await
            .map_err(|e| ConnectionError::new(e, format!("address: {}", addr)))?;

        let resp = client
            .forward(req)
            .await
            .map_err(|e| MetaError::UnknownError(e.to_string()))?;
        let raft_mes = resp.into_inner();

        let res: Result<AdminResponse, MetaError> = raft_mes.into();
        res
    }
}
