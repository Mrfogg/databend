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

use std::sync::Arc;
use std::time::Instant;

use common_datablocks::DataBlock;
use common_datavalues::prelude::SeriesFrom;
use common_datavalues::series::Series;
use common_exception::Result;
use common_planners::PlanNode;

use crate::sessions::QueryContext;

#[derive(Clone, Copy)]
pub enum LogType {
    Start = 1,
    Finish = 2,
    Error = 3,
}

#[derive(Clone)]
pub struct LogEvent {
    // Type.
    pub log_type: LogType,
    pub handler_type: String,

    // User.
    pub tenant_id: String,
    pub cluster_id: String,
    pub sql_user: String,
    pub sql_user_quota: String,
    pub sql_user_privileges: String,

    // Query.
    pub query_id: String,
    pub query_kind: String,
    pub query_text: String,
    pub query_start_time: u64,
    pub query_end_time: u64,

    // Stats.
    pub written_rows: u64,
    pub written_bytes: u64,
    pub read_rows: u64,
    pub read_bytes: u64,
    pub result_rows: u64,
    pub result_bytes: u64,
    pub cpu_usage: u32,
    pub memory_usage: u64,

    // Client.
    pub client_info: String,
    pub client_address: String,

    // Schema.
    pub current_database: String,
    pub databases: String,
    pub tables: String,
    pub columns: String,
    pub projections: String,

    // Exception.
    pub exception_code: i32,
    pub exception: String,
    pub stack_trace: String,

    // Server.
    pub server_version: String,

    // Extra.
    pub extra: String,
}

pub struct InterpreterQueryLog {
    ctx: Arc<QueryContext>,
    plan: PlanNode,
}

impl InterpreterQueryLog {
    pub fn create(ctx: Arc<QueryContext>, plan: PlanNode) -> Self {
        InterpreterQueryLog { ctx, plan }
    }

    async fn write_log(&self, event: &LogEvent) -> Result<()> {
        let query_log = self.ctx.get_table("system", "query_log").await?;
        let schema = query_log.get_table_info().meta.schema.clone();

        let block = DataBlock::create_by_array(schema.clone(), vec![
            // Type.
            Series::new(vec![event.log_type as u8]),
            Series::new(vec![event.handler_type.as_str()]),
            // User.
            Series::new(vec![event.tenant_id.as_str()]),
            Series::new(vec![event.cluster_id.as_str()]),
            Series::new(vec![event.sql_user.as_str()]),
            Series::new(vec![event.sql_user_privileges.as_str()]),
            Series::new(vec![event.sql_user_quota.as_str()]),
            // Query.
            Series::new(vec![event.query_id.as_str()]),
            Series::new(vec![event.query_kind.as_str()]),
            Series::new(vec![event.query_text.as_str()]),
            Series::new(vec![event.query_start_time as u64]),
            Series::new(vec![event.query_end_time as u64]),
            // Stats.
            Series::new(vec![event.written_rows as u64]),
            Series::new(vec![event.written_bytes as u64]),
            Series::new(vec![event.read_rows as u64]),
            Series::new(vec![event.read_bytes as u64]),
            Series::new(vec![event.result_rows as u64]),
            Series::new(vec![event.result_bytes as u64]),
            Series::new(vec![event.cpu_usage]),
            Series::new(vec![event.memory_usage as u64]),
            // Client.
            Series::new(vec![event.client_info.as_str()]),
            Series::new(vec![event.client_address.as_str()]),
            // Schema.
            Series::new(vec![event.current_database.as_str()]),
            Series::new(vec![event.databases.as_str()]),
            Series::new(vec![event.tables.as_str()]),
            Series::new(vec![event.columns.as_str()]),
            Series::new(vec![event.projections.as_str()]),
            // Exception.
            Series::new(vec![event.exception_code]),
            Series::new(vec![event.exception.as_str()]),
            Series::new(vec![event.stack_trace.as_str()]),
            // Server.
            Series::new(vec![event.server_version.as_str()]),
            // Extra.
            Series::new(vec![event.extra.as_str()]),
        ]);
        let blocks = vec![Ok(block)];
        let input_stream = futures::stream::iter::<Vec<Result<DataBlock>>>(blocks);
        let _ = query_log
            .append_data(self.ctx.clone(), Box::pin(input_stream))
            .await?;

        Ok(())
    }

    pub async fn log_start(&self) -> Result<()> {
        // User.
        let handler_type = self.ctx.get_session().get_type();
        let tenant_id = self.ctx.get_config().query.tenant_id;
        let cluster_id = self.ctx.get_config().query.cluster_id;
        let user = self.ctx.get_current_user()?;
        let sql_user = user.name;
        let sql_user_quota = format!("{:?}", user.quota);
        let sql_user_privileges = format!("{:?}", user.grants);

        // Query.
        let query_id = self.ctx.get_id();
        let query_kind = self.plan.name().to_string();
        let query_text = self.ctx.get_query_str();

        // Stats.
        let query_start_time = Instant::now().elapsed().as_secs();
        let query_end_time = 0;
        let written_rows = 0u64;
        let written_bytes = 0u64;
        let read_rows = 0u64;
        let read_bytes = 0u64;
        let result_rows = 0u64;
        let result_bytes = 0u64;
        let cpu_usage = self.ctx.get_settings().get_max_threads()? as u32;
        let memory_usage = self.ctx.get_session().get_memory_usage() as u64;

        // Client.
        let client_address = format!("{:?}", self.ctx.get_client_address());

        // Schema.
        let current_database = self.ctx.get_current_database();

        let log_event = LogEvent {
            log_type: LogType::Start,
            handler_type,
            tenant_id,
            cluster_id,
            sql_user,
            sql_user_quota,
            sql_user_privileges,
            query_id,
            query_kind,
            query_text,
            query_start_time,
            query_end_time,
            written_rows,
            written_bytes,
            read_rows,
            read_bytes,
            result_rows,
            result_bytes,
            cpu_usage,
            memory_usage,
            client_info: "".to_string(),
            client_address,
            current_database,
            databases: "".to_string(),
            tables: "".to_string(),
            columns: "".to_string(),
            projections: "".to_string(),
            exception_code: 0,
            exception: "".to_string(),
            stack_trace: "".to_string(),
            server_version: "".to_string(),
            extra: "".to_string(),
        };

        self.write_log(&log_event).await
    }

    pub async fn log_finish(&self, result_rows: u64, result_bytes: u64) -> Result<()> {
        // User.
        let handler_type = self.ctx.get_session().get_type();
        let tenant_id = self.ctx.get_config().query.tenant_id;
        let cluster_id = self.ctx.get_config().query.cluster_id;
        let user = self.ctx.get_current_user()?;
        let sql_user = user.name;
        let sql_user_quota = format!("{:?}", user.quota);
        let sql_user_privileges = format!("{:?}", user.grants);

        // Query.
        let query_id = self.ctx.get_id();
        let query_kind = self.plan.name().to_string();
        let query_text = self.ctx.get_query_str();

        // Stats.
        let query_start_time = 0;
        let query_end_time = Instant::now().elapsed().as_secs();
        let written_rows = 0u64;
        let dal_metrics = self.ctx.get_dal_metrics();
        let written_bytes = dal_metrics.write_bytes as u64;
        let read_rows = self.ctx.get_progress_value().read_rows as u64;
        let read_bytes = self.ctx.get_progress_value().read_bytes as u64;
        let cpu_usage = self.ctx.get_settings().get_max_threads()? as u32;
        let memory_usage = self.ctx.get_session().get_memory_usage() as u64;

        // Client.
        let client_address = format!("{:?}", self.ctx.get_client_address());

        // Schema.
        let current_database = self.ctx.get_current_database();

        let log_event = LogEvent {
            log_type: LogType::Finish,
            handler_type,
            tenant_id,
            cluster_id,
            sql_user,
            sql_user_quota,
            sql_user_privileges,
            query_id,
            query_kind,
            query_text,
            query_start_time,
            query_end_time,
            written_rows,
            written_bytes,
            read_rows,
            read_bytes,
            result_rows,
            result_bytes,
            cpu_usage,
            memory_usage,
            client_info: "".to_string(),
            client_address,
            current_database,
            databases: "".to_string(),
            tables: "".to_string(),
            columns: "".to_string(),
            projections: "".to_string(),
            exception_code: 0,
            exception: "".to_string(),
            stack_trace: "".to_string(),
            server_version: "".to_string(),
            extra: "".to_string(),
        };

        self.write_log(&log_event).await
    }
}
