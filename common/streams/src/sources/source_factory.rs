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

use std::collections::HashMap;
use std::sync::Arc;

use common_dal::DataAccessor;
use common_datavalues::DataSchemaRef;
use common_exception::ErrorCode;
use common_exception::Result;

use crate::CsvSource;
use crate::ParquetSource;
use crate::Source;

pub struct SourceFactory {}

pub struct SourceParams<'a> {
    pub acc: Arc<dyn DataAccessor>,
    pub path: &'a str,
    pub format: &'a str,
    pub schema: DataSchemaRef,
    pub max_block_size: usize,
    pub projection: Vec<usize>,
    pub options: &'a HashMap<String, String>,
}

impl SourceFactory {
    pub fn try_get(params: SourceParams) -> Result<Box<dyn Source>> {
        let format = params.format.to_lowercase();
        match format.as_str() {
            "csv" => {
                let has_header = params
                    .options
                    .get("csv_header")
                    .cloned()
                    .unwrap_or_else(|| "0".to_string());

                let reader = params.acc.get_input_stream(params.path, None)?;
                Ok(Box::new(CsvSource::try_create(
                    reader,
                    params.schema,
                    has_header.eq_ignore_ascii_case("1"),
                    params.max_block_size,
                )?))
            }
            "parquet" => Ok(Box::new(ParquetSource::new(
                params.acc,
                params.path.to_owned(),
                params.schema,
                params.projection,
            ))),
            _ => Err(ErrorCode::InvalidSourceFormat(format)),
        }
    }
}
