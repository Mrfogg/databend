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

use colored::Colorize;

use crate::cmds::config::Mode;
use crate::cmds::Config;
pub struct Env {
    pub conf: Config,
    pub prompt: String,
    pub multiline_prompt: String,
}

impl Env {
    pub fn create(conf: Config) -> Self {
        let namespace = conf.group.clone();
        let mode: Mode = conf.mode.clone();
        Env {
            conf,
            prompt: format!("[{}] [{}]> ", namespace.green(), mode),
            multiline_prompt: format!("{} > ", " ".repeat(namespace.len() + 2)),
        }
    }
    pub fn load_mode(&mut self, mode: Mode) {
        self.conf.mode = mode.clone();
        self.prompt = format!("[{}] [{}]> ", self.conf.group.green(), mode);
    }
}
