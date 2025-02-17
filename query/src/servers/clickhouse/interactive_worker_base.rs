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

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use common_base::tokio;
use common_base::tokio::sync::mpsc::channel;
use common_base::tokio::time::interval;
use common_base::ProgressValues;
use common_base::TrySpawn;
use common_clickhouse_srv::types::Block as ClickHouseBlock;
use common_clickhouse_srv::CHContext;
use common_datablocks::DataBlock;
use common_datavalues::DataSchemaRef;
use common_exception::Result;
use common_planners::InsertPlan;
use common_planners::PlanNode;
use futures::channel::mpsc;
use futures::channel::mpsc::Receiver;
use futures::SinkExt;
use futures::StreamExt;
use metrics::histogram;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::wrappers::ReceiverStream;

use super::writers::from_clickhouse_block;
use crate::interpreters::InterpreterFactory;
use crate::sessions::QueryContext;
use crate::sessions::SessionRef;
use crate::sql::PlanParser;

pub struct InteractiveWorkerBase;

pub enum BlockItem {
    Block(Result<DataBlock>),
    // for insert prepare, we do not need to send another block again
    InsertSample(DataBlock),
    ProgressTicker(ProgressValues),
}

impl InteractiveWorkerBase {
    pub async fn do_query(
        ch_ctx: &mut CHContext,
        session: SessionRef,
    ) -> Result<Receiver<BlockItem>> {
        let query = &ch_ctx.state.query;
        log::debug!("{}", query);

        let ctx = session.create_context().await?;
        ctx.attach_query_str(query);

        let plan = PlanParser::parse(query, ctx.clone()).await?;

        match plan {
            PlanNode::Insert(insert) => Self::process_insert_query(insert, ch_ctx, ctx).await,
            _ => {
                let start = Instant::now();
                let interpreter = InterpreterFactory::get(ctx.clone(), plan)?;
                let name = interpreter.name().to_string();
                let async_data_stream = interpreter.execute(None);
                let mut data_stream = async_data_stream.await?;
                histogram!(
                    super::clickhouse_metrics::METRIC_INTERPRETER_USEDTIME,
                    start.elapsed(),
                    "interpreter" => name
                );
                let mut interval_stream = IntervalStream::new(interval(Duration::from_millis(30)));
                let cancel = Arc::new(AtomicBool::new(false));

                let (mut tx, rx) = mpsc::channel(20);
                let mut tx2 = tx.clone();
                let cancel_clone = cancel.clone();

                let progress_ctx = ctx.clone();
                tokio::spawn(async move {
                    while !cancel.load(Ordering::Relaxed) {
                        let _ = interval_stream.next().await;
                        let values = progress_ctx.get_and_reset_progress_value();
                        tx.send(BlockItem::ProgressTicker(values)).await.ok();
                    }
                });

                ctx.try_spawn(async move {
                    while let Some(block) = data_stream.next().await {
                        tx2.send(BlockItem::Block(block)).await.ok();
                    }

                    cancel_clone.store(true, Ordering::Relaxed);
                })?;

                Ok(rx)
            }
        }
    }

    pub async fn process_insert_query(
        insert: InsertPlan,
        ch_ctx: &mut CHContext,
        ctx: Arc<QueryContext>,
    ) -> Result<Receiver<BlockItem>> {
        let sample_block = DataBlock::empty_with_schema(insert.schema());
        let (sender, rec) = channel(4);
        ch_ctx.state.out = Some(sender);

        let sc = sample_block.schema().clone();
        let stream = ReceiverStream::new(rec);
        let stream = FromClickHouseBlockStream {
            input: stream,
            schema: sc,
        };

        let interpreter = InterpreterFactory::get(ctx.clone(), PlanNode::Insert(insert))?;
        let name = interpreter.name().to_string();

        let (mut tx, rx) = mpsc::channel(20);
        tx.send(BlockItem::InsertSample(sample_block)).await.ok();

        // the data is comming in async mode
        let sent_all_data = ch_ctx.state.sent_all_data.clone();
        let start = Instant::now();
        ctx.try_spawn(async move {
            interpreter.execute(Some(Box::pin(stream))).await.unwrap();
            sent_all_data.notify_one();
        })?;
        histogram!(
            super::clickhouse_metrics::METRIC_INTERPRETER_USEDTIME,
            start.elapsed(),
            "interpreter" => name
        );
        Ok(rx)
    }
}

pub struct FromClickHouseBlockStream {
    input: ReceiverStream<ClickHouseBlock>,
    schema: DataSchemaRef,
}

impl futures::stream::Stream for FromClickHouseBlockStream {
    type Item = Result<DataBlock>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.input.poll_next_unpin(cx).map(|x| match x {
            Some(v) => Some(from_clickhouse_block(self.schema.clone(), v)),
            _ => None,
        })
    }
}
