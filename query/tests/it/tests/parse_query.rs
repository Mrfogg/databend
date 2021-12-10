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

use common_exception::Result;
use common_planners::PlanNode;
use databend_query::sessions::QueryContext;
use databend_query::sql::PlanParser;

pub fn parse_query(query: impl ToString, ctx: &Arc<QueryContext>) -> Result<PlanNode> {
    let query = query.to_string();
    futures::executor::block_on(PlanParser::parse(&query, ctx.clone()))
}
