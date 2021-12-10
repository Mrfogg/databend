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

use std::fmt::Debug;

use common_arrow::arrow::compute::comparison;
use common_arrow::arrow::compute::comparison::Simd8;
use common_exception::ErrorCode;
use common_exception::Result;
use num::Num;
use num::NumCast;

use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operator {
    /// Less than
    Lt,
    /// Less than or equal to
    LtEq,
    /// Greater than
    Gt,
    /// Greater than or equal to
    GtEq,
    /// Equal
    Eq,
    /// Not equal
    Neq,
}

pub trait NumComp: Num + NumCast + PartialOrd {}

impl NumComp for f32 {}
impl NumComp for f64 {}
impl NumComp for i8 {}
impl NumComp for i16 {}
impl NumComp for i32 {}
impl NumComp for i64 {}
impl NumComp for u8 {}
impl NumComp for u16 {}
impl NumComp for u32 {}
impl NumComp for u64 {}

pub trait ArrayCompare<Rhs>: Debug {
    /// Check for equality.
    fn eq(&self, _rhs: Rhs) -> Result<DFBooleanArray> {
        Err(ErrorCode::BadDataValueType(format!(
            "Unsupported compare operation: eq for {:?}",
            self,
        )))
    }

    /// Check for inequality.
    fn neq(&self, _rhs: Rhs) -> Result<DFBooleanArray> {
        Err(ErrorCode::BadDataValueType(format!(
            "Unsupported compare operation: neq for {:?}",
            self,
        )))
    }

    /// Greater than comparison.
    fn gt(&self, _rhs: Rhs) -> Result<DFBooleanArray> {
        Err(ErrorCode::BadDataValueType(format!(
            "Unsupported compare operation: gt for {:?}",
            self,
        )))
    }

    /// Greater than or equal comparison.
    fn gt_eq(&self, _rhs: Rhs) -> Result<DFBooleanArray> {
        Err(ErrorCode::BadDataValueType(format!(
            "Unsupported compare operation: gt_eq for {:?}",
            self,
        )))
    }

    /// Less than comparison.
    fn lt(&self, _rhs: Rhs) -> Result<DFBooleanArray> {
        Err(ErrorCode::BadDataValueType(format!(
            "Unsupported compare operation: lt for {:?}",
            self,
        )))
    }

    /// Less than or equal comparison
    fn lt_eq(&self, _rhs: Rhs) -> Result<DFBooleanArray> {
        Err(ErrorCode::BadDataValueType(format!(
            "Unsupported compare operation: lt_eq for {:?}",
            self,
        )))
    }

    fn like(&self, _rhs: Rhs) -> Result<DFBooleanArray> {
        Err(ErrorCode::BadDataValueType(format!(
            "Unsupported compare operation: like for {:?}",
            self,
        )))
    }

    fn nlike(&self, _rhs: Rhs) -> Result<DFBooleanArray> {
        Err(ErrorCode::BadDataValueType(format!(
            "Unsupported compare operation: nlike for {:?}",
            self,
        )))
    }
}

impl<T> DFPrimitiveArray<T>
where T: DFPrimitiveType + NumComp + Simd8
{
    /// First ensure that the Arrays of lhs and rhs match and then iterates over the Arrays and applies
    /// the comparison operator.
    fn comparison(&self, rhs: &DFPrimitiveArray<T>, op: Operator) -> Result<DFBooleanArray> {
        match op {
            Operator::Eq => Ok(comparison::primitive::eq(&self.array, &rhs.array).into()),
            Operator::Neq => Ok(comparison::primitive::neq(&self.array, &rhs.array).into()),
            Operator::Gt => Ok(comparison::primitive::gt(&self.array, &rhs.array).into()),
            Operator::GtEq => Ok(comparison::primitive::gt_eq(&self.array, &rhs.array).into()),
            Operator::Lt => Ok(comparison::primitive::lt(&self.array, &rhs.array).into()),
            Operator::LtEq => Ok(comparison::primitive::lt_eq(&self.array, &rhs.array).into()),
        }
    }

    fn comparison_scalar(&self, rhs: T, op: Operator) -> Result<DFBooleanArray> {
        match op {
            Operator::Eq => Ok(comparison::primitive::eq_scalar(&self.array, rhs).into()),
            Operator::Neq => Ok(comparison::primitive::neq_scalar(&self.array, rhs).into()),
            Operator::Gt => Ok(comparison::primitive::gt_scalar(&self.array, rhs).into()),
            Operator::GtEq => Ok(comparison::primitive::gt_eq_scalar(&self.array, rhs).into()),
            Operator::Lt => Ok(comparison::primitive::lt_scalar(&self.array, rhs).into()),
            Operator::LtEq => Ok(comparison::primitive::lt_eq_scalar(&self.array, rhs).into()),
        }
    }
}

macro_rules! impl_cmp_common {
    ($self:ident, $rhs:ident, $kop:ident, $neg_func:tt) => {{
        if $self.len() == $rhs.len() {
            $self.comparison($rhs, Operator::$kop)
        } else if $rhs.len() == 1 {
            if let Some(value) = $rhs.get(0) {
                $self.comparison_scalar(value, Operator::$kop)
            } else {
                Ok(DFBooleanArray::full(false, $self.len()))
            }
        } else if $self.len() == 1 {
            $rhs.$neg_func($self)
        } else {
            unreachable!()
        }
    }};
}

impl<T> ArrayCompare<&DFPrimitiveArray<T>> for DFPrimitiveArray<T>
where
    T: DFPrimitiveType,
    T: NumComp + Simd8,
{
    fn eq(&self, rhs: &DFPrimitiveArray<T>) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, Eq, eq}
    }

    fn neq(&self, rhs: &DFPrimitiveArray<T>) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, Neq, neq}
    }

    fn gt(&self, rhs: &DFPrimitiveArray<T>) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, Gt, lt_eq}
    }

    fn gt_eq(&self, rhs: &DFPrimitiveArray<T>) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, GtEq, lt}
    }

    fn lt(&self, rhs: &DFPrimitiveArray<T>) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, Lt, gt_eq}
    }

    fn lt_eq(&self, rhs: &DFPrimitiveArray<T>) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, LtEq, gt}
    }
}

impl DFBooleanArray {
    /// First ensure that the Arrays of lhs and rhs match and then iterates over the Arrays and applies
    /// the comparison operator.
    fn comparison(&self, rhs: &DFBooleanArray, op: Operator) -> Result<DFBooleanArray> {
        match op {
            Operator::Eq => Ok(comparison::boolean::eq(&self.array, &rhs.array).into()),
            Operator::Neq => Ok(comparison::boolean::neq(&self.array, &rhs.array).into()),
            Operator::Gt => Ok(comparison::boolean::gt(&self.array, &rhs.array).into()),
            Operator::GtEq => Ok(comparison::boolean::gt_eq(&self.array, &rhs.array).into()),
            Operator::Lt => Ok(comparison::boolean::lt(&self.array, &rhs.array).into()),
            Operator::LtEq => Ok(comparison::boolean::lt_eq(&self.array, &rhs.array).into()),
        }
    }

    fn comparison_scalar(&self, rhs: bool, op: Operator) -> Result<DFBooleanArray> {
        match op {
            Operator::Eq => Ok(comparison::boolean::eq_scalar(&self.array, rhs).into()),
            Operator::Neq => Ok(comparison::boolean::neq_scalar(&self.array, rhs).into()),
            Operator::Gt => Ok(comparison::boolean::gt_scalar(&self.array, rhs).into()),
            Operator::GtEq => Ok(comparison::boolean::gt_eq_scalar(&self.array, rhs).into()),
            Operator::Lt => Ok(comparison::boolean::lt_scalar(&self.array, rhs).into()),
            Operator::LtEq => Ok(comparison::boolean::lt_eq_scalar(&self.array, rhs).into()),
        }
    }
}

impl ArrayCompare<&DFBooleanArray> for DFBooleanArray {
    fn eq(&self, rhs: &DFBooleanArray) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, Eq, eq}
    }

    fn neq(&self, rhs: &DFBooleanArray) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, Neq, neq}
    }

    fn gt(&self, rhs: &DFBooleanArray) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, Gt, lt_eq}
    }

    fn gt_eq(&self, rhs: &DFBooleanArray) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, GtEq, lt}
    }

    fn lt(&self, rhs: &DFBooleanArray) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, Lt, gt_eq}
    }

    fn lt_eq(&self, rhs: &DFBooleanArray) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, LtEq, gt}
    }
}

impl DFStringArray {
    fn comparison(&self, rhs: &DFStringArray, op: Operator) -> Result<DFBooleanArray> {
        match op {
            Operator::Eq => Ok(comparison::binary::eq(&self.array, &rhs.array).into()),
            Operator::Neq => Ok(comparison::binary::neq(&self.array, &rhs.array).into()),
            Operator::Gt => Ok(comparison::binary::gt(&self.array, &rhs.array).into()),
            Operator::GtEq => Ok(comparison::binary::gt_eq(&self.array, &rhs.array).into()),
            Operator::Lt => Ok(comparison::binary::lt(&self.array, &rhs.array).into()),
            Operator::LtEq => Ok(comparison::binary::lt_eq(&self.array, &rhs.array).into()),
        }
    }

    fn comparison_scalar(&self, rhs: &[u8], op: Operator) -> Result<DFBooleanArray> {
        match op {
            Operator::Eq => Ok(comparison::binary::eq_scalar(&self.array, rhs).into()),
            Operator::Neq => Ok(comparison::binary::neq_scalar(&self.array, rhs).into()),
            Operator::Gt => Ok(comparison::binary::gt_scalar(&self.array, rhs).into()),
            Operator::GtEq => Ok(comparison::binary::gt_eq_scalar(&self.array, rhs).into()),
            Operator::Lt => Ok(comparison::binary::lt_scalar(&self.array, rhs).into()),
            Operator::LtEq => Ok(comparison::binary::lt_eq_scalar(&self.array, rhs).into()),
        }
    }

    fn like(&self, rhs: &DFStringArray) -> Result<DFBooleanArray> {
        self.a_like_binary(rhs, |x| x)
    }

    fn like_scalar(&self, rhs: &[u8]) -> Result<DFBooleanArray> {
        self.a_like_binary_scalar(rhs, |x| x)
    }

    fn nlike(&self, rhs: &DFStringArray) -> Result<DFBooleanArray> {
        self.a_like_binary(rhs, |x| !x)
    }

    fn nlike_scalar(&self, rhs: &[u8]) -> Result<DFBooleanArray> {
        self.a_like_binary_scalar(rhs, |x| !x)
    }
}

macro_rules! impl_like_string {
    ($self:ident, $rhs:ident, $op:ident, $scalar_op:ident) => {{
        // broadcast
        if $rhs.len() == 1 {
            if let Some(value) = $rhs.get(0) {
                $self.$scalar_op(value)
            } else {
                Ok(DFBooleanArray::full(false, $self.len()))
            }
        } else if $self.len() == 1 {
            if let Some(value) = $self.get(0) {
                let it = (0..$rhs.len()).map(|_| value);
                let left = DFStringArray::new_from_iter(it);
                left.$op($rhs)
            } else {
                Ok(DFBooleanArray::full(false, $rhs.len()))
            }
        } else {
            $self.$op($rhs)
        }
    }};
}

impl ArrayCompare<&DFStringArray> for DFStringArray {
    fn eq(&self, rhs: &DFStringArray) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, Eq, eq}
    }

    fn neq(&self, rhs: &DFStringArray) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, Neq, neq}
    }

    fn gt(&self, rhs: &DFStringArray) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, Gt, lt_eq}
    }

    fn gt_eq(&self, rhs: &DFStringArray) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, GtEq, lt}
    }

    fn lt(&self, rhs: &DFStringArray) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, Lt, gt_eq}
    }

    fn lt_eq(&self, rhs: &DFStringArray) -> Result<DFBooleanArray> {
        impl_cmp_common! {self, rhs, LtEq, gt}
    }

    fn like(&self, rhs: &DFStringArray) -> Result<DFBooleanArray> {
        impl_like_string! {self, rhs, like, like_scalar}
    }

    fn nlike(&self, rhs: &DFStringArray) -> Result<DFBooleanArray> {
        impl_like_string! {self, rhs, nlike, nlike_scalar}
    }
}

impl ArrayCompare<&DFNullArray> for DFNullArray {}

impl ArrayCompare<&DFStructArray> for DFStructArray {}

macro_rules! impl_cmp_numeric_string_list {
    ($self:ident, $rhs:ident, $cmp_method:ident) => {{
        match ($self.null_count(), $rhs.null_count()) {
            (0, 0) => $self
                .into_no_null_iter()
                .zip($rhs.into_no_null_iter())
                .map(|(left, right)| left.$cmp_method(&right))
                .collect(),
            (0, _) => $self
                .into_no_null_iter()
                .zip($rhs.into_iter())
                .map(|(left, opt_right)| opt_right.map(|right| left.$cmp_method(&right)))
                .collect(),
            (_, 0) => $self
                .into_iter()
                .zip($rhs.into_no_null_iter())
                .map(|(opt_left, right)| opt_left.map(|left| left.$cmp_method(&right)))
                .collect(),
            (_, _) => $self
                .into_iter()
                .zip($rhs.into_iter())
                .map(|(opt_left, opt_right)| match (opt_left, opt_right) {
                    (None, None) => None,
                    (None, Some(_)) => None,
                    (Some(_), None) => None,
                    (Some(left), Some(right)) => Some(left.$cmp_method(&right)),
                })
                .collect_trusted(),
        }
    }};
}

impl ArrayCompare<&DFListArray> for DFListArray {
    fn eq(&self, rhs: &DFListArray) -> Result<DFBooleanArray> {
        Ok(impl_cmp_numeric_string_list!(self, rhs, series_equal))
    }

    fn neq(&self, rhs: &DFListArray) -> Result<DFBooleanArray> {
        self.eq(rhs)?.not()
    }
}
