// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// The relation between this and the 'RuleType' type is currently very hacky.
use linkspace_pkt::PointTypeFlags;
use std::fmt::Display;

use crate::prelude::RuleType;
pub struct PredInfo {
    pub name: &'static str,
    pub help: &'static str,
    pub example: &'static str,
    pub implies: PointTypeFlags,
}
macro_rules! predty {
    ( enum PredicateType { $( $(#[$outer:meta])* $fname:ident => ($name:expr,$ptype:tt,$example:expr,$help:expr)),*}) => {
        /// A list of all supported query predicates
        #[derive(Debug,Copy,Clone,Eq,PartialEq)]
        #[non_exhaustive]
        pub enum PredicateType{
            $(
                #[doc=stringify!($name)]
                #[doc=" - "]
                #[doc=$help]
                #[doc=concat!(" e.g. " ,$example, " ( implies ",stringify!($ptype),")")]
                $(#[$outer:meta])*
                $fname
            ),*
        }
        impl PredicateType{
            pub const ALL : [PredicateType;24] = [$(PredicateType::$fname),*];
            pub fn try_from_id(id:&[u8]) -> Option<Self> {
                $( if id == $name.as_bytes() { return Some(PredicateType::$fname);})*
                    None
            }
            pub const fn info(self) -> PredInfo{
                match self {
                $(PredicateType::$fname =>
                        PredInfo{
                            implies: PointTypeFlags::$ptype,
                            name:$name,
                            help:$help,
                            example:$example
                        }),*
            }
        }
        }
        impl Display for PredicateType{
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(PredicateType::$fname => f.write_str($name)),*
                }
            }
        }
    };
}

predty!( enum PredicateType {
    Hash => ("hash",DATA,r"\[b:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\]","the point hash"),
    Group => ("group",LINK,r"\[#:pub\]","group id"),
    Domain => ("domain",LINK,r"\[a:example\]","domain - if fewer than 16 bytes, prepadded with \0"),
    Prefix => ("prefix",LINK,r"/hello/world","path prefix - only accepts '=' op"),
    Path => ("path",LINK,r"/hello/world","exact path - only accepts '=' op"),
    Pubkey => ("pubkey",SIGNATURE,r"\[@:me:local\]","public key used to sign point"),
    Create => ("create",LINK,r"\[now:-1H\]","the create stamp"),
    PathLen => ("path_len",LINK,r"\[u8:0\]","the total number of path components - max 8"),
    LinksLen => ("links_len",LINK,r"\[u16:0\]","the number of links in a packet"),
    DataSize => ("data_size",LINK,r"\[u16:0\]","the byte size of the data field"),
    Recv => ("recv",DATA,r"\[now:+1D\]","the recv time of a packet"),
    IBranch => ("i_branch",LINK,r"\[u32:0\]","total packets per uniq (group,domain,path,key) - only applicable during local tree index, ignored otherwise"),
    IDb  => ("i_db",EMPTY,r"\[u32:0\]","total packets read from local instance"),
    INew  => ("i_new",EMPTY,r"\[u32:0\]","total newly received packets"),
    I => ("i",EMPTY,r"\[u32:0\]","total matched packets"),
    Hop => ("hop",EMPTY,r"\[u16:5\]","(mutable) number of hops"),
    Stamp => ("stamp",EMPTY,r"\[now\]","(mutable) variable stamp"),
    Ubits0 => ("ubits0",EMPTY,r"\[u32:0\]","(mutable) user defined bits"),
    Ubits1 => ("ubits1",EMPTY,r"\[u32:0\]","(mutable) user defined bits"),
    Ubits2 => ("ubits2",EMPTY,r"\[u32:0\]","(mutable) user defined bits"),
    Ubits3 => ("ubits3",EMPTY,r"\[u32:0\]","(mutable) user defined bits"),
    Type => ("type",EMPTY,r"\[b2:00000001\]","the field type bits - implied by other predicates"),
    Netflags => ("netflags",EMPTY,r"\[b2:00000000\]","(mutable) netflags"),
    PointSize => ("point_size",DATA,r"\[u16:4\]","exact point size - (netpkt_size - 32b header - 32b hash)")
});
impl From<PredicateType> for RuleType {
    fn from(val: PredicateType) -> Self {
        val.to_string().parse().unwrap()
    }
}

#[test]
fn names() {
    for f in PredicateType::ALL {
        println!("{f}");
        let rt: RuleType = f.into();
        println!("RT {rt}");
        debug_assert_eq!(rt.to_string(), f.info().name, "translation error");
    }
}
