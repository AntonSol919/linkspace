// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::sync::LazyLock;

use bytefmt::abe::{parse_abe, ABE};
/// default fmt in many cases and output for `[pkt]`
pub static DEFAULT_PKT: &str = "\
type\\t[type:str]\\n\
hash\\t[hash:str]\\n\
group\\t[/?:[group]:#/b]\\n\
domain\\t[domain:str]\\n\
path\\t[path:str]\\n\
pubkey\\t[/?:[pubkey]:@/b]\\n\
create\\t[create:str]\\n\
links\\t[links_len:str]\\n\
[/links:\\t[tag:str] [ptr:str]\\n]\\n\
data\\t[data_size:str]\\n\
[data:str]\\n\
";
pub static DEFAULT_FMT: LazyLock<Vec<ABE>> = LazyLock::new(|| parse_abe(DEFAULT_PKT).unwrap());
pub static DEFAULT_POINT_FMT: LazyLock<Vec<ABE>> =
    LazyLock::new(|| parse_abe(DEFAULT_PKT).unwrap());
pub static DEFAULT_NETPKT_FMT: LazyLock<Vec<ABE>> =
    LazyLock::new(|| parse_abe(DEFAULT_PKT).unwrap());

pub static PYTHON_REPR_PKT_FMT: LazyLock<Vec<ABE>> =
    LazyLock::new(|| parse_abe(PYTHON_PKT).unwrap());

pub static PYTHON_PKT: &str = "todo - PYTHON_PKT";

pub static JSON_PKT: &str = "todo";
