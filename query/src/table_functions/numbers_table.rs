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

use std::any::Any;
use std::mem::size_of;
use std::sync::Arc;

use common_datavalues::DataField;
use common_datavalues::DataSchemaRefExt;
use common_datavalues::DataType;
use common_datavalues::DataValue;
use common_exception::ErrorCode;
use common_exception::Result;
use common_meta_types::TableIdent;
use common_meta_types::TableInfo;
use common_meta_types::TableMeta;
use common_planners::Expression;
use common_planners::Extras;
use common_planners::Partitions;
use common_planners::ReadDataSourcePlan;
use common_planners::Statistics;
use common_streams::SendableDataBlockStream;

use super::numbers_stream::NumbersStream;
use crate::pipelines::transforms::get_sort_descriptions;
use crate::sessions::QueryContext;
use crate::storages::Table;
use crate::table_functions::generate_block_parts;
use crate::table_functions::table_function_factory::TableArgs;
use crate::table_functions::TableFunction;

pub struct NumbersTable {
    table_info: TableInfo,
    total: u64,
}

impl NumbersTable {
    pub fn create(
        database_name: &str,
        table_func_name: &str,
        table_id: u64,
        table_args: TableArgs,
    ) -> Result<Arc<dyn TableFunction>> {
        let mut total = None;
        if let Some(args) = &table_args {
            if args.len() == 1 {
                let arg = &args[0];
                if let Expression::Literal { value, .. } = arg {
                    total = Some(value.as_u64()?);
                }
            }
        }

        let total = total.ok_or_else(|| {
            ErrorCode::BadArguments(format!(
                "Must have exactly one number argument for table function.{}",
                &table_func_name
            ))
        })?;

        let engine = match table_func_name {
            "numbers" => "SystemNumbers",
            "numbers_mt" => "SystemNumbersMt",
            "numbers_local" => "SystemNumbersLocal",
            _ => unreachable!(),
        };

        let table_info = TableInfo {
            ident: TableIdent::new(table_id, 0),
            desc: format!("'{}'.'{}'", database_name, table_func_name),
            name: table_func_name.to_string(),
            meta: TableMeta {
                schema: DataSchemaRefExt::create(vec![DataField::new(
                    "number",
                    DataType::UInt64,
                    false,
                )]),
                engine: engine.to_string(),
                options: Default::default(),
            },
        };

        Ok(Arc::new(NumbersTable { table_info, total }))
    }
}

#[async_trait::async_trait]
impl Table for NumbersTable {
    fn is_local(&self) -> bool {
        self.name() == "numbers_local"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_table_info(&self) -> &TableInfo {
        &self.table_info
    }

    async fn read_partitions(
        &self,
        ctx: Arc<QueryContext>,
        _push_downs: Option<Extras>,
    ) -> Result<(Statistics, Partitions)> {
        let statistics = Statistics::new_exact(
            self.total as usize,
            ((self.total) * size_of::<u64>() as u64) as usize,
        );
        let parts =
            generate_block_parts(0, ctx.get_settings().get_max_threads()? as u64, self.total);

        Ok((statistics, parts))
    }

    fn table_args(&self) -> Option<Vec<Expression>> {
        Some(vec![Expression::create_literal(DataValue::UInt64(Some(
            self.total,
        )))])
    }

    async fn read(
        &self,
        ctx: Arc<QueryContext>,
        plan: &ReadDataSourcePlan,
    ) -> Result<SendableDataBlockStream> {
        // If we have order-by and limit push-downs, try the best to only generate top n rows.
        if let Some(extras) = &plan.push_downs {
            if extras.limit.is_some() {
                let sort_descriptions_result =
                    get_sort_descriptions(&self.table_info.schema(), &extras.order_by);

                // It is allowed to have an error when we can't get sort columns from the expression. For
                // example 'select number from numbers(10) order by number+4 limit 10', the column 'number+4'
                // doesn't exist in the numbers table.
                // For case like that, we ignore the error and don't apply any optimization.
                if sort_descriptions_result.is_err() {
                    return Ok(Box::pin(NumbersStream::try_create(
                        ctx,
                        self.schema(),
                        vec![],
                        None,
                    )?));
                }
                let stream = NumbersStream::try_create(
                    ctx,
                    self.schema(),
                    sort_descriptions_result.unwrap(),
                    extras.limit,
                )?;
                return Ok(Box::pin(stream));
            }
        }

        Ok(Box::pin(NumbersStream::try_create(
            ctx,
            self.schema(),
            vec![],
            None,
        )?))
    }
}

impl TableFunction for NumbersTable {
    fn function_name(&self) -> &str {
        self.name()
    }

    fn as_table<'a>(self: Arc<Self>) -> Arc<dyn Table + 'a>
    where Self: 'a {
        self
    }
}
