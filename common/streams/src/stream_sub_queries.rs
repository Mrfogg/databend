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

use std::pin::Pin;
use std::task::Context;

use common_datablocks::DataBlock;
use common_datavalues::columns::DataColumn;
use common_datavalues::DataSchemaRef;
use common_datavalues::DataValue;
use common_exception::Result;
use futures::task::Poll;
use futures::Stream;
use futures::StreamExt;

use crate::SendableDataBlockStream;

pub struct SubQueriesStream {
    input: SendableDataBlockStream,
    schema: DataSchemaRef,
    sub_queries_columns: Vec<DataValue>,
}

impl SubQueriesStream {
    pub fn create(
        schema: DataSchemaRef,
        input: SendableDataBlockStream,
        sub_queries_columns: Vec<DataValue>,
    ) -> SubQueriesStream {
        SubQueriesStream {
            input,
            schema,
            sub_queries_columns,
        }
    }
}

impl Stream for SubQueriesStream {
    type Item = Result<DataBlock>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.input.poll_next_unpin(cx).map(|x| match x {
            Some(Ok(ref block)) => {
                let mut new_columns = block.columns().to_vec();
                for index in 0..self.sub_queries_columns.len() {
                    let values = self.sub_queries_columns[index].clone();
                    new_columns.push(DataColumn::Constant(values, block.num_rows()));
                }

                Some(Ok(DataBlock::create(self.schema.clone(), new_columns)))
            }
            other => other,
        })
    }
}
