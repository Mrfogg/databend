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
//

use std::sync::Arc;

use async_trait::async_trait;
use common_base::tokio;
use common_exception::ErrorCode;
use common_management::*;
use common_meta_api::KVApi;
use common_meta_types::GetKVActionReply;
use common_meta_types::MGetKVActionReply;
use common_meta_types::MatchSeq;
use common_meta_types::Operation;
use common_meta_types::PrefixListReply;
use common_meta_types::SeqV;
use common_meta_types::UpsertKVAction;
use common_meta_types::UpsertKVActionReply;
use mockall::predicate::*;
use mockall::*;

// and mock!
mock! {
    pub KV {}
    #[async_trait]
    impl KVApi for KV {
        async fn upsert_kv(
            &self,
            act: UpsertKVAction,
        ) -> common_exception::Result<UpsertKVActionReply>;

        async fn get_kv(&self, key: &str) -> common_exception::Result<GetKVActionReply>;

        async fn mget_kv(
            &self,
            key: &[String],
        ) -> common_exception::Result<MGetKVActionReply>;

        async fn prefix_list_kv(&self, prefix: &str) -> common_exception::Result<PrefixListReply>;
        }
}

fn format_user_key(username: &str, hostname: &str) -> String {
    format!("'{}'@'{}'", username, hostname)
}

mod add {
    use common_meta_types::AuthType;
    use common_meta_types::Operation;
    use common_meta_types::UserInfo;

    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_add_user() -> common_exception::Result<()> {
        let test_user_name = "test_user";
        let test_password = "test_password";
        let test_hostname = "localhost";
        let auth_type = AuthType::Sha256;
        let user_info = UserInfo::new(
            test_user_name.to_string(),
            test_hostname.to_string(),
            Vec::from(test_password),
            auth_type.clone(),
        );
        let v = serde_json::to_vec(&user_info)?;
        let value = Operation::Update(serde_json::to_vec(&user_info)?);

        let test_key = format!(
            "__fd_users/tenant1/{}",
            format_user_key(test_user_name, test_hostname)
        );
        let test_seq = MatchSeq::Exact(0);

        // normal
        {
            let test_key = test_key.clone();
            let mut api = MockKV::new();
            api.expect_upsert_kv()
                .with(predicate::eq(UpsertKVAction::new(
                    &test_key,
                    test_seq,
                    value.clone(),
                    None,
                )))
                .times(1)
                .return_once(|_u| Ok(UpsertKVActionReply::new(None, Some(SeqV::new(1, v)))));
            let api = Arc::new(api);
            let user_mgr = UserMgr::new(api, "tenant1");
            let res = user_mgr.add_user(user_info);

            assert!(res.await.is_ok());
        }

        // already exists
        {
            let test_key = test_key.clone();
            let mut api = MockKV::new();
            api.expect_upsert_kv()
                .with(predicate::eq(UpsertKVAction::new(
                    &test_key,
                    test_seq,
                    value.clone(),
                    None,
                )))
                .times(1)
                .returning(|_u| {
                    Ok(UpsertKVActionReply::new(
                        Some(SeqV::new(1, vec![])),
                        Some(SeqV::new(1, vec![])),
                    ))
                });

            let api = Arc::new(api);
            let user_mgr = UserMgr::new(api, "tenant1");

            let user_info = UserInfo::new(
                test_user_name.to_string(),
                test_hostname.to_string(),
                Vec::from(test_password),
                auth_type.clone(),
            );

            let res = user_mgr.add_user(user_info).await;

            assert_eq!(
                res.unwrap_err().code(),
                ErrorCode::UserAlreadyExists("").code()
            );
        }

        // unknown exception
        {
            let mut api = MockKV::new();
            api.expect_upsert_kv()
                .with(predicate::eq(UpsertKVAction::new(
                    &test_key,
                    test_seq,
                    value.clone(),
                    None,
                )))
                .times(1)
                .returning(|_u| Ok(UpsertKVActionReply::new(None, None)));

            let kv = Arc::new(api);

            let user_mgr = UserMgr::new(kv, "tenant1");
            let user_info = UserInfo::new(
                test_user_name.to_string(),
                test_hostname.to_string(),
                Vec::from(test_password),
                auth_type,
            );

            let res = user_mgr.add_user(user_info).await;

            assert_eq!(
                res.unwrap_err().code(),
                ErrorCode::UnknownException("").code()
            );
        }
        Ok(())
    }
}

mod get {
    use common_meta_types::AuthType;
    use common_meta_types::UserInfo;

    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_get_user_seq_match() -> common_exception::Result<()> {
        let test_user_name = "test";
        let test_hostname = "localhost";
        let test_key = format!(
            "__fd_users/tenant1/{}",
            format_user_key(test_user_name, test_hostname)
        );

        let user_info = UserInfo::new(
            test_user_name.to_string(),
            test_hostname.to_string(),
            Vec::from("pass"),
            AuthType::Sha256,
        );
        let value = serde_json::to_vec(&user_info)?;

        let mut kv = MockKV::new();
        kv.expect_get_kv()
            .with(predicate::function(move |v| v == test_key.as_str()))
            .times(1)
            .return_once(move |_k| Ok(Some(SeqV::new(1, value))));

        let kv = Arc::new(kv);
        let user_mgr = UserMgr::new(kv, "tenant1");
        let res = user_mgr.get_user(
            test_user_name.to_string(),
            test_hostname.to_string(),
            Some(1),
        );
        assert!(res.await.is_ok());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_get_user_do_not_care_seq() -> common_exception::Result<()> {
        let test_user_name = "test";
        let test_hostname = "localhost";
        let test_key = format!(
            "__fd_users/tenant1/{}",
            format_user_key(test_user_name, test_hostname)
        );

        let user_info = UserInfo::new(
            test_user_name.to_string(),
            test_hostname.to_string(),
            Vec::from("pass"),
            AuthType::Sha256,
        );
        let value = serde_json::to_vec(&user_info)?;

        let mut kv = MockKV::new();
        kv.expect_get_kv()
            .with(predicate::function(move |v| v == test_key.as_str()))
            .times(1)
            .return_once(move |_k| Ok(Some(SeqV::new(100, value))));

        let kv = Arc::new(kv);
        let user_mgr = UserMgr::new(kv, "tenant1");
        let res = user_mgr.get_user(test_user_name.to_string(), test_hostname.to_string(), None);
        assert!(res.await.is_ok());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_get_user_not_exist() -> common_exception::Result<()> {
        let test_user_name = "test";
        let test_hostname = "localhost";
        let test_key = format!(
            "__fd_users/tenant1/{}",
            format_user_key(test_user_name, test_hostname)
        );

        let mut kv = MockKV::new();
        kv.expect_get_kv()
            .with(predicate::function(move |v| v == test_key.as_str()))
            .times(1)
            .return_once(move |_k| Ok(None));

        let kv = Arc::new(kv);
        let user_mgr = UserMgr::new(kv, "tenant1");
        let res = user_mgr
            .get_user(test_user_name.to_string(), test_hostname.to_string(), None)
            .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), ErrorCode::UnknownUser("").code());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_get_user_not_exist_seq_mismatch() -> common_exception::Result<()> {
        let test_user_name = "test";
        let test_hostname = "localhost";
        let test_key = format!(
            "__fd_users/tenant1/{}",
            format_user_key(test_user_name, test_hostname)
        );

        let mut kv = MockKV::new();
        kv.expect_get_kv()
            .with(predicate::function(move |v| v == test_key.as_str()))
            .times(1)
            .return_once(move |_k| Ok(Some(SeqV::new(1, vec![]))));

        let kv = Arc::new(kv);
        let user_mgr = UserMgr::new(kv, "tenant1");
        let res = user_mgr
            .get_user(
                test_user_name.to_string(),
                test_hostname.to_string(),
                Some(2),
            )
            .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), ErrorCode::UnknownUser("").code());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_get_user_invalid_user_info_encoding() -> common_exception::Result<()> {
        let test_user_name = "test";
        let test_hostname = "localhost";
        let test_key = format!(
            "__fd_users/tenant1/{}",
            format_user_key(test_user_name, test_hostname)
        );

        let mut kv = MockKV::new();
        kv.expect_get_kv()
            .with(predicate::function(move |v| v == test_key.as_str()))
            .times(1)
            .return_once(move |_k| Ok(Some(SeqV::new(1, vec![]))));

        let kv = Arc::new(kv);
        let user_mgr = UserMgr::new(kv, "tenant1");
        let res = user_mgr.get_user(test_user_name.to_string(), test_hostname.to_string(), None);
        assert_eq!(
            res.await.unwrap_err().code(),
            ErrorCode::IllegalUserInfoFormat("").code()
        );

        Ok(())
    }
}

mod get_users {
    use common_meta_types::AuthType;
    use common_meta_types::UserInfo;

    use super::*;

    type FakeKeys = Vec<(String, SeqV<Vec<u8>>)>;
    type UserInfos = Vec<SeqV<UserInfo>>;

    fn prepare() -> common_exception::Result<(FakeKeys, UserInfos)> {
        let mut names = vec![];
        let mut hostnames = vec![];
        let mut keys = vec![];
        let mut res = vec![];
        let mut user_infos = vec![];
        for i in 0..9 {
            let name = format!("test_user_{}", i);
            names.push(name.clone());
            let hostname = format!("test_hostname_{}", i);
            hostnames.push(hostname.clone());

            let key = format!("tenant1/{}", format_user_key(&name, &hostname));
            keys.push(key);
            let user_info = UserInfo::new(name, hostname, Vec::from("pass"), AuthType::Sha256);
            res.push((
                "fake_key".to_string(),
                SeqV::new(i, serde_json::to_vec(&user_info)?),
            ));
            user_infos.push(SeqV::new(i, user_info));
        }
        Ok((res, user_infos))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_get_users_normal() -> common_exception::Result<()> {
        let (res, user_infos) = prepare()?;
        let mut kv = MockKV::new();
        {
            let k = "__fd_users/tenant1";
            kv.expect_prefix_list_kv()
                .with(predicate::eq(k))
                .times(1)
                .return_once(|_p| Ok(res));
        }

        let kv = Arc::new(kv);
        let user_mgr = UserMgr::new(kv, "tenant1");
        let res = user_mgr.get_users();
        assert_eq!(res.await?, user_infos);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_get_all_users_invalid_user_info_encoding() -> common_exception::Result<()> {
        let (mut res, _user_infos) = prepare()?;
        res.insert(
            8,
            (
                "fake_key".to_string(),
                SeqV::new(0, b"some arbitrary str".to_vec()),
            ),
        );

        let mut kv = MockKV::new();
        {
            let k = "__fd_users/tenant1";
            kv.expect_prefix_list_kv()
                .with(predicate::eq(k))
                .times(1)
                .return_once(|_p| Ok(res));
        }

        let kv = Arc::new(kv);
        let user_mgr = UserMgr::new(kv, "tenant1");
        let res = user_mgr.get_users();
        assert_eq!(
            res.await.unwrap_err().code(),
            ErrorCode::IllegalUserInfoFormat("").code()
        );

        Ok(())
    }
}

mod drop {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_drop_user_normal_case() -> common_exception::Result<()> {
        let mut kv = MockKV::new();
        let test_user = "test";
        let test_hostname = "localhost";
        let test_key = format!(
            "__fd_users/tenant1/{}",
            format_user_key(test_user, test_hostname)
        );
        kv.expect_upsert_kv()
            .with(predicate::eq(UpsertKVAction::new(
                &test_key,
                MatchSeq::Any,
                Operation::Delete,
                None,
            )))
            .times(1)
            .returning(|_k| Ok(UpsertKVActionReply::new(Some(SeqV::new(1, vec![])), None)));
        let kv = Arc::new(kv);
        let user_mgr = UserMgr::new(kv, "tenant1");
        let res = user_mgr.drop_user(test_user.to_string(), test_hostname.to_string(), None);
        assert!(res.await.is_ok());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_drop_user_unknown() -> common_exception::Result<()> {
        let mut kv = MockKV::new();
        let test_user = "test";
        let test_hostname = "localhost";
        let test_key = format!(
            "__fd_users/tenant1/{}",
            format_user_key(test_user, test_hostname)
        );
        kv.expect_upsert_kv()
            .with(predicate::eq(UpsertKVAction::new(
                &test_key,
                MatchSeq::Any,
                Operation::Delete,
                None,
            )))
            .times(1)
            .returning(|_k| Ok(UpsertKVActionReply::new(None, None)));
        let kv = Arc::new(kv);
        let user_mgr = UserMgr::new(kv, "tenant1");
        let res = user_mgr.drop_user(test_user.to_string(), test_hostname.to_string(), None);
        assert_eq!(
            res.await.unwrap_err().code(),
            ErrorCode::UnknownUser("").code()
        );
        Ok(())
    }
}

mod update {
    use common_meta_types::AuthType;
    use common_meta_types::UserInfo;

    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_update_user_normal_partial_update() -> common_exception::Result<()> {
        let test_user_name = "name";
        let test_hostname = "localhost";
        let test_key = format!(
            "__fd_users/tenant1/{}",
            format_user_key(test_user_name, test_hostname)
        );
        let test_seq = None;

        let old_pass = "old_key";
        let old_auth_type = AuthType::DoubleSha1;

        let user_info = UserInfo::new(
            test_user_name.to_string(),
            test_hostname.to_string(),
            Vec::from(old_pass),
            old_auth_type,
        );
        let prev_value = serde_json::to_vec(&user_info)?;

        // get_kv should be called
        let mut kv = MockKV::new();
        {
            let test_key = test_key.clone();
            kv.expect_get_kv()
                .with(predicate::function(move |v| v == test_key.as_str()))
                .times(1)
                .return_once(move |_k| Ok(Some(SeqV::new(0, prev_value))));
        }

        // and then, update_kv should be called

        let new_pass = "new pass";
        let new_user_info = UserInfo::new(
            test_user_name.to_string(),
            test_hostname.to_string(),
            Vec::from(new_pass),
            AuthType::DoubleSha1,
        );
        let new_value_with_old_salt = serde_json::to_vec(&new_user_info)?;

        kv.expect_upsert_kv()
            .with(predicate::eq(UpsertKVAction::new(
                &test_key,
                MatchSeq::GE(1),
                Operation::Update(new_value_with_old_salt.clone()),
                None,
            )))
            .times(1)
            .return_once(|_| Ok(UpsertKVActionReply::new(None, Some(SeqV::new(0, vec![])))));

        let kv = Arc::new(kv);
        let user_mgr = UserMgr::new(kv, "tenant1");

        let res = user_mgr.update_user(
            test_user_name.to_string(),
            test_hostname.to_string(),
            Some(new_user_info.password),
            None,
            test_seq,
        );

        assert!(res.await.is_ok());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_update_user_normal_full_update() -> common_exception::Result<()> {
        let test_user_name = "name";
        let test_hostname = "localhost";
        let test_key = format!(
            "__fd_users/tenant1/{}",
            format_user_key(test_user_name, test_hostname)
        );
        let test_seq = None;

        let old_pass = "old_key";
        let old_auth_type = AuthType::DoubleSha1;

        let user_info = UserInfo::new(
            test_user_name.to_string(),
            test_hostname.to_string(),
            Vec::from(old_pass),
            old_auth_type,
        );
        let prev_value = serde_json::to_vec(&user_info)?;

        // - get_kv should be called
        let mut kv = MockKV::new();
        {
            let test_key = test_key.clone();
            kv.expect_get_kv()
                .with(predicate::function(move |v| v == test_key.as_str()))
                .times(1)
                .return_once(move |_k| Ok(Some(SeqV::new(0, prev_value))));
        }
        // - update_kv should be called

        let new_pass = "new_pass";
        let new_auth_type = AuthType::Sha256;

        let new_user_info = UserInfo::new(
            test_user_name.to_string(),
            test_hostname.to_string(),
            Vec::from(new_pass),
            new_auth_type.clone(),
        );
        let new_value = serde_json::to_vec(&new_user_info)?;

        kv.expect_upsert_kv()
            .with(predicate::eq(UpsertKVAction::new(
                &test_key,
                MatchSeq::GE(1),
                Operation::Update(new_value),
                None,
            )))
            .times(1)
            .return_once(|_| Ok(UpsertKVActionReply::new(None, Some(SeqV::new(0, vec![])))));

        let kv = Arc::new(kv);
        let user_mgr = UserMgr::new(kv, "tenant1");

        let res = user_mgr.update_user(
            test_user_name.to_string(),
            test_hostname.to_string(),
            Some(new_user_info.password),
            Some(new_auth_type),
            test_seq,
        );
        assert!(res.await.is_ok());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_update_user_none_update() -> common_exception::Result<()> {
        // mock kv expects nothing
        let test_name = "name";
        let test_hostname = "localhost";
        let kv = MockKV::new();

        let kv = Arc::new(kv);
        let user_mgr = UserMgr::new(kv, "tenant1");

        let new_password: Option<Vec<u8>> = None;
        let res = user_mgr.update_user(
            test_name.to_string(),
            test_hostname.to_string(),
            new_password,
            None,
            None,
        );
        assert!(res.await.is_ok());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_update_user_partial_unknown() -> common_exception::Result<()> {
        let test_user_name = "name";
        let test_hostname = "localhost";
        let test_key = format!(
            "__fd_users/tenant1/{}",
            format_user_key(test_user_name, test_hostname)
        );
        let test_seq = None;

        // if partial update, and get_kv returns None
        // update_kv should NOT be called
        let mut kv = MockKV::new();
        kv.expect_get_kv()
            .with(predicate::function(move |v| v == test_key.as_str()))
            .times(1)
            .return_once(move |_k| Ok(None));

        let kv = Arc::new(kv);
        let user_mgr = UserMgr::new(kv, "tenant1");

        let res = user_mgr.update_user(
            test_user_name.to_string(),
            test_hostname.to_string(),
            Some(Vec::from("new_pass".as_bytes())),
            None,
            test_seq,
        );
        assert_eq!(
            res.await.unwrap_err().code(),
            ErrorCode::UnknownUser("").code()
        );
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_update_user_full_unknown() -> common_exception::Result<()> {
        let test_user_name = "name";
        let test_hostname = "localhost";
        let test_key = format!(
            "__fd_users/tenant1/{}",
            format_user_key(test_user_name, test_hostname)
        );
        let test_seq = None;

        let old_pass = "old_key";
        let old_auth_type = AuthType::DoubleSha1;

        let user_info = UserInfo::new(
            test_user_name.to_string(),
            test_hostname.to_string(),
            Vec::from(old_pass),
            old_auth_type,
        );
        let prev_value = serde_json::to_vec(&user_info)?;

        // - get_kv should be called
        let mut kv = MockKV::new();
        {
            let test_key = test_key.clone();
            kv.expect_get_kv()
                .with(predicate::function(move |v| v == test_key.as_str()))
                .times(1)
                .return_once(move |_k| Ok(Some(SeqV::new(0, prev_value))));
        }

        // upsert should be called
        kv.expect_upsert_kv()
            .with(predicate::function(move |act: &UpsertKVAction| {
                act.key == test_key.as_str() && act.seq == MatchSeq::GE(1)
            }))
            .times(1)
            .returning(|_| Ok(UpsertKVActionReply::new(None, None)));

        let kv = Arc::new(kv);
        let user_mgr = UserMgr::new(kv, "tenant1");

        let res = user_mgr.update_user(
            test_user_name.to_string(),
            test_hostname.to_string(),
            Some(Vec::from("new_pass".as_bytes())),
            Some(AuthType::Sha256),
            test_seq,
        );
        assert_eq!(
            res.await.unwrap_err().code(),
            ErrorCode::UnknownUser("").code()
        );
        Ok(())
    }
}

mod set_user_privileges {
    use common_meta_types::AuthType;
    use common_meta_types::GrantObject;
    use common_meta_types::UserInfo;
    use common_meta_types::UserPrivilege;
    use common_meta_types::UserPrivilegeType;

    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_grant_user_privileges() -> common_exception::Result<()> {
        let test_user_name = "name";
        let test_hostname = "localhost";
        let test_key = format!(
            "__fd_users/tenant1/{}",
            format_user_key(test_user_name, test_hostname)
        );
        let test_seq = None;

        let mut user_info = UserInfo::new(
            test_user_name.to_string(),
            test_hostname.to_string(),
            Vec::from("pass"),
            AuthType::DoubleSha1,
        );
        let prev_value = serde_json::to_vec(&user_info)?;

        // - get_kv should be called
        let mut kv = MockKV::new();
        {
            let test_key = test_key.clone();
            kv.expect_get_kv()
                .with(predicate::function(move |v| v == test_key.as_str()))
                .times(1)
                .return_once(move |_k| Ok(Some(SeqV::new(0, prev_value))));
        }
        // - update_kv should be called
        let mut privileges = UserPrivilege::empty();
        privileges.set_privilege(UserPrivilegeType::Select);
        user_info.grants.grant_privileges(
            test_user_name,
            test_hostname,
            &GrantObject::Global,
            privileges,
        );
        let new_value = serde_json::to_vec(&user_info)?;

        kv.expect_upsert_kv()
            .with(predicate::eq(UpsertKVAction::new(
                &test_key,
                MatchSeq::GE(1),
                Operation::Update(new_value),
                None,
            )))
            .times(1)
            .return_once(|_| Ok(UpsertKVActionReply::new(None, Some(SeqV::new(0, vec![])))));

        let kv = Arc::new(kv);
        let user_mgr = UserMgr::new(kv, "tenant1");

        let res = user_mgr.grant_user_privileges(
            test_user_name.to_string(),
            test_hostname.to_string(),
            GrantObject::Global,
            privileges,
            test_seq,
        );
        assert!(res.await.is_ok());
        Ok(())
    }
}
