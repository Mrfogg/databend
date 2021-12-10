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
use common_arrow::arrow::bitmap::MutableBitmap;
use common_arrow::arrow::buffer::MutableBuffer;

use crate::prelude::*;

pub fn transform<F>(from: &DFStringArray, estimate_bytes: usize, mut f: F) -> DFStringArray
where F: FnMut(&[u8], &mut [u8]) -> Option<usize> {
    let mut values: MutableBuffer<u8> = MutableBuffer::with_capacity(estimate_bytes);
    let mut offsets: MutableBuffer<i64> = MutableBuffer::with_capacity(from.len() + 1);
    let mut validity = MutableBitmap::with_capacity(from.len());
    offsets.push(0);

    let mut offset: usize = 0;

    unsafe {
        for x in from.into_no_null_iter() {
            let bytes = std::slice::from_raw_parts_mut(
                values.as_mut_ptr().add(offset),
                values.capacity() - offset,
            );
            if let Some(len) = f(x, bytes) {
                offset += len;
                offsets.push(i64::from_isize(offset as isize).unwrap());
                validity.push(true);
            } else {
                offsets.push(offset as i64);
                validity.push(false);
            }
        }
        values.set_len(offset);
        values.shrink_to_fit();
        let validity = combine_validities(from.array.validity(), Some(&validity.into()));
        let array = BinaryArray::<i64>::from_data_unchecked(
            BinaryArray::<i64>::default_data_type(),
            offsets.into(),
            values.into(),
            validity,
        );
        DFStringArray::from_arrow_array(&array)
    }
}

pub fn transform_with_no_null<F>(
    from: &DFStringArray,
    estimate_bytes: usize,
    mut f: F,
) -> DFStringArray
where
    F: FnMut(&[u8], &mut [u8]) -> usize,
{
    let mut values: MutableBuffer<u8> = MutableBuffer::with_capacity(estimate_bytes);
    let mut offsets: MutableBuffer<i64> = MutableBuffer::with_capacity(from.len() + 1);
    offsets.push(0);

    let mut offset: usize = 0;

    unsafe {
        for x in from.into_no_null_iter() {
            let bytes = std::slice::from_raw_parts_mut(
                values.as_mut_ptr().add(offset),
                values.capacity() - offset,
            );
            let len = f(x, bytes);

            offset += len;
            offsets.push(i64::from_isize(offset as isize).unwrap());
        }
        values.set_len(offset);
        values.shrink_to_fit();
        let array = BinaryArray::<i64>::from_data_unchecked(
            BinaryArray::<i64>::default_data_type(),
            offsets.into(),
            values.into(),
            from.array.validity().cloned(),
        );
        DFStringArray::from_arrow_array(&array)
    }
}

pub fn transform_from_primitive_with_no_null<F1, F2, T>(
    from: &DFPrimitiveArray<T>,
    f1: F1,
    mut f2: F2,
) -> DFStringArray
where
    T: DFPrimitiveType,
    F1: Fn(&T) -> usize, // each value may turn to value with max size
    F2: FnMut(&T, &mut [u8]) -> usize,
{
    let mut values: MutableBuffer<u8> = MutableBuffer::new();
    let mut offsets: MutableBuffer<i64> = MutableBuffer::with_capacity(from.len() + 1);
    offsets.push(0);

    let mut offset: usize = 0;

    unsafe {
        for x in from.into_no_null_iter() {
            let max_additional = f1(x);
            values.reserve(max_additional);

            let buffer =
                std::slice::from_raw_parts_mut(values.as_mut_ptr().add(offset), max_additional);
            let len = f2(x, buffer);
            offset += len;
            values.set_len(offset);

            offsets.push(offset as i64);
        }
        values.shrink_to_fit();
        let array = BinaryArray::<i64>::from_data_unchecked(
            BinaryArray::<i64>::default_data_type(),
            offsets.into(),
            values.into(),
            from.array.validity().cloned(),
        );
        DFStringArray::from_arrow_array(&array)
    }
}
