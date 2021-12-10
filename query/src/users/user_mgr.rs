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

use common_exception::ErrorCode;
use common_exception::Result;
use common_meta_types::AuthType;
use common_meta_types::GrantObject;
use common_meta_types::UserInfo;
use common_meta_types::UserPrivilege;
use sha2::Digest;

use crate::users::CertifiedInfo;
use crate::users::User;
use crate::users::UserApiProvider;

impl UserApiProvider {
    // Get one user from by tenant.
    pub async fn get_user(&self, user: &str, hostname: &str) -> Result<UserInfo> {
        match user {
            // TODO(BohuTANG): Mock, need removed.
            "default" | "" | "root" => {
                let user = User::new(user, "%", "", AuthType::None);
                Ok(user.into())
            }
            _ => {
                let client = self.get_user_api_client();
                let get_user = client.get_user(user.to_string(), hostname.to_string(), None);
                Ok(get_user.await?.data)
            }
        }
    }

    // Auth the user and password for different Auth type.
    pub async fn auth_user(&self, user: UserInfo, info: CertifiedInfo) -> Result<bool> {
        match user.auth_type {
            AuthType::None => Ok(true),
            AuthType::PlainText => Ok(user.password == info.user_password),
            // MySQL already did x = sha1(x)
            // so we just check double sha1(x)
            AuthType::DoubleSha1 => {
                let mut m = sha1::Sha1::new();
                m.update(&info.user_password);

                let bs = m.digest().bytes();
                let mut m = sha1::Sha1::new();
                m.update(&bs[..]);

                Ok(user.password == m.digest().bytes().to_vec())
            }
            AuthType::Sha256 => {
                let result = sha2::Sha256::digest(&info.user_password);
                Ok(user.password == result.to_vec())
            }
        }
    }

    // Get the tenant all users list.
    pub async fn get_users(&self) -> Result<Vec<UserInfo>> {
        let client = self.get_user_api_client();
        let get_users = client.get_users();

        let mut res = vec![];
        match get_users.await {
            Err(failure) => Err(failure.add_message_back("(while get users).")),
            Ok(seq_users_info) => {
                for seq_user_info in seq_users_info {
                    res.push(seq_user_info.data);
                }

                Ok(res)
            }
        }
    }

    // Add a new user info.
    pub async fn add_user(&self, user_info: UserInfo) -> Result<u64> {
        let client = self.get_user_api_client();
        let add_user = client.add_user(user_info);
        match add_user.await {
            Ok(res) => Ok(res),
            Err(failure) => Err(failure.add_message_back("(while add user).")),
        }
    }

    pub async fn grant_user_privileges(
        &self,
        username: &str,
        hostname: &str,
        object: GrantObject,
        privileges: UserPrivilege,
    ) -> Result<Option<u64>> {
        let client = self.get_user_api_client();
        client
            .grant_user_privileges(
                username.to_string(),
                hostname.to_string(),
                object,
                privileges,
                None,
            )
            .await
            .map_err(|failure| failure.add_message_back("(while set user privileges)"))
    }

    pub async fn revoke_user_privileges(
        &self,
        username: &str,
        hostname: &str,
        object: GrantObject,
        privileges: UserPrivilege,
    ) -> Result<Option<u64>> {
        let client = self.get_user_api_client();
        client
            .revoke_user_privileges(
                username.to_string(),
                hostname.to_string(),
                object,
                privileges,
                None,
            )
            .await
            .map_err(|failure| failure.add_message_back("(while revoke user privileges)"))
    }

    // Drop a user by name and hostname.
    pub async fn drop_user(&self, username: &str, hostname: &str, if_exist: bool) -> Result<()> {
        let client = self.get_user_api_client();
        let drop_user = client.drop_user(username.to_string(), hostname.to_string(), None);
        match drop_user.await {
            Ok(res) => Ok(res),
            Err(failure) => {
                if if_exist && failure.code() == ErrorCode::UnknownUserCode() {
                    Ok(())
                } else {
                    Err(failure.add_message_back("(while set drop user)"))
                }
            }
        }
    }

    // Update a user by name and hostname.
    pub async fn update_user(
        &self,
        username: &str,
        hostname: &str,
        new_auth_type: Option<AuthType>,
        new_password: Option<Vec<u8>>,
    ) -> Result<Option<u64>> {
        let client = self.get_user_api_client();
        let update_user = client.update_user(
            username.to_string(),
            hostname.to_string(),
            new_password,
            new_auth_type,
            None,
        );
        match update_user.await {
            Ok(res) => Ok(res),
            Err(failure) => Err(failure.add_message_back("(while alter user).")),
        }
    }
}
