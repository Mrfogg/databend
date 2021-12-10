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

use std::marker::PhantomData;
use std::ops::AddAssign;

use chrono::Duration;
use chrono::NaiveDateTime;
use common_exception::*;

use crate::prelude::*;

pub struct DateTimeSerializer<T: DFPrimitiveType> {
    t: PhantomData<T>,
}

impl<T: DFPrimitiveType> Default for DateTimeSerializer<T> {
    fn default() -> Self {
        Self {
            t: Default::default(),
        }
    }
}

impl<T: DFPrimitiveType> TypeSerializer for DateTimeSerializer<T> {
    fn serialize_value(&self, value: &DataValue) -> Result<String> {
        if value.is_null() {
            return Ok("NULL".to_owned());
        }

        let mut dt = NaiveDateTime::from_timestamp(0, 0);
        let d = Duration::seconds(value.as_i64()?);
        dt.add_assign(d);
        Ok(dt.format("%Y-%m-%d %H:%M:%S").to_string())
    }

    fn serialize_column(&self, column: &DataColumn) -> Result<Vec<String>> {
        let array = column.to_array()?;
        let array: &DFPrimitiveArray<T> = array.static_cast();

        let result: Vec<String> = array
            .iter()
            .map(|x| {
                x.map(|v| {
                    let mut dt = NaiveDateTime::from_timestamp(0, 0);
                    let d = Duration::seconds(v.to_i64().unwrap());
                    dt.add_assign(d);
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_else(|| "NULL".to_owned())
            })
            .collect();
        Ok(result)
    }
}
