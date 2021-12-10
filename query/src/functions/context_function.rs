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

use common_datavalues::DataValue;
use common_exception::ErrorCode;
use common_exception::Result;
use common_functions::aggregates::AggregateFunctionFactory;
use common_functions::scalars::FunctionFactory;
use common_planners::Expression;

use crate::sessions::QueryContext;

pub struct ContextFunction;

impl ContextFunction {
    // Some function args need from context
    // such as `SELECT database()`, the arg is ctx.get_default_db()
    pub fn build_args_from_ctx(name: &str, ctx: Arc<QueryContext>) -> Result<Vec<Expression>> {
        // Check the function is supported in common functions.
        let function_factory = FunctionFactory::instance();
        let aggregate_function_factory = AggregateFunctionFactory::instance();

        if !function_factory.check(name) && !aggregate_function_factory.check(name) {
            return Result::Err(ErrorCode::UnknownFunction(format!(
                "Unsupported function: {:?}",
                name
            )));
        }

        Ok(match name.to_lowercase().as_str() {
            "database" => vec![Expression::create_literal(DataValue::String(Some(
                ctx.get_current_database().into_bytes(),
            )))],
            "version" => vec![Expression::create_literal(DataValue::String(Some(
                ctx.get_fuse_version().into_bytes(),
            )))],
            "current_user" => vec![Expression::create_literal(DataValue::String(Some(
                ctx.get_current_user()?.name.into_bytes(),
            )))],
            _ => vec![],
        })
    }
}
