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

use common_exception::exception::Result;
use common_meta_types::AuthType;
use common_meta_types::UserInfo;

#[test]
fn test_user_info() -> Result<()> {
    // This test will introduce a older UserInfo struct and a new UserInfo struct.
    // And check the serialize(old_userinfo) can be deserialized by the new UserInfo.
    #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Eq, PartialEq)]
    pub struct OldUserInfo {
        #[serde(default)]
        pub name: String,
        #[serde(default)]
        pub hostname: String,

        #[serde(default)]
        pub password: Vec<u8>,

        #[serde(default)]
        pub auth_type: AuthType,
    }

    let old = OldUserInfo {
        name: "old-name".to_string(),
        hostname: "old-host".to_string(),
        password: Vec::from("pwd"),
        auth_type: AuthType::Sha256,
    };

    let ser_old = serde_json::to_string(&old)?;
    let new = UserInfo::try_from(ser_old.into_bytes())?;

    let expect = UserInfo::new(
        "old-name".to_string(),
        "old-host".to_string(),
        Vec::from("pwd"),
        AuthType::Sha256,
    );
    assert_eq!(new, expect);

    Ok(())
}
