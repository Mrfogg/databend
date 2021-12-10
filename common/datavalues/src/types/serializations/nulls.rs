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

use common_exception::Result;

use crate::prelude::DataColumn;
use crate::prelude::DataValue;
use crate::TypeSerializer;

pub struct NullSerializer {}

impl TypeSerializer for NullSerializer {
    fn serialize_value(&self, _value: &DataValue) -> Result<String> {
        Ok("NULL".to_owned())
    }

    fn serialize_column(&self, column: &DataColumn) -> Result<Vec<String>> {
        let result: Vec<String> = vec!["NULL".to_owned(); column.len()];
        Ok(result)
    }
}
