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

use common_exception::Result;
use common_tracing::tracing;
use octocrab::params;

use crate::databases::Database;
use crate::databases::DatabaseContext;
use crate::storages::github::RepoCommentsTable;
use crate::storages::github::RepoInfoTable;
use crate::storages::github::RepoIssuesTable;
use crate::storages::github::RepoPrsTable;
use crate::storages::StorageContext;

#[derive(Clone)]
pub struct GithubDatabase {
    ctx: DatabaseContext,
    db_name: String,
}

impl GithubDatabase {
    pub fn try_create(
        ctx: DatabaseContext,
        db_name: &str,
        _db_engine: &str,
    ) -> Result<Box<dyn Database>> {
        Ok(Box::new(Self {
            ctx,
            db_name: db_name.to_string(),
        }))
    }
}

#[async_trait::async_trait]
impl Database for GithubDatabase {
    fn name(&self) -> &str {
        &self.db_name
    }

    async fn init_database(&self) -> Result<()> {
        // 1. get all repos in this organization
        let repos = octocrab::instance()
            .orgs(self.name())
            .list_repos()
            .repo_type(params::repos::Type::Sources)
            .sort(params::repos::Sort::Pushed)
            .direction(params::Direction::Descending)
            .per_page(100)
            .send()
            .await?;

        let storage_ctx = StorageContext {
            meta: self.ctx.meta.clone(),
            in_memory_data: self.ctx.in_memory_data.clone(),
        };
        // 2. create all tables in need
        let mut iter = repos.items.iter();
        for repo in &mut iter {
            tracing::error!("creating {} related repo", &repo.name);
            // Create default db
            RepoInfoTable::create(
                storage_ctx.clone(),
                self.name().to_string(),
                repo.name.clone(),
            )
            .await?;

            RepoIssuesTable::create(
                storage_ctx.clone(),
                self.name().to_string(),
                repo.name.clone(),
            )
            .await?;

            RepoPrsTable::create(
                storage_ctx.clone(),
                self.name().to_string(),
                repo.name.clone(),
            )
            .await?;

            RepoCommentsTable::create(
                storage_ctx.clone(),
                self.name().to_string(),
                repo.name.clone(),
            )
            .await?;
        }

        Ok(())
    }
}
