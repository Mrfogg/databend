use common_arrow::arrow::bitmap::Bitmap;

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

pub trait ArrayBuilder<N, Array> {
    fn append_value(&mut self, val: N);
    fn append_null(&mut self);
    fn append_option(&mut self, opt_val: Option<N>) {
        match opt_val {
            Some(v) => self.append_value(v),
            None => self.append_null(),
        }
    }
    fn finish(&mut self) -> Array;
}

pub trait NewDataArray<N> {
    fn new_from_slice(v: &[N]) -> Self;
    fn new_from_opt_slice(opt_v: &[Option<N>]) -> Self;

    /// Create a new DataArray from an iterator.
    fn new_from_opt_iter(it: impl Iterator<Item = Option<N>>) -> Self;

    /// Create a new DataArray from an iterator.
    fn new_from_iter(it: impl Iterator<Item = N>) -> Self;
    fn new_from_iter_validity(it: impl Iterator<Item = N>, validity: Option<Bitmap>) -> Self;
}
