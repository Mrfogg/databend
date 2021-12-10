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
use common_arrow::arrow::bitmap::utils::zip_validity;
use common_arrow::arrow::bitmap::utils::ZipValidity;
use common_arrow::arrow::trusted_len::TrustedLen;

use crate::prelude::*;

impl<'a> IntoIterator for &'a DFStringArray {
    type Item = Option<&'a [u8]>;
    type IntoIter = ZipValidity<'a, &'a [u8], StringValueIter<'a, i64>>;
    fn into_iter(self) -> Self::IntoIter {
        zip_validity(
            StringValueIter::new(&self.array),
            self.array.validity().as_ref().map(|x| x.iter()),
        )
    }
}

impl DFStringArray {
    pub fn into_no_null_iter<'a>(&'a self) -> impl TrustedLen<Item = &'a [u8]> + '_ + Send + Sync {
        StringIterNoNull::new(self.inner())
    }
}

/// Iterator over slices of `&[u8]`.
#[derive(Debug, Clone)]
pub struct StringValueIter<'a, O: Offset> {
    array: &'a BinaryArray<O>,
    index: usize,
}

impl<'a, O: Offset> StringValueIter<'a, O> {
    pub fn new(array: &'a BinaryArray<O>) -> Self {
        Self { array, index: 0 }
    }
}

impl<'a, O: Offset> Iterator for StringValueIter<'a, O> {
    type Item = &'a [u8];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.array.len() {
            return None;
        } else {
            self.index += 1;
        }
        Some(unsafe { self.array.value_unchecked(self.index - 1) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            self.array.len() - self.index,
            Some(self.array.len() - self.index),
        )
    }
}

/// all arrays have known size.
impl<'a> ExactSizeIterator for StringIterNoNull<'a> {}
unsafe impl<'a> TrustedLen for StringIterNoNull<'a> {}

pub struct StringIterNoNull<'a> {
    array: &'a LargeBinaryArray,
    current: usize,
    current_end: usize,
}

impl<'a> StringIterNoNull<'a> {
    /// create a new iterator
    pub fn new(array: &'a LargeBinaryArray) -> Self {
        StringIterNoNull {
            array,
            current: 0,
            current_end: array.len(),
        }
    }
}

impl<'a> Iterator for StringIterNoNull<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.current_end {
            None
        } else {
            let old = self.current;
            self.current += 1;
            unsafe { Some(self.array.value_unchecked(old)) }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            self.array.len() - self.current,
            Some(self.array.len() - self.current),
        )
    }
}
