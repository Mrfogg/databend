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

use std::io::Read;

use common_exception::Result;
use common_io::prelude::BinaryRead;

use crate::prelude::*;

pub struct StringDeserializer {
    pub builder: StringArrayBuilder,
}

impl TypeDeserializer for StringDeserializer {
    fn de(&mut self, reader: &mut &[u8]) -> Result<()> {
        let offset: u64 = reader.read_uvarint()?;
        let mut values: Vec<u8> = Vec::with_capacity(offset as usize);
        reader.read_exact(&mut values)?;
        self.builder.append_value(reader);
        Ok(())
    }

    fn de_batch(&mut self, reader: &[u8], step: usize, rows: usize) -> Result<()> {
        for row in 0..rows {
            let reader = &reader[step * row..];
            self.builder.append_value(reader);
        }
        Ok(())
    }

    fn de_text(&mut self, reader: &[u8]) -> Result<()> {
        self.builder.append_value(reader);
        Ok(())
    }

    fn de_null(&mut self) {
        self.builder.append_null()
    }

    fn finish_to_series(&mut self) -> Series {
        self.builder.finish().into_series()
    }
}
