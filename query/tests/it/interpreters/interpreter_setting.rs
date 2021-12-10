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
use pretty_assertions::assert_eq;

use crate::tests::parse_query;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_setting_interpreter() -> Result<()> {
    let ctx = crate::tests::create_query_context()?;

    if let PlanNode::SetVariable(plan) = parse_query("SET max_block_size=1", &ctx)? {
        let executor = SettingInterpreter::try_create(ctx, plan)?;
        assert_eq!(executor.name(), "SettingInterpreter");

        let mut stream = executor.execute(None).await?;
        while let Some(_block) = stream.next().await {}
    } else {
        panic!()
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_setting_interpreter_error() -> Result<()> {
    let ctx = crate::tests::create_query_context()?;

    if let PlanNode::SetVariable(plan) = parse_query("SET xx = 1", &ctx)? {
        let executor = SettingInterpreter::try_create(ctx, plan)?;
        if let Err(e) = executor.execute(None).await {
            let expect = "Code: 20, displayText = Unknown variable: \"xx\".";
            assert_eq!(expect, format!("{}", e));
        } else {
            panic!();
        }
    }

    Ok(())
}
