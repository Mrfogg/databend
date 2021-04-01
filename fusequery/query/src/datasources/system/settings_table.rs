// Copyright 2020-2021 The Datafuse Authors.
//
// SPDX-License-Identifier: Apache-2.0.

use std::any::Any;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use common_datablocks::DataBlock;
use common_datavalues::{DataField, DataSchema, DataSchemaRef, DataType, DataValue, StringArray};
use common_planners::{Partition, PlanNode, ReadDataSourcePlan, Statistics};
use common_streams::{DataBlockStream, SendableDataBlockStream};

use crate::datasources::ITable;
use crate::sessions::FuseQueryContextRef;

pub struct SettingsTable {
    schema: DataSchemaRef,
}

impl SettingsTable {
    pub fn create() -> Self {
        SettingsTable {
            schema: Arc::new(DataSchema::new(vec![
                DataField::new("name", DataType::Utf8, false),
                DataField::new("value", DataType::Utf8, false),
                DataField::new("default_value", DataType::Utf8, false),
                DataField::new("description", DataType::Utf8, false),
            ])),
        }
    }
}

#[async_trait]
impl ITable for SettingsTable {
    fn name(&self) -> &str {
        "settings"
    }

    fn engine(&self) -> &str {
        "SystemSettings"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> Result<DataSchemaRef> {
        Ok(self.schema.clone())
    }

    fn read_plan(
        &self,
        _ctx: FuseQueryContextRef,
        _push_down_plan: PlanNode,
    ) -> Result<ReadDataSourcePlan> {
        Ok(ReadDataSourcePlan {
            db: "system".to_string(),
            table: self.name().to_string(),
            schema: self.schema.clone(),
            partitions: vec![Partition {
                name: "".to_string(),
                version: 0,
            }],
            statistics: Statistics::default(),
            description: "(Read from system.settings table)".to_string(),
        })
    }

    async fn read(&self, ctx: FuseQueryContextRef) -> Result<SendableDataBlockStream> {
        let settings = ctx.get_settings()?;

        let mut names: Vec<String> = vec![];
        let mut values: Vec<String> = vec![];
        let mut default_values: Vec<String> = vec![];
        let mut descs: Vec<String> = vec![];
        for setting in settings.iter() {
            if let DataValue::Struct(vals) = setting {
                names.push(format!("{:?}", vals[0]));
                values.push(format!("{:?}", vals[1]));
                default_values.push(format!("{:?}", vals[2]));
                descs.push(format!("{:?}", vals[3]));
            }
        }

        let names: Vec<&str> = names.iter().map(|x| x.as_str()).collect();
        let values: Vec<&str> = values.iter().map(|x| x.as_str()).collect();
        let default_values: Vec<&str> = default_values.iter().map(|x| x.as_str()).collect();
        let descs: Vec<&str> = descs.iter().map(|x| x.as_str()).collect();
        let block = DataBlock::create(
            self.schema.clone(),
            vec![
                Arc::new(StringArray::from(names)),
                Arc::new(StringArray::from(values)),
                Arc::new(StringArray::from(default_values)),
                Arc::new(StringArray::from(descs)),
            ],
        );
        Ok(Box::pin(DataBlockStream::create(
            self.schema.clone(),
            None,
            vec![block],
        )))
    }
}