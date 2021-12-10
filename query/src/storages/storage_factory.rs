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

use std::collections::HashMap;
use std::sync::Arc;

use common_exception::ErrorCode;
use common_exception::Result;
use common_infallible::RwLock;
use common_meta_types::TableInfo;

use crate::configs::Config;
use crate::storages::csv::CsvTable;
use crate::storages::fuse::FuseTable;
use crate::storages::github;
use crate::storages::memory::MemoryTable;
use crate::storages::null::NullTable;
use crate::storages::parquet::ParquetTable;
use crate::storages::StorageContext;
use crate::storages::Table;

pub trait StorageCreator: Send + Sync {
    fn try_create(&self, ctx: StorageContext, table_info: TableInfo) -> Result<Box<dyn Table>>;
}

impl<T> StorageCreator for T
where
    T: Fn(StorageContext, TableInfo) -> Result<Box<dyn Table>>,
    T: Send + Sync,
{
    fn try_create(&self, ctx: StorageContext, table_info: TableInfo) -> Result<Box<dyn Table>> {
        self(ctx, table_info)
    }
}

#[derive(Default)]
pub struct StorageFactory {
    creators: RwLock<HashMap<String, Arc<dyn StorageCreator>>>,
}

impl StorageFactory {
    pub fn create(conf: Config) -> Self {
        let mut creators: HashMap<String, Arc<dyn StorageCreator>> = Default::default();

        // Register csv table engine.
        if conf.query.table_engine_csv_enabled {
            creators.insert("CSV".to_string(), Arc::new(CsvTable::try_create));
        }

        // Register memory table engine.
        if conf.query.table_engine_memory_enabled {
            creators.insert("MEMORY".to_string(), Arc::new(MemoryTable::try_create));
        }

        // Register parquet table engine.
        if conf.query.table_engine_parquet_enabled {
            creators.insert("PARQUET".to_string(), Arc::new(ParquetTable::try_create));
        }

        // Register github table engine;
        if conf.query.table_engine_github_enabled {
            creators.insert(
                github::GITHUB_REPO_COMMENTS_TABLE_ENGINE.to_string(),
                Arc::new(github::RepoCommentsTable::try_create),
            );
            creators.insert(
                github::GITHUB_REPO_INFO_TABLE_ENGINE.to_string(),
                Arc::new(github::RepoInfoTable::try_create),
            );
            creators.insert(
                github::GITHUB_REPO_ISSUES_TABLE_ENGINE.to_string(),
                Arc::new(github::RepoIssuesTable::try_create),
            );
            creators.insert(
                github::GITHUB_REPO_PRS_TABLE_ENGINE.to_string(),
                Arc::new(github::RepoPrsTable::try_create),
            );
        }

        // Register NULL table engine.
        creators.insert("NULL".to_string(), Arc::new(NullTable::try_create));

        // Register FUSE table engine.
        creators.insert("FUSE".to_string(), Arc::new(FuseTable::try_create));

        StorageFactory {
            creators: RwLock::new(creators),
        }
    }

    pub fn get_table(&self, ctx: StorageContext, table_info: &TableInfo) -> Result<Arc<dyn Table>> {
        let engine = table_info.engine().to_uppercase();
        let lock = self.creators.read();
        let factory = lock.get(&engine).ok_or_else(|| {
            ErrorCode::UnknownTableEngine(format!("Unknown table engine {}", engine))
        })?;

        let table: Arc<dyn Table> = factory.try_create(ctx, table_info.clone())?.into();
        Ok(table)
    }
}
