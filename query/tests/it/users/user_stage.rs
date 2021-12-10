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

use common_base::tokio;
use common_exception::Result;
use common_meta_types::Credentials;
use common_meta_types::StageParams;
use common_meta_types::UserStageInfo;
use databend_query::configs::Config;
use databend_query::users::UserApiProvider;
use pretty_assertions::assert_eq;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_user_stage() -> Result<()> {
    let mut config = Config::default();
    config.query.tenant_id = "tenant1".to_string();

    let comments = "this is a comment";
    let stage_name1 = "stage1";
    let stage_name2 = "stage2";
    let user_mgr = UserApiProvider::create_global(config).await?;

    // add 1.
    {
        let stage_info = UserStageInfo::new(
            stage_name1,
            comments,
            StageParams::new("test", Credentials::S3 {
                access_key_id: String::from("test"),
                secret_access_key: String::from("test"),
            }),
            None,
        );
        user_mgr.add_stage(stage_info).await?;
    }

    // add 2.
    {
        let stage_info = UserStageInfo::new(
            stage_name2,
            comments,
            StageParams::new("test", Credentials::S3 {
                access_key_id: String::from("test"),
                secret_access_key: String::from("test"),
            }),
            None,
        );
        user_mgr.add_stage(stage_info).await?;
    }

    // get all.
    {
        let stages = user_mgr.get_stages().await?;
        assert_eq!(2, stages.len());
        assert_eq!(stage_name2, stages[1].stage_name);
    }

    // get.
    {
        let stage = user_mgr.get_stage(stage_name1).await?;
        assert_eq!(stage_name1, stage.stage_name);
    }

    // drop.
    {
        user_mgr.drop_stage(stage_name1, false).await?;
        let stages = user_mgr.get_stages().await?;
        assert_eq!(1, stages.len());
    }

    // repeat drop same one not with if exist.
    {
        let res = user_mgr.drop_stage(stage_name1, false).await;
        assert!(res.is_err());
    }

    // repeat drop same one with if exist.
    {
        let res = user_mgr.drop_stage(stage_name1, true).await;
        assert!(res.is_ok());
    }

    Ok(())
}
