// Copyright (c) 2021 Quark Container Authors / 2018 The gVisor Authors.
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

use alloc::string::String;
use alloc::vec::Vec;

use super::auth::cap_set::*;
use super::limits::*;

#[derive(Serialize, Deserialize, Default, Debug, Eq, PartialEq, Clone)]
pub struct Process {
    //user
    pub UID: u32,
    pub GID: u32,
    pub AdditionalGids: Vec<u32>,

    pub Terminal: bool,
    pub Args: Vec<String>,
    pub Envs: Vec<String>,
    pub Cwd: String,

    //caps
    pub Caps: TaskCaps,

    pub NoNewPrivileges: bool,

    //host
    pub NumCpu: u32,
    pub HostName: String,

    //Container
    pub limitSet: LimitSetInternal,
    pub ID: String,

    pub Root: String,   // root dir
    pub Stdiofds: [i32; 3],
    pub ExecId: Option<String>,
}
