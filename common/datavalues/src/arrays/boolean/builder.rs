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

use common_arrow::arrow::array::*;

use crate::prelude::*;
use crate::utils::get_iter_capacity;

pub struct BooleanArrayBuilder {
    builder: MutableBooleanArray,
}

impl ArrayBuilder<bool, DFBooleanArray> for BooleanArrayBuilder {
    /// Appends a value of type `T` into the builder
    #[inline]
    fn append_value(&mut self, v: bool) {
        self.builder.push(Some(v))
    }

    /// Appends a null slot into the builder
    #[inline]
    fn append_null(&mut self) {
        self.builder.push_null();
    }

    fn finish(&mut self) -> DFBooleanArray {
        let array = self.builder.as_arc();
        DFBooleanArray::from_arrow_array(array.as_ref())
    }
}

impl BooleanArrayBuilder {
    pub fn with_capacity(capacity: usize) -> Self {
        BooleanArrayBuilder {
            builder: MutableBooleanArray::with_capacity(capacity),
        }
    }
}

impl NewDataArray<bool> for DFBooleanArray {
    fn new_from_slice(v: &[bool]) -> Self {
        Self::new_from_iter(v.iter().copied())
    }

    fn new_from_opt_slice(opt_v: &[Option<bool>]) -> Self {
        Self::new_from_opt_iter(opt_v.iter().copied())
    }

    fn new_from_opt_iter(it: impl Iterator<Item = Option<bool>>) -> DFBooleanArray {
        let mut builder = BooleanArrayBuilder::with_capacity(get_iter_capacity(&it));
        it.for_each(|opt| builder.append_option(opt));
        builder.finish()
    }

    /// Create a new DataArray from an iterator.
    fn new_from_iter(it: impl Iterator<Item = bool>) -> DFBooleanArray {
        it.collect()
    }

    fn new_from_iter_validity(
        it: impl Iterator<Item = bool>,
        validity: Option<common_arrow::arrow::bitmap::Bitmap>,
    ) -> Self {
        let mut array: DFBooleanArray = it.collect();
        array.array = array.inner().with_validity(validity);

        array
    }
}
