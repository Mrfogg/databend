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
use futures::stream::StreamExt;

use crate::tests::parse_query;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_create_table_interpreter() -> Result<()> {
    let ctx = crate::tests::create_query_context()?;

    static TEST_CREATE_QUERY: &str = "\
        CREATE TABLE default.a(\
            a bigint not null default 3, b int default a + 3, c varchar(255), d smallint, e Date\
        ) Engine = Null\
    ";

    if let PlanNode::CreateTable(plan) = parse_query(TEST_CREATE_QUERY, &ctx)? {
        let interpreter = CreateTableInterpreter::try_create(ctx, plan.clone())?;
        let mut stream = interpreter.execute(None).await?;
        while let Some(_block) = stream.next().await {}

        let schema = plan.schema();

        let field_a = schema.field_with_name("a").unwrap();
        assert_eq!(
            format!("{:?}", field_a),
            r#"DataField { name: "a", data_type: Int64, nullable: false, default_expr: "{\"Literal\":{\"value\":{\"UInt8\":3},\"column_name\":null,\"data_type\":\"UInt8\"}}" }"#
        );

        let field_b = schema.field_with_name("b").unwrap();
        assert_eq!(
            format!("{:?}", field_b),
            r#"DataField { name: "b", data_type: Int32, nullable: true, default_expr: "{\"BinaryExpression\":{\"left\":{\"Column\":\"a\"},\"op\":\"+\",\"right\":{\"Literal\":{\"value\":{\"UInt8\":3},\"column_name\":null,\"data_type\":\"UInt8\"}}}}" }"#
        );
    } else {
        panic!()
    }

    Ok(())
}
