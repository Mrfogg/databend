//  Copyright 2021 Datafuse Labs.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//

use std::collections::HashSet;
use std::sync::Arc;

use common_exception::Result;
use common_planners::Extras;
use common_planners::Part;
use common_planners::Partitions;
use common_planners::Statistics;

use crate::sessions::QueryContext;
use crate::storages::fuse::meta::BlockMeta;
use crate::storages::fuse::pruning::apply_block_pruning;
use crate::storages::fuse::FuseTable;

impl FuseTable {
    #[inline]
    pub async fn do_read_partitions(
        &self,
        ctx: Arc<QueryContext>,
        push_downs: Option<Extras>,
    ) -> Result<(Statistics, Partitions)> {
        let snapshot = self.table_snapshot(ctx.as_ref()).await?;
        match snapshot {
            Some(snapshot) => {
                let da = ctx.get_data_accessor()?;
                let schema = self.table_info.schema();
                let block_metas = apply_block_pruning(&snapshot, schema, &push_downs, da).await?;
                let (statistics, parts) = Self::to_partitions(&block_metas, push_downs);
                Ok((statistics, parts))
            }
            None => Ok((Statistics::default(), vec![])),
        }
    }

    pub fn to_partitions(
        blocks_metas: &[BlockMeta],
        push_downs: Option<Extras>,
    ) -> (Statistics, Partitions) {
        let proj_cols =
            push_downs.and_then(|extras| extras.projection.map(HashSet::<usize>::from_iter));
        blocks_metas.iter().fold(
            (Statistics::default(), Partitions::default()),
            |(mut stats, mut parts), block_meta| {
                parts.push(Part {
                    name: block_meta.location.location.clone(),
                    version: 0,
                });

                stats.read_rows += block_meta.row_count as usize;

                match &proj_cols {
                    Some(proj) => {
                        stats.read_bytes += block_meta
                            .col_stats
                            .iter()
                            .filter(|(cid, _)| proj.contains(&(**cid as usize)))
                            .map(|(_, col_stats)| col_stats.in_memory_size)
                            .sum::<u64>() as usize
                    }
                    None => stats.read_bytes += block_meta.block_size as usize,
                }

                (stats, parts)
            },
        )
    }
}
