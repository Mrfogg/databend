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

use std::env;

use common_exception::Result;
use databend_query::storages::csv::count_lines;
use pretty_assertions::assert_eq;

#[test]
fn test_lines_count() -> Result<()> {
    let file = env::current_dir()?
        .join("../tests/data/sample.csv")
        .display()
        .to_string();

    let lines = count_lines(std::fs::File::open(file.as_str())?)?;
    assert_eq!(6, lines);
    Ok(())
}
