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

use std::any::Any;
use std::net::SocketAddr;
use std::sync::Arc;

use common_datablocks::DataBlock;
use common_datavalues::series::Series;
use common_datavalues::series::SeriesFrom;
use common_datavalues::DataField;
use common_datavalues::DataSchemaRefExt;
use common_datavalues::DataType;
use common_exception::Result;
use common_meta_types::TableIdent;
use common_meta_types::TableInfo;
use common_meta_types::TableMeta;
use common_meta_types::UserInfo;
use common_planners::ReadDataSourcePlan;
use common_streams::DataBlockStream;
use common_streams::SendableDataBlockStream;

use crate::sessions::QueryContext;
use crate::storages::Table;

pub struct ProcessesTable {
    table_info: TableInfo,
}

impl ProcessesTable {
    pub fn create(table_id: u64) -> Self {
        let schema = DataSchemaRefExt::create(vec![
            DataField::new("id", DataType::String, false),
            DataField::new("type", DataType::String, false),
            DataField::new("host", DataType::String, true),
            DataField::new("user", DataType::String, true),
            DataField::new("state", DataType::String, false),
            DataField::new("database", DataType::String, false),
            DataField::new("extra_info", DataType::String, true),
            DataField::new("memory_usage", DataType::Int64, true),
        ]);

        let table_info = TableInfo {
            desc: "'system'.'processes'".to_string(),
            name: "processes".to_string(),
            ident: TableIdent::new(table_id, 0),
            meta: TableMeta {
                schema,
                engine: "SystemProcesses".to_string(),

                ..Default::default()
            },
        };
        ProcessesTable { table_info }
    }

    fn process_host(client_address: &Option<SocketAddr>) -> Option<Vec<u8>> {
        client_address.as_ref().map(|s| s.to_string().into_bytes())
    }

    fn process_user_info(user_info: &Option<UserInfo>) -> Option<Vec<u8>> {
        user_info.as_ref().map(|s| s.name.clone().into_bytes())
    }

    fn process_extra_info(session_extra_info: &Option<String>) -> Option<Vec<u8>> {
        session_extra_info.clone().map(|s| s.into_bytes())
    }
}

#[async_trait::async_trait]
impl Table for ProcessesTable {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_table_info(&self) -> &TableInfo {
        &self.table_info
    }

    async fn read(
        &self,
        ctx: Arc<QueryContext>,
        _plan: &ReadDataSourcePlan,
    ) -> Result<SendableDataBlockStream> {
        let sessions_manager = ctx.get_sessions_manager();
        let processes_info = sessions_manager.processes_info();

        let mut processes_id = Vec::with_capacity(processes_info.len());
        let mut processes_type = Vec::with_capacity(processes_info.len());
        let mut processes_host = Vec::with_capacity(processes_info.len());
        let mut processes_user = Vec::with_capacity(processes_info.len());
        let mut processes_state = Vec::with_capacity(processes_info.len());
        let mut processes_database = Vec::with_capacity(processes_info.len());
        let mut processes_extra_info = Vec::with_capacity(processes_info.len());
        let mut processes_memory_usage = Vec::with_capacity(processes_info.len());

        for process_info in &processes_info {
            processes_id.push(process_info.id.clone().into_bytes());
            processes_type.push(process_info.typ.clone().into_bytes());
            processes_state.push(process_info.state.clone().into_bytes());
            processes_database.push(process_info.database.clone().into_bytes());
            processes_host.push(ProcessesTable::process_host(&process_info.client_address));
            processes_user.push(ProcessesTable::process_user_info(&process_info.user));
            processes_extra_info.push(ProcessesTable::process_extra_info(
                &process_info.session_extra_info,
            ));
            processes_memory_usage.push(process_info.memory_usage);
        }

        let schema = self.table_info.schema();
        let block = DataBlock::create_by_array(schema.clone(), vec![
            Series::new(processes_id),
            Series::new(processes_type),
            Series::new(processes_host),
            Series::new(processes_user),
            Series::new(processes_state),
            Series::new(processes_database),
            Series::new(processes_extra_info),
            Series::new(processes_memory_usage),
        ]);

        Ok(Box::pin(DataBlockStream::create(schema, None, vec![block])))
    }
}
