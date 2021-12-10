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

use std::alloc::Layout;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use bytes::BytesMut;
use common_datavalues::prelude::*;
use common_datavalues::DFTryFrom;
use common_exception::ErrorCode;
use common_exception::Result;
use common_io::prelude::*;
use num::traits::AsPrimitive;

use super::AggregateFunctionRef;
use super::StateAddr;
use crate::aggregates::aggregate_function_factory::AggregateFunctionDescription;
use crate::aggregates::aggregator_common::assert_unary_arguments;
use crate::aggregates::AggregateFunction;
use crate::with_match_primitive_type;

struct AggregateSumState<T> {
    pub value: Option<T>,
}

impl<T> AggregateSumState<T>
where
    T: std::ops::Add<Output = T> + Copy + Clone,
    Option<T>: BinarySer + BinaryDe,
{
    #[inline(always)]
    fn add(&mut self, other: T) {
        match &self.value {
            Some(a) => self.value = Some(a.add(other)),
            None => self.value = Some(other),
        }
    }

    fn serialize(&self, writer: &mut BytesMut) -> Result<()> {
        self.value.serialize_to_buf(writer)
    }

    fn deserialize(&mut self, reader: &mut &[u8]) -> Result<()> {
        self.value = Option::<T>::deserialize(reader)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct AggregateSumFunction<T, SumT> {
    display_name: String,
    _arguments: Vec<DataField>,
    t: PhantomData<T>,
    sum_t: PhantomData<SumT>,
}

impl<T, SumT> AggregateFunction for AggregateSumFunction<T, SumT>
where
    T: DFPrimitiveType + AsPrimitive<SumT>,
    SumT: DFPrimitiveType + std::ops::Add<Output = SumT>,
    Option<SumT>: Into<DataValue>,
{
    fn name(&self) -> &str {
        "AggregateSumFunction"
    }

    fn return_type(&self) -> Result<DataType> {
        let value: DataValue = Some(SumT::default()).into();

        Ok(value.data_type())
    }

    fn nullable(&self, _input_schema: &DataSchema) -> Result<bool> {
        Ok(false)
    }

    fn init_state(&self, place: StateAddr) {
        place.write(|| AggregateSumState::<SumT> { value: None });
    }

    fn state_layout(&self) -> Layout {
        Layout::new::<AggregateSumState<SumT>>()
    }

    fn accumulate(&self, place: StateAddr, arrays: &[Series], _input_rows: usize) -> Result<()> {
        let value = arrays[0].sum()?;
        let opt_sum: Result<SumT> = DFTryFrom::try_from(value);

        if let Ok(s) = opt_sum {
            let state = place.get::<AggregateSumState<SumT>>();
            state.add(s);
        }

        Ok(())
    }

    fn accumulate_keys(
        &self,
        places: &[StateAddr],
        offset: usize,
        arrays: &[Series],
        _input_rows: usize,
    ) -> Result<()> {
        let darray: &DFPrimitiveArray<T> = arrays[0].static_cast();
        if darray.null_count() == 0 {
            darray
                .inner()
                .values()
                .as_slice()
                .iter()
                .zip(places.iter())
                .for_each(|(v, place)| {
                    let place = place.next(offset);
                    let state = place.get::<AggregateSumState<SumT>>();
                    state.add(v.as_());
                });
        } else {
            darray
                .into_iter()
                .zip(places.iter())
                .for_each(|(c, place)| {
                    if let Some(v) = c {
                        let place = place.next(offset);
                        let state = place.get::<AggregateSumState<SumT>>();
                        state.add(v.as_());
                    }
                });
        }

        Ok(())
    }

    fn serialize(&self, place: StateAddr, writer: &mut BytesMut) -> Result<()> {
        let state = place.get::<AggregateSumState<SumT>>();
        state.serialize(writer)
    }

    fn deserialize(&self, place: StateAddr, reader: &mut &[u8]) -> Result<()> {
        let state = place.get::<AggregateSumState<SumT>>();
        state.deserialize(reader)
    }

    fn merge(&self, place: StateAddr, rhs: StateAddr) -> Result<()> {
        let rhs = rhs.get::<AggregateSumState<SumT>>();
        if let Some(s) = &rhs.value {
            let state = place.get::<AggregateSumState<SumT>>();
            state.add(*s);
        }
        Ok(())
    }

    fn merge_result(&self, place: StateAddr) -> Result<DataValue> {
        let state = place.get::<AggregateSumState<SumT>>();
        Ok(state.value.into())
    }
}

impl<T, SumT> fmt::Display for AggregateSumFunction<T, SumT> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.display_name)
    }
}

impl<T, SumT> AggregateSumFunction<T, SumT>
where
    T: DFPrimitiveType + AsPrimitive<SumT>,
    SumT: DFPrimitiveType + std::ops::Add<Output = SumT>,
    Option<SumT>: Into<DataValue>,
{
    pub fn try_create(
        display_name: &str,
        arguments: Vec<DataField>,
    ) -> Result<AggregateFunctionRef> {
        Ok(Arc::new(Self {
            display_name: display_name.to_owned(),
            _arguments: arguments,
            t: PhantomData,
            sum_t: PhantomData,
        }))
    }
}

pub fn try_create_aggregate_sum_function(
    display_name: &str,
    _params: Vec<DataValue>,
    arguments: Vec<DataField>,
) -> Result<AggregateFunctionRef> {
    assert_unary_arguments(display_name, arguments.len())?;

    let data_type = arguments[0].data_type();
    with_match_primitive_type!(data_type, |$T| {
        AggregateSumFunction::<$T, <$T as DFPrimitiveType>::LargestType>::try_create(
             display_name,
             arguments,
        )
    },

    // no matching branch
    {
        Err(ErrorCode::BadDataValueType(format!(
            "AggregateSumFunction does not support type '{:?}'",
            data_type
        )))
    })
}

pub fn aggregate_sum_function_desc() -> AggregateFunctionDescription {
    AggregateFunctionDescription::creator(Box::new(try_create_aggregate_sum_function))
}
