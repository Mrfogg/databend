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

use std::time::Duration;

use common_base::tokio;
use common_meta_api::MetaApi;
use common_meta_flight::MetaFlightClient;
use common_meta_types::GetDatabaseReq;

use crate::start_flight_server;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_flight_client_action_timeout() {
    let srv_addr = start_flight_server();

    let timeout = Duration::from_secs(3);
    let client = MetaFlightClient::with_tls_conf(&srv_addr, "", "", Some(timeout), None)
        .await
        .unwrap();

    let res = client.get_database(GetDatabaseReq::new("xx")).await;
    let actual = res.unwrap_err().message();
    let expect = "status: Cancelled, message: \"Timeout expired\", details: [], metadata: MetadataMap { headers: {} }";
    assert_eq!(actual, expect);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_flight_client_handshake_timeout() {
    let srv_addr = start_flight_server();

    let timeout = Duration::from_secs(1);
    let res = MetaFlightClient::with_tls_conf(&srv_addr, "", "", Some(timeout), None).await;
    let actual = res.unwrap_err().message();
    let expect = "status: Cancelled, message: \"Timeout expired\", details: [], metadata: MetadataMap { headers: {} }";
    assert_eq!(actual, expect);
}
