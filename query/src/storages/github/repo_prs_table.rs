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
use std::collections::HashMap;
use std::sync::Arc;

use common_datablocks::DataBlock;
use common_datavalues::prelude::*;
use common_exception::Result;
use common_meta_types::CreateTableReq;
use common_meta_types::TableInfo;
use common_meta_types::TableMeta;
use common_planners::ReadDataSourcePlan;
use common_streams::DataBlockStream;
use common_streams::SendableDataBlockStream;
use octocrab::models;
use octocrab::params;

use crate::sessions::QueryContext;
use crate::storages::github::github_client::create_github_client;
use crate::storages::github::github_client::get_own_repo_from_table_info;
use crate::storages::github::GITHUB_REPO_PRS_TABLE_ENGINE;
use crate::storages::github::OWNER;
use crate::storages::github::REPO;
use crate::storages::StorageContext;
use crate::storages::Table;

const NUMBER: &str = "number";
const TITLE: &str = "title";
const STATE: &str = "state";
const USER: &str = "user";
const LABELS: &str = "labels";
const ASSIGNESS: &str = "assigness";
const CREATED_AT: &str = "created_at";
const UPDATED_AT: &str = "updated_at";
const CLOSED_AT: &str = "closed_at";

pub struct RepoPrsTable {
    table_info: TableInfo,
}

impl RepoPrsTable {
    pub fn try_create(_ctx: StorageContext, table_info: TableInfo) -> Result<Box<dyn Table>> {
        Ok(Box::new(RepoPrsTable { table_info }))
    }

    pub async fn create(ctx: StorageContext, owner: String, repo: String) -> Result<()> {
        let mut options = HashMap::new();
        options.insert(OWNER.to_string(), owner.clone());
        options.insert(REPO.to_string(), repo.clone());

        let req = CreateTableReq {
            if_not_exists: false,
            db: owner.clone(),
            table: repo.clone() + "_prs",
            table_meta: TableMeta {
                schema: RepoPrsTable::schema(),
                engine: GITHUB_REPO_PRS_TABLE_ENGINE.into(),
                options,
            },
        };
        ctx.meta.create_table(req).await?;
        Ok(())
    }

    fn schema() -> Arc<DataSchema> {
        let fields = vec![
            DataField::new(NUMBER, DataType::UInt64, false),
            DataField::new(TITLE, DataType::String, true),
            DataField::new(STATE, DataType::String, true),
            DataField::new(USER, DataType::String, true),
            DataField::new(LABELS, DataType::String, true),
            DataField::new(ASSIGNESS, DataType::String, true),
            DataField::new(CREATED_AT, DataType::DateTime32(None), true),
            DataField::new(UPDATED_AT, DataType::DateTime32(None), true),
            DataField::new(CLOSED_AT, DataType::DateTime32(None), true),
        ];

        Arc::new(DataSchema::new(fields))
    }

    async fn get_data_from_github(&self) -> Result<Vec<Series>> {
        // init array
        let mut issue_numer_array: Vec<u64> = Vec::new();
        let mut title_array: Vec<Vec<u8>> = Vec::new();
        // let mut body_array: Vec<Vec<u8>> = Vec::new();
        let mut state_array: Vec<Vec<u8>> = Vec::new();
        let mut user_array: Vec<Vec<u8>> = Vec::new();
        let mut labels_array: Vec<Vec<u8>> = Vec::new();
        let mut assigness_array: Vec<Vec<u8>> = Vec::new();
        let mut created_at_array: Vec<Option<u32>> = Vec::new();
        let mut updated_at_array: Vec<Option<u32>> = Vec::new();
        let mut closed_at_array: Vec<Option<u32>> = Vec::new();

        // get owner repo info from table meta
        let (owner, repo) = get_own_repo_from_table_info(&self.table_info)?;
        let instance = create_github_client()?;

        #[allow(unused_mut)]
        let mut page = instance
            .pulls(owner, repo)
            .list()
            // Optional Parameters
            .state(params::State::All)
            .per_page(100)
            .send()
            .await?;

        let prs = instance
            .all_pages::<models::pulls::PullRequest>(page)
            .await?;
        for pr in prs {
            issue_numer_array.push(pr.number);
            title_array.push(
                pr.title
                    .clone()
                    .unwrap_or_else(|| "".to_string())
                    .as_bytes()
                    .to_vec(),
            );
            state_array.push(
                pr.state
                    .clone()
                    .and_then(|state| match state {
                        models::IssueState::Closed => Some("Closed"),
                        models::IssueState::Open => Some("Open"),
                        _ => None,
                    })
                    .unwrap_or("Unknown")
                    .as_bytes()
                    .to_vec(),
            );
            user_array.push(
                pr.user
                    .clone()
                    .map(|user| user.login.clone())
                    .unwrap_or_else(|| "".to_string())
                    .as_bytes()
                    .to_vec(),
            );
            let mut labels_str = pr
                .labels
                .map(|labels| {
                    let label_str = labels.iter().fold(Vec::new(), |mut content, label| {
                        content.extend_from_slice(label.name.clone().as_bytes());
                        content.push(b',');
                        content
                    });
                    label_str
                })
                .unwrap_or_else(|| vec![b' ']);
            labels_str.pop();
            labels_array.push(labels_str);
            let mut assignees_str = pr
                .assignees
                .map(|assignees| {
                    let assigness_str = assignees.iter().fold(Vec::new(), |mut content, user| {
                        content.extend_from_slice(user.login.clone().as_bytes());
                        content.push(b',');
                        content
                    });
                    assigness_str
                })
                .unwrap_or_else(|| vec![b' ']);
            assignees_str.pop();
            assigness_array.push(assignees_str);
            let created_at = pr
                .created_at
                .map(|created_at| (created_at.timestamp_millis() / 1000) as u32);
            created_at_array.push(created_at);
            let updated_at = pr
                .updated_at
                .map(|updated_at| (updated_at.timestamp_millis() / 1000) as u32);
            updated_at_array.push(updated_at);
            let closed_at = pr
                .closed_at
                .map(|closed_at| (closed_at.timestamp_millis() / 1000) as u32);
            closed_at_array.push(closed_at);
        }

        Ok(vec![
            Series::new(issue_numer_array),
            Series::new(title_array),
            Series::new(state_array),
            Series::new(user_array),
            Series::new(labels_array),
            Series::new(assigness_array),
            Series::new(created_at_array),
            Series::new(updated_at_array),
            Series::new(closed_at_array),
        ])
    }
}

#[async_trait::async_trait]
impl Table for RepoPrsTable {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_table_info(&self) -> &TableInfo {
        &self.table_info
    }

    async fn read(
        &self,
        _ctx: Arc<QueryContext>,
        _plan: &ReadDataSourcePlan,
    ) -> Result<SendableDataBlockStream> {
        let arrays = self.get_data_from_github().await?;
        let block = DataBlock::create_by_array(self.table_info.schema(), arrays);

        Ok(Box::pin(DataBlockStream::create(
            self.table_info.schema(),
            None,
            vec![block],
        )))
    }
}
