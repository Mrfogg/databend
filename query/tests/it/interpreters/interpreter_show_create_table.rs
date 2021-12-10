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

use common_base::tokio;
use common_exception::Result;
use common_planners::*;
use databend_query::interpreters::*;
use futures::TryStreamExt;
use pretty_assertions::assert_eq;

use crate::tests::parse_query;

#[tokio::test]
async fn interpreter_show_create_table_test() -> Result<()> {
    let ctx = crate::tests::create_query_context()?;

    // Create table.
    {
        static TEST_CREATE_QUERY: &str = "\
            CREATE TABLE default.a(\
                a bigint, b int, c varchar(255), d smallint, e Date\
            ) Engine = Null COMMENT = 'test create'\
        ";

        if let PlanNode::CreateTable(plan) = parse_query(TEST_CREATE_QUERY, &ctx)? {
            let executor = CreateTableInterpreter::try_create(ctx.clone(), plan.clone())?;
            let _ = executor.execute(None).await?;
        }
    }

    // Show create table.
    {
        if let PlanNode::ShowCreateTable(plan) = parse_query("SHOW CREATE TABLE a", &ctx)? {
            let executor = ShowCreateTableInterpreter::try_create(ctx.clone(), plan.clone())?;
            assert_eq!(executor.name(), "ShowCreateTableInterpreter");
            let stream = executor.execute(None).await?;
            let result = stream.try_collect::<Vec<_>>().await?;
            let expected = vec![
                "+-------+-------------------------------------+",
                "| Table | Create Table                        |",
                "+-------+-------------------------------------+",
                "| a     | CREATE TABLE `a` (                  |",
                "|       |   `a` Int64,                        |",
                "|       |   `b` Int32,                        |",
                "|       |   `c` String,                       |",
                "|       |   `d` Int16,                        |",
                "|       |   `e` Date16,                       |",
                "|       | ) ENGINE=Null COMMENT='test create' |",
                "+-------+-------------------------------------+",
            ];
            common_datablocks::assert_blocks_sorted_eq(expected, result.as_slice());
        } else {
            panic!()
        }
    }

    Ok(())
}
