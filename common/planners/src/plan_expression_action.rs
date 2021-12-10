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

use common_datavalues::DataField;
use common_datavalues::DataType;
use common_datavalues::DataValue;
use common_exception::ErrorCode;
use common_exception::Result;
use common_functions::aggregates::AggregateFunctionFactory;
use common_functions::aggregates::AggregateFunctionRef;
use common_functions::scalars::CastFunction;
use common_functions::scalars::Function;
use common_functions::scalars::FunctionFactory;

#[derive(Debug, Clone)]
pub enum ExpressionAction {
    /// Column which must be in input.
    Input(ActionInput),
    /// Constant column with known value.
    Constant(ActionConstant),
    Alias(ActionAlias),
    Function(ActionFunction),
}

#[derive(Debug, Clone)]
pub struct ActionInput {
    pub name: String,
    pub return_type: DataType,
}

#[derive(Debug, Clone)]
pub struct ActionConstant {
    pub name: String,
    pub value: DataValue,
    pub data_type: DataType,
}

#[derive(Debug, Clone)]
pub struct ActionAlias {
    pub name: String,
    pub arg_name: String,
    pub arg_type: DataType,
}

#[derive(Debug, Clone)]
pub struct ActionFunction {
    pub name: String,
    pub func_name: String,
    pub return_type: DataType,
    pub is_nullable: bool,
    pub is_aggregated: bool,

    // for functions
    pub params: Vec<DataValue>,
    pub arg_names: Vec<String>,
    pub arg_types: Vec<DataType>,

    // only for aggregate functions
    pub arg_fields: Vec<DataField>,
}

impl ExpressionAction {
    pub fn column_name(&self) -> &str {
        match self {
            ExpressionAction::Input(input) => &input.name,
            ExpressionAction::Constant(c) => &c.name,
            ExpressionAction::Alias(a) => &a.name,
            ExpressionAction::Function(f) => &f.name,
        }
    }
}

impl ActionFunction {
    pub fn to_function(&self) -> Result<Box<dyn Function>> {
        if self.is_aggregated {
            return Err(ErrorCode::LogicalError(
                "Action must be non-aggregated function",
            ));
        }

        match self.func_name.as_str() {
            "cast" => CastFunction::create(self.func_name.clone(), self.return_type.clone()),
            _ => FunctionFactory::instance().get(&self.func_name),
        }
    }

    pub fn to_aggregate_function(&self) -> Result<AggregateFunctionRef> {
        if !self.is_aggregated {
            return Err(ErrorCode::LogicalError(
                "Action must be aggregated function",
            ));
        }

        AggregateFunctionFactory::instance().get(
            &self.func_name,
            self.params.clone(),
            self.arg_fields.clone(),
        )
    }
}
