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
use std::str;

use common_datavalues::prelude::ArrayApply;
use common_datavalues::prelude::DataColumn;
use common_datavalues::prelude::DataColumnWithField;
use common_datavalues::prelude::DataColumnsWithField;
use common_datavalues::DataSchema;
use common_datavalues::DataType;
use common_exception::ErrorCode;
use common_exception::Result;

use crate::scalars::function_factory::FunctionDescription;
use crate::scalars::function_factory::FunctionFeatures;
use crate::scalars::Function;
use crate::scalars::Monotonicity;

#[derive(Clone)]
pub struct SignFunction {
    display_name: String,
}

impl SignFunction {
    pub fn try_create(display_name: &str) -> Result<Box<dyn Function>> {
        Ok(Box::new(SignFunction {
            display_name: display_name.to_string(),
        }))
    }

    pub fn desc() -> FunctionDescription {
        FunctionDescription::creator(Box::new(Self::try_create))
            .features(FunctionFeatures::default().deterministic().monotonicity())
    }
}

impl Function for SignFunction {
    fn name(&self) -> &str {
        &*self.display_name
    }

    fn num_arguments(&self) -> usize {
        1
    }

    fn return_type(&self, args: &[DataType]) -> Result<DataType> {
        if matches!(
            args[0],
            DataType::UInt8
                | DataType::UInt16
                | DataType::UInt32
                | DataType::UInt64
                | DataType::Int8
                | DataType::Int16
                | DataType::Int32
                | DataType::Int64
                | DataType::Float32
                | DataType::Float64
                | DataType::String
                | DataType::Null
        ) {
            Ok(DataType::Int8)
        } else {
            Err(ErrorCode::IllegalDataType(format!(
                "Expected numeric types, but got {}",
                args[0]
            )))
        }
    }

    fn nullable(&self, _input_schema: &DataSchema) -> Result<bool> {
        Ok(false)
    }

    fn eval(&self, columns: &DataColumnsWithField, _input_rows: usize) -> Result<DataColumn> {
        let result = columns[0]
            .column()
            .to_minimal_array()?
            .cast_with_type(&DataType::Float64)?
            .f64()?
            .apply_cast_numeric(|v| {
                if v > 0_f64 {
                    1_i8
                } else if v < 0_f64 {
                    -1_i8
                } else {
                    0_i8
                }
            });
        let column: DataColumn = result.into();
        Ok(column)
    }

    fn get_monotonicity(&self, args: &[Monotonicity]) -> Result<Monotonicity> {
        let mono = args[0].clone();
        if mono.is_constant {
            return Ok(Monotonicity::create_constant());
        }

        // check whether the left/right boundary is numeric or not.
        let is_boundary_numeric = |boundary: Option<DataColumnWithField>| -> bool {
            if let Some(column_field) = boundary {
                column_field.data_type().is_numeric()
            } else {
                false
            }
        };

        // sign operator is monotonically non-decreasing for numeric values. However,'String' input is an exception.
        // For example, query like "SELECT sign('-1'), sign('+1'), '-1' >= '+1';" returns -1, 1, 1(true),
        // which is not monotonically increasing.
        if is_boundary_numeric(mono.left) || is_boundary_numeric(mono.right) {
            return Ok(Monotonicity::clone_without_range(&args[0]));
        }

        Ok(Monotonicity::default())
    }
}

impl fmt::Display for SignFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.display_name.to_uppercase())
    }
}
