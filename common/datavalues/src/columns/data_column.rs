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

use common_arrow::arrow::array::ArrayRef;
use common_exception::Result;

use crate::prelude::*;
use crate::series::IntoSeries;
use crate::series::Series;
use crate::DataValue;

#[derive(Clone, Debug)]
pub enum DataColumn {
    // Array of values.
    Array(Series),
    // A Single value.
    Constant(DataValue, usize),
}

#[derive(Clone, Debug)]
pub struct DataColumnWithField {
    pub(crate) column: DataColumn,
    pub(crate) field: DataField,
}
impl DataColumnWithField {
    pub fn new(column: DataColumn, field: DataField) -> Self {
        Self { column, field }
    }
    pub fn column(&self) -> &DataColumn {
        &self.column
    }
    pub fn field(&self) -> &DataField {
        &self.field
    }
    pub fn data_type(&self) -> &DataType {
        self.field.data_type()
    }
}

pub type DataColumnsWithField = [DataColumnWithField];

impl DataColumn {
    #[inline]
    pub fn data_type(&self) -> DataType {
        match self {
            DataColumn::Array(array) => array.data_type().clone(),
            DataColumn::Constant(v, _) => v.data_type(),
        }
    }

    #[inline]
    pub fn physical_type(&self) -> PhysicalDataType {
        self.data_type().to_physical_type()
    }

    #[inline]
    pub fn to_array(&self) -> Result<Series> {
        match self {
            DataColumn::Array(array) => Ok(array.clone()),
            DataColumn::Constant(scalar, size) => scalar.to_series_with_size(*size),
        }
    }

    #[inline]
    pub fn to_values(&self) -> Result<Vec<DataValue>> {
        match self {
            DataColumn::Array(array) => array.to_values(),
            DataColumn::Constant(scalar, size) => scalar.to_values(*size),
        }
    }

    #[inline]
    pub fn get_array_ref(&self) -> Result<ArrayRef> {
        match self {
            DataColumn::Array(array) => Ok(array.get_array_ref()),
            DataColumn::Constant(scalar, size) => {
                Ok(scalar.to_series_with_size(*size)?.get_array_ref())
            }
        }
    }

    /// Return the minimal series, if it's constant value, it's size is 1.
    /// This could be useful when Constant <op> Constant
    /// Since our kernel is based on Array <op> Array
    /// 1. Constant -----> minimal Array; 2. Array <op> Array; 3. resize_constant
    #[inline]
    pub fn to_minimal_array(&self) -> Result<Series> {
        match self {
            DataColumn::Array(array) => Ok(array.clone()),
            DataColumn::Constant(scalar, _) => scalar.to_series_with_size(1),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match self {
            DataColumn::Array(array) => array.len(),
            DataColumn::Constant(_, size) => *size,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        match self {
            DataColumn::Array(array) => array.len() == 0,
            DataColumn::Constant(_, size) => *size == 0,
        }
    }

    #[inline]
    pub fn get_array_memory_size(&self) -> usize {
        match self {
            DataColumn::Array(array) => array.get_array_memory_size(),
            DataColumn::Constant(scalar, size) => scalar
                .to_series_with_size(*size)
                .map(|arr| arr.get_array_memory_size())
                .unwrap_or(0),
        }
    }

    #[inline]
    pub fn slice(&self, offset: usize, length: usize) -> DataColumn {
        match self {
            DataColumn::Array(array) => DataColumn::Array(array.slice(offset, length)),
            DataColumn::Constant(scalar, _) => DataColumn::Constant(scalar.clone(), length),
        }
    }

    #[inline]
    pub fn clone_empty(&self) -> DataColumn {
        match self {
            DataColumn::Array(array) => DataColumn::Array(array.slice(0, 0)),
            DataColumn::Constant(scalar, _) => DataColumn::Constant(scalar.clone(), 0),
        }
    }

    #[inline]
    pub fn cast_with_type(&self, data_type: &DataType) -> Result<DataColumn> {
        match self {
            DataColumn::Array(array) => Ok(DataColumn::Array(array.cast_with_type(data_type)?)),
            DataColumn::Constant(scalar, size) => {
                let array = scalar.to_series_with_size(1)?;
                let array = array.cast_with_type(data_type)?;

                let value = array.try_get(0)?;
                Ok(DataColumn::Constant(value, *size))
            }
        }
    }

    #[inline]
    pub fn resize_constant(&self, size: usize) -> Self {
        match self {
            DataColumn::Array(array) if array.len() == 1 => {
                let value = array.try_get(0).unwrap();
                DataColumn::Constant(value, size)
            }
            DataColumn::Constant(scalar, _) => DataColumn::Constant(scalar.clone(), size),
            _ => self.clone(),
        }
    }

    #[inline]
    pub fn try_get(&self, index: usize) -> Result<DataValue> {
        match self {
            DataColumn::Array(array) => Ok(array.try_get(index)?),
            DataColumn::Constant(scalar, _) => Ok(scalar.clone()),
        }
    }

    #[inline]
    pub fn serialize(&self, vec: &mut Vec<Vec<u8>>) -> Result<()> {
        let array = self.to_array()?;
        array.serialize(vec)
    }

    /// # Safety
    /// Note this doesn't do any bound checking, for performance reason.
    #[inline]
    pub unsafe fn scatter_unchecked(
        &self,
        indices: &mut dyn Iterator<Item = u64>,
        scatter_size: usize,
    ) -> Result<Vec<DataColumn>> {
        match self {
            DataColumn::Array(array) => {
                let series = array.scatter_unchecked(indices, scatter_size)?;
                Ok(series.iter().map(|s| s.into()).collect())
            }
            DataColumn::Constant(scalar, _) => {
                let mut vs = vec![0; scatter_size];
                indices.for_each(|d| vs[d as usize] += 1);

                Ok(vs
                    .iter()
                    .map(|v| DataColumn::Constant(scalar.clone(), *v))
                    .collect())
            }
        }
    }
}

impl From<Series> for DataColumn {
    fn from(array: Series) -> Self {
        DataColumn::Array(array)
    }
}
impl From<&Series> for DataColumn {
    fn from(array: &Series) -> Self {
        DataColumn::Array(array.clone())
    }
}

impl<T> From<T> for DataColumn
where T: IntoSeries
{
    fn from(array: T) -> Self {
        DataColumn::Array(array.into_series())
    }
}
