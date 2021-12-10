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

use std::fmt;

use common_datavalues::columns::DataColumn;
use common_datavalues::prelude::DataColumnsWithField;
use common_datavalues::DataSchema;
use common_datavalues::DataType;
use common_exception::Result;

use crate::scalars::function_factory::FunctionDescription;
use crate::scalars::function_factory::FunctionFeatures;
use crate::scalars::Function;

#[derive(Clone)]
pub struct CurrentUserFunction {}

impl CurrentUserFunction {
    pub fn try_create(_display_name: &str) -> Result<Box<dyn Function>> {
        Ok(Box::new(CurrentUserFunction {}))
    }

    pub fn desc() -> FunctionDescription {
        FunctionDescription::creator(Box::new(Self::try_create))
            .features(FunctionFeatures::default().context_function())
    }
}

impl Function for CurrentUserFunction {
    fn name(&self) -> &str {
        "CurrentUserFunction"
    }

    fn return_type(&self, _args: &[DataType]) -> Result<DataType> {
        Ok(DataType::String)
    }

    fn nullable(&self, _input_schema: &DataSchema) -> Result<bool> {
        Ok(false)
    }

    fn eval(&self, columns: &DataColumnsWithField, _input_rows: usize) -> Result<DataColumn> {
        Ok(columns[0].column().clone())
    }

    fn num_arguments(&self) -> usize {
        1
    }
}

impl fmt::Display for CurrentUserFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "current_user")
    }
}
