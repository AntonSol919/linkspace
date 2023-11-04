// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![deny(missing_docs, missing_debug_implementations)]
#![feature(
    thread_local,
    write_all_vectored,
    control_flow_enum,
    type_alias_impl_trait,
    concat_bytes,
    try_trait_v2,
    strict_provenance
)]
#![doc = include_str!("../README.md")]
#![doc = r#"

The functions re-exported below is essentially the entire linkspace interface.
Bindings in other languages follow the same pattern.

It is designed to be small, low level, compatible across languages, and allow zero copy where possible.
That makes some functions awkward.
E.g. [lk_get_all] only accepts a callback instead of returning a list/vector.
Creating a utility function in your programming language that copies the packets into a list is plenty fast for 99% of users.

[prelude] imports common functions and types with a few utilities for creating them.
"#]

/// The only error format linkspace supports - alias for anyhow::Error
pub type LkError = anyhow::Error;
/// Result for [LkError]
pub type LkResult<T = ()> = std::result::Result<T, LkError>;

/// Re-export common types
pub mod prelude {
    pub use super::*;
    pub use linkspace_common::{
        byte_fmt::{endian_types, AB, B64},
        core::env::RecvPktPtr,
        pkt::{
            ab, as_abtxt_c, now, repr::PktFmt, rspace1, rspace_buf, space_buf, try_ab, Domain,
            Error as PktError, GroupID, Link, LkHash, NetFlags, NetPkt, NetPktArc, NetPktBox,
            NetPktExt, NetPktHeader, NetPktParts, NetPktPtr, Point, PointExt, PointTypeFlags,
            PubKey, RootedSpace, RootedSpaceBuf, RootedStaticSpace, SigningExt, SigningKey, Space,
            SpaceBuf, SpaceError, Stamp, Tag,
        },
        thread_local::{domain, group, set_domain, set_group},
    };
}
use linkspace_common::pkt;
use prelude::*;

pub use prelude::SigningKey;

pub use point::{lk_datapoint, lk_keypoint, lk_linkpoint};
/// creating points
pub mod point {
    use std::{borrow::Cow, io};

    use super::*;
    /**

    create a datapoint with upto MAX_CONTENT_SIZE bytes

    ```
    # use linkspace::{*,prelude::*,abe::*};
    # fn main() -> LkResult{
    let datap = lk_datapoint(b"Some data")?;
    assert_eq!(datap.hash().to_string(), "ATE6XG70_mx-kV2GCZUIkNijcPa8gd1-C3OYu1pXqcU");
    assert_eq!(datap.data() , b"Some data");
    # Ok(())}
    ```

    **/
    pub fn lk_datapoint(data: &[u8]) -> LkResult<NetPktBox> {
        lk_datapoint_ref(data).map(|v| v.as_netbox())
    }
    /// like [lk_datapoint] but keeps it on the stack in rust enum format.
    pub fn lk_datapoint_ref(data: &[u8]) -> LkResult<NetPktParts<'_>> {
        Ok(pkt::try_datapoint_ref(data, pkt::NetOpts::Default)?)
    }
    /**

    create a new linkpoint [NetPktBox]
    ```
    # use linkspace::{*,prelude::{*,endian_types::*},abe::*};
    # fn main() -> LkResult{

    let datap = lk_datapoint(b"this is a datapoint")?;
    let space = rspace_buf(&[b"hello",b"world"]);
    let links = [
        Link{tag: ab(b"a datapoint"),ptr:datap.hash()},
        Link{tag: ab(b"another tag"),ptr:PUBLIC}
    ];
    let data = b"extra data for the linkpoint";
    let create = Some(U64::new(0)); // None == Some(now()).
    let linkpoint = lk_linkpoint(data,ab(b"mydomain"),PUBLIC,&space,&links,create)?;

    assert_eq!(linkpoint.hash().to_string(),"IdnnQjgxJLGxLZGKdaXWVxc82-U8KyJoyKK3sKlD8Lc");
    assert_eq!(linkpoint.data(), data);
    assert_eq!(*linkpoint.get_group(), PUBLIC);

    # Ok(())}

    ```
    **/
    pub fn lk_linkpoint(
        data: &[u8],
        domain: Domain,
        group: GroupID,
        spacename: &RootedSpace,
        links: &[Link],
        create_stamp: Option<Stamp>,
    ) -> LkResult<NetPktBox> {
        lk_linkpoint_ref(data, domain, group, spacename, links, create_stamp).map(|v| v.as_netbox())
    }
    /// like [lk_linkpoint] but keeps it on the stack in rust enum format.
    pub fn lk_linkpoint_ref<'o>(
        data: &'o [u8],
        domain: Domain,
        group: GroupID,
        spacename: &'o RootedSpace,
        links: &'o [Link],
        create_stamp: Option<Stamp>,
    ) -> LkResult<NetPktParts<'o>> {
        Ok(pkt::try_linkpoint_ref(
            group,
            domain,
            spacename,
            links,
            data,
            create_stamp.unwrap_or_else(pkt::now),
            pkt::NetOpts::Default,
        )?)
    }
    /// create a keypoint, i.e. a signed [lk_linkpoint]
    pub fn lk_keypoint(
        signkey: &SigningKey,
        data: &[u8],
        domain: Domain,
        group: GroupID,
        spacename: &RootedSpace,
        links: &[Link],
        create_stamp: Option<Stamp>,
    ) -> LkResult<NetPktBox> {
        lk_keypoint_ref(signkey, data, domain, group, spacename, links, create_stamp)
            .map(|v| v.as_netbox())
    }
    /// like [lk_keypoint] but keeps it on the stack in rust enum format.
    pub fn lk_keypoint_ref<'o>(
        signkey: &SigningKey,
        data: &'o [u8],
        domain: Domain,
        group: GroupID,
        spacename: &'o RootedSpace,
        links: &'o [Link],
        create_stamp: Option<Stamp>,
    ) -> LkResult<NetPktParts<'o>> {
        let create_stamp = create_stamp.unwrap_or_else(now);
        Ok(pkt::try_keypoint_ref(
            group,
            domain,
            spacename,
            links,
            data,
            create_stamp,
            signkey,
            pkt::NetOpts::Default,
        )?)
    }

    /// parse a [NetPktPtr] (the standard binary format) from a buffer
    pub fn lk_read(buf: &[u8], allow_private: bool) -> Result<(Cow<NetPktPtr>, &[u8]), PktError> {
        let pkt = linkspace_common::pkt::read::read_pkt(buf, false)?;
        if !allow_private {
            pkt.check_private()?
        };
        let size: usize = pkt.size().into();
        Ok((pkt, &buf[size..]))
    }
    /// like [lk_read] but skips the hash validation check
    pub fn lk_read_unchecked(
        buf: &[u8],
        allow_private: bool,
    ) -> Result<(Cow<NetPktPtr>, &[u8]), PktError> {
        let pkt = linkspace_common::pkt::read::read_pkt(buf, cfg!(debug_assertions))?;
        if !allow_private {
            pkt.check_private()?
        };
        let size: usize = pkt.size().into();
        Ok((pkt, &buf[size..]))
    }

    /// Writes any impl [NetPkt] into the binary netpkt format
    pub fn lk_write(
        p: &dyn NetPkt,
        allow_private: bool,
        out: &mut dyn io::Write,
    ) -> io::Result<()> {
        if !allow_private {
            p.check_private().map_err(io::Error::other)?
        }
        let mut segments = p.byte_segments().io_slices();
        out.write_all_vectored(&mut segments)
    }
}

pub use abe::{lk_encode, lk_eval, lk_tokenize_abe};
/** ascii byte expression utilities

ABE is a byte templating language.
See guide#ABE to understand its use and indepth explanation.
 **/
pub mod abe {
    use self::scope::UserData;

    use super::*;
    pub use linkspace_common::pkt::repr::DEFAULT_PKT;
    use linkspace_common::{
        abe::abtxt::as_abtxt,
        prelude::{abtxt::CtrChar, ast::tokenize_abe},
    };

    /**
    Evaluate an expression and return the bytes

    Print a list of active scopes with help by using `lk_eval("[help]")`

    Optionally add a `pkt` in the scope.
    See [lk_tokenize_abe] for different delimiter behavior
    See [lk_eval_loose] that is less strict on its input

    ```
    # use linkspace::{*,prelude::*,abe::*};
    # fn main() -> LkResult{
    assert_eq!( b"abc" as &[u8]    , lk_eval( "abc" ,())?, );
    assert_eq!( &[0u8,1,255] as &[u8], lk_eval( r#"\0\x01\xff"# ,())?,);

    // calling functions such as 'u8'
    assert_eq!( b"abc" as &[u8]    , lk_eval( "ab[u8:99]" ,())?, );

    assert_eq!(
               b"The '' function returns its first argument" as &[u8],
       lk_eval( "The '' function returns its first [:argument]", ())?
    );

    assert_eq!(
               b"Bytes are joined" as &[u8],
       lk_eval( "Bytes[: are][: joined]" , ())?
    );

    let result = lk_eval( "Nest expressions [u8:65] == [u8:[:65]] == \x41" , ())?;
    assert_eq!(result,   b"Nest expressions A == A == A");

    let result = lk_eval( "Use result as first argument with '/' [u8:65] == [:65/u8] == \x41" , ())?;
    assert_eq!(result,   b"Use result as first argument with '/' A == A == A");


    let result = lk_eval( "You can provide an argv [0] [1]" , &[b"like" as &[u8], b"this"])?;
    assert_eq!(result,   b"You can provide an argv like this");

    let lp : NetPktBox = lk_linkpoint(&[],ab(b"mydomain"),PUBLIC,RootedSpace::empty(),&[],None)?;
    let pkt: &dyn NetPkt = &lp;

    assert_eq!( lk_eval( "[hash]" , pkt)?,&*pkt.hash());
    let by_arg   = lk_eval( "[hash:str]", pkt)?;
    let by_apply = lk_eval( "[hash/?b]",  pkt)?;
    let as_field = pkt.hash().b64().into_bytes();
    assert_eq!( by_arg, by_apply);
    assert_eq!( by_arg, as_field);

    // or provide both at once with (pkt,&[b"argv"])
    // More options are available in [varscope]

    // escaped characters
    assert_eq!( lk_eval( r#"\n\t\:\/\\\[\]"# ,())?,  &[b'\n',b'\t',b':',b'/',b'\\',b'[',b']'] );

    # Ok(())
    # }
    ```

    **/
    pub fn lk_eval<'o>(expr: &str, udata: impl Into<UserData<'o>>) -> LkResult<Vec<u8>> {
        varscope::lk_eval(scope::scope(udata.into())?, expr, false)
    }

    /**
    Same as lk_eval but accepts bytes outside the range 0x20..0xfe as-is.
    useful for templating with newlines and utf bytes.

    This distinction exists because UTF has a bunch of characters that can hide a surprise - lk_eval input and lk_encode output is only ever ascii.
    ```
    # use linkspace::{*,prelude::*,abe::*};
    # fn main() -> LkResult{
    assert_eq!( "abc 🔗🔗".as_bytes() as &[u8], &lk_eval_loose( "abc 🔗🔗" ,())?, );
    assert_eq!( "\0\0\0\0\0\0\0\0\0\0\0\0🔗 4036990103".as_bytes() as &[u8], &lk_eval_loose( "[a:🔗] [:🔗/?u]",())?, );
    # Ok(())}
    ```
    **/
    pub fn lk_eval_loose<'o>(expr: &str, udata: impl Into<UserData<'o>>) -> LkResult<Vec<u8>> {
        varscope::lk_eval(scope::scope(udata.into())?, expr, true)
    }
    /**
    An abe parser. Useful to split a cli argument like 'domain:[#:test]:/thing/[12/u32] correctly.
    The callback is called with (ctrl, contains_brackets, bytes ) where ctrl is 0 | ':' | '/'
    Only the first ctrl can be '\0'.
    ```
    # use linkspace::abe::lk_tokenize_abe;
    let mut v = vec![];
    lk_tokenize_abe("this:is/the:example[::]\n[::]newline[:/]",b"/",|expr,has_brackets,ctr| { v.push((expr,has_brackets,ctr)); true} );
    assert_eq!(v,&[(0,false,"this"), (b':',false,"is/the"), (b':',true,"example[::]"),(b'\n',true,"[::]newline[:/]")])
    ```
     **/
    pub fn lk_tokenize_abe<'o>(
        expr: &'o str,
        exclude_ctr: &[u8],
        mut per_comp: impl FnMut(u8, bool, &'o str) -> bool,
    ) -> LkResult<()> {
        let plain = exclude_ctr
            .iter()
            .filter_map(|v| CtrChar::try_from_char(*v))
            .fold(0, |a, b| a ^ b as u32);
        for (a, b, c) in tokenize_abe(expr, plain, 0)? {
            if !per_comp(a, b, c) {
                return Ok(());
            }
        }
        Ok(())
    }
    /**
    encode bytes as an abe that evaluate back to bytes.

    accepts a list of func to try and encode.
    This function can also be used as the evaluator '[/?:..]'.
    ```
    # use linkspace::{*,abe::*};
    # fn main() -> LkResult{
    let bytes= lk_eval("[u32:8]",())?;
    assert_eq!(bytes,&[0,0,0,8]);
    assert_eq!(lk_encode(&bytes,"/:"), r#"\0\0\0\x08"#);
    assert_eq!(lk_encode(&bytes,"u32"), "[u32:8]");


    // the options are a list of '/' separated functions
    // In this example 'u32' wont fit, LNS '#' lookup will succeed, if not the encoding would be base64

    let public_grp = PUBLIC;
    assert_eq!(lk_encode(&*public_grp,"u32/#/b"), "[#:pub]");

    // We can get meta - encode is also available as a scope during lk_eval

    // As the '?' function - with the tail argument being a single reverse option
    assert_eq!(lk_eval(r#"[?:\0\0\0[u8:8]:u32]"#,())?,b"[u32:8]");
    assert_eq!(lk_eval(r#"[:\0\0\0[u8:8]/?:u32]"#,())?,b"[u32:8]");

    // Or as the '?' macro
    assert_eq!(lk_eval(r#"[/?:\0\0\0[u8:8]/u32/#/b]"#,())?,b"[u32:8]");

    # Ok(())
    # }
    ```
    encode doesn't error, it falls back on plain abtxt.
    this variant also swallows bad options, see [lk_try_encode] to avoid doing so.
    **/
    pub fn lk_encode(bytes: impl AsRef<[u8]>, options: &str) -> String {
        let bytes = bytes.as_ref();
        varscope::lk_try_encode(scope::scope(().into()).unwrap(), bytes, options, true)
            .unwrap_or_else(|_v| as_abtxt(bytes).to_string())
    }
    /** [lk_encode] with Err on:
    - wrong options
    - no result ( use a /: to fallback to abtxt)
    - if !ignore_encoder_err on any encoder error
    **/
    pub fn lk_try_encode(
        bytes: impl AsRef<[u8]>,
        options: &str,
        ignore_encoder_err: bool,
    ) -> LkResult<String> {
        let bytes = bytes.as_ref();
        varscope::lk_try_encode(
            scope::scope(().into()).unwrap(),
            bytes,
            options,
            ignore_encoder_err,
        )
    }
    /// build a custom scope for ABE for use in [varscope]
    pub mod scope {

        #[cfg(feature = "runtime")]
        #[thread_local]
        pub(crate) static LK_EVAL_SCOPE_RT: std::cell::RefCell<
            Option<linkspace_common::runtime::Linkspace>,
        > = std::cell::RefCell::new(None);

        use core::fmt;

        use anyhow::Context;
        use linkspace_common::abe::eval::Scope;
        use linkspace_common::prelude::scope::ArgV;
        use linkspace_common::prelude::NetPkt;

        use crate::LkResult;
        pub(crate) type StdScope<'o> = impl Scope + 'o;
        /// Custom scope used in [crate::varscope] build with [core_scope], [scope], or [lk_scope] (default)
        pub struct LkScope<'o>(pub(crate) InlineScope<'o>);
        impl<'o> fmt::Debug for LkScope<'o> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("LkScope").field(&"_").finish()
            }
        }

        #[allow(clippy::large_enum_variant)]
        pub(crate) enum InlineScope<'o> {
            Std(StdScope<'o>),
            Dyn(&'o dyn Scope),
            Core,
            // TODO UserCb
        }

        #[derive(Copy, Clone, Default, Debug)]
        #[repr(C)]
        /// User config for adding common scopes
        pub struct UserData<'o> {
            /// Set a packet in scope e.g. "\[hash:str\] in group \[group:str\]"
            pub pkt: Option<&'o dyn NetPkt>,
            /// Add the argv scope e.g. "\[0\] and \[1\]"
            pub argv: Option<&'o [&'o [u8]]>,
        }
        impl From<()> for UserData<'static> {
            fn from(_: ()) -> Self {
                UserData {
                    pkt: None,
                    argv: None,
                }
            }
        }
        impl<'o> From<&'o dyn NetPkt> for UserData<'o> {
            fn from(pkt: &'o dyn NetPkt) -> Self {
                UserData {
                    pkt: Some(pkt),
                    argv: None,
                }
            }
        }
        impl<'o> From<(&'o dyn NetPkt, &'o [&'o [u8]])> for UserData<'o> {
            fn from((p, i): (&'o dyn NetPkt, &'o [&'o [u8]])) -> Self {
                UserData {
                    pkt: Some(p),
                    argv: Some(i),
                }
            }
        }

        impl<'o, const N: usize> From<&'o [&'o [u8]; N]> for UserData<'o> {
            fn from(inp: &'o [&'o [u8]; N]) -> Self {
                UserData {
                    argv: Some(inp),
                    pkt: None,
                }
            }
        }
        impl<'o> From<&'o [&'o [u8]]> for UserData<'o> {
            fn from(inp: &'o [&'o [u8]]) -> Self {
                UserData {
                    argv: Some(inp),
                    pkt: None,
                }
            }
        }

        /// create the a core_scope - includes basic byte functions (eval "\[help\]" to see the full scope)
        pub const fn core_scope() -> LkScope<'static> {
            LkScope(InlineScope::Core)
        }

        #[cfg(feature = "runtime")]
        /// the default scope used with lk_eval - includes runtime dependent scopes
        pub fn scope(udata: UserData<'_>) -> LkResult<LkScope<'_>> {
            _scope(None, udata, false)
        }
        #[cfg(not(feature = "runtime"))]
        /// the default scope used with lk_eval - includes runtime dependent scopes
        pub fn scope(udata: UserData<'_>) -> LkResult<LkScope<'_>> {
            use linkspace_common::prelude::*;

            let argv = udata
                .argv
                .map(|v| ArgV::try_fit(v).context("Too many inp values"))
                .transpose()?
                .map(EScope);
            Ok(LkScope(InlineScope::Std((
                udata.pkt.map(|v| pkt_scope(v)),
                core_scope(),
                argv,
            ))))
        }

        #[cfg(feature = "runtime")]
        /// [scope] with a explicit Linkspace and optionally add the os environment scope (used with `[env:ENV_VAR]`)
        pub fn lk_scope<'o>(
            lk: Option<&'o crate::Linkspace>,
            udata: UserData<'o>,
            enable_env: bool,
        ) -> LkResult<LkScope<'o>> {
            _scope(Some(lk.map(|o| &o.0)), udata, enable_env)
        }
        #[cfg(feature = "runtime")]
        fn _scope<'o>(
            lk: Option<Option<&'o linkspace_common::runtime::Linkspace>>,
            udata: UserData<'o>,
            enable_env: bool,
        ) -> LkResult<LkScope<'o>> {
            use linkspace_common::prelude::*;
            let argv = udata
                .argv
                .map(|v| ArgV::try_fit(v).context("Too many inp values"))
                .transpose()?
                .map(EScope);
            let get = move || {
                match lk {
                    None => LK_EVAL_SCOPE_RT.borrow().as_ref().cloned(),
                    Some(v) => v.cloned(),
                }
                .ok_or_else(|| anyhow::anyhow!("no linkspace instance was set"))
            };
            Ok(LkScope(InlineScope::Std((
                udata.pkt.map(pkt_scope),
                lk_scope(get, enable_env),
                argv,
            ))))
        }
        impl<'o> LkScope<'o> {
            pub(crate) fn as_dyn(&self) -> &(dyn Scope + 'o) {
                match &self.0 {
                    InlineScope::Std(scope) => scope,
                    InlineScope::Core => &linkspace_common::prelude::CORE_SCOPE,
                    InlineScope::Dyn(scope) => scope,
                }
            }
            #[doc(hidden)]
            pub fn from_dyn(sdyn: &'o dyn Scope) -> Self {
                LkScope(InlineScope::Dyn(sdyn))
            }
        }
    }
}

pub use query::{lk_query, lk_query_parse, lk_query_print, lk_query_push, Query, Q};
/// query functions to match points
pub mod query {
    /**
    A set of predicates and options used to select packets

    The supported predicate fields are found in [PredicateType].
    The known options are found in [KnownOptions].

    ```
    # use linkspace::{*,prelude::*,abe::*};
    # fn main() -> LkResult{

    let mut query = lk_query(&Q);

    // Add multiple predicates and options at once.
    let query_str = [
      "group:=:[#:pub]",
      "domain:=:[a:hello]",
      "prefix:=:/some/space",
      ":qid:default"
    ];
    query = lk_query_parse(query,&query_str,())?;
    // Optionally with user data such as an argv
    query = lk_query_parse(query,&["prefix:=:/some/[0]"],&[b"space" as &[u8]])?;

    // conflicting predicates return an error
    let result = lk_query_parse(lk_query(&query), &["depth:=:[u8:0]"],());
    assert!( result.is_err());

    // You can add a single statement directly
    query = lk_query_push(query,"create","<", &*now())?;
    // Predicates get merged if they overlap
    query = lk_query_push(query,"create","<",&lk_eval("[now:-1D]",())?)?;

    // As shown with:
    println!("{}",lk_query_print(&query,false));
    # Ok(()) }
    ```

    Unlike predicates, options are non-associative. i.e. the order in which they're added matters.
    'push' and 'parse' add them to the top of the query.
    In string form you find the matching option by reading from top to bottom.
    */
    #[derive(Clone)]
    /// a set of predicates and options to select/filter packets
    pub struct Query(pub(crate) linkspace_common::core::query::Query);

    /// The empty_query
    pub static Q: Query = Query(linkspace_common::core::query::Query::DEFAULT);

    impl std::fmt::Display for Query {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }
    impl std::fmt::Debug for Query {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }
    use std::ops::ControlFlow;

    use anyhow::Context;
    pub use linkspace_common::core::predicate::predicate_type::PredicateType;
    pub use linkspace_common::core::query::KnownOptions;
    use linkspace_common::prelude::ExtPredicate;

    use crate::abe::scope::UserData;

    use super::*;
    /// Create a new [Query]. Copy from a template. [Q] is the empty query.
    pub fn lk_query(copy_from: &Query) -> Query {
        Query(copy_from.0.clone())
    }
    /// Create a new [Query] specifically for a hash. Sets the right mode and 'i' count. See [runtime::lk_get_hashes] if you don't care to watch the db.
    pub fn lk_hash_query(hash: LkHash) -> Query {
        Query(linkspace_common::core::query::Query::hash_eq(hash))
    }

    /// Add a single statement to a [Query], potentially skipping an encode step.
    /// i.e. fast path for adding a single statement - lk_query_parse(q,"{field}:{op}:{lk_encode(bytes)}")
    ///
    /// Also accepts options in the form ```lk_query_push(q,"","mode",b"tree-asc")```.
    /// Unlike predicates, options don't join and instead are pushed to the front of the query;
    pub fn lk_query_push(mut query: Query, field: &str, test: &str, val: &[u8]) -> LkResult<Query> {
        if field.is_empty() {
            query.0.add_option(test, &[val]);
            return Ok(query);
        }
        let epre = ExtPredicate {
            kind: field.parse().with_context(|| format!("Field={field}"))?,
            op: test.parse().with_context(|| format!("Operator={test}"))?,
            val: val.to_vec().into(),
        };
        query.0.predicates.add_ext_predicate(epre)?;
        Ok(query)
    }
    /// Add multiple ABE encoded statements to a [Query]
    pub fn lk_query_parse<'o>(
        query: Query,
        expr: &[&str],
        udata: impl Into<UserData<'o>>,
    ) -> LkResult<Query> {
        varscope::lk_query_parse(crate::abe::scope::scope(udata.into())?, query, expr)
    }
    /// Clear a [Query] for reuse
    pub fn lk_query_clear(query: &mut Query) {
        //if fields.is_some() || keep_options { todo!()}
        *query = Q.clone();
    }
    /// Get the string representation of a [Query]
    pub fn lk_query_print(query: &Query, as_expr: bool) -> String {
        query.0.to_str(as_expr)
    }

    /// Compile a [Query] into a function which tests packets to deteremine if they match - WARN - slow and subject to change.
    #[allow(clippy::type_complexity)]
    pub fn lk_query_compile(
        q: Query,
    ) -> LkResult<Box<dyn FnMut(&dyn NetPkt) -> (bool, ControlFlow<()>)>> {
        Ok(q.0.compile()?)
    }
}

#[cfg(feature = "runtime")]
pub use key::lk_key;
/// cryptographic key functions for use in [lk_keypoint]
pub mod key {
    use super::prelude::*;
    use super::LkResult;
    use linkspace_common::identity;

    /// generate a new key
    pub fn lk_keygen() -> SigningKey {
        SigningKey::generate()
    }
    /** Encrypt the private key into a storable/share-able string - sometimes called an enckey
    Uses argon2d (using the public key as salt).
    An empty password sets the difficulty to trivial.
    **/
    pub fn lk_key_encrypt(key: &SigningKey, password: &[u8]) -> String {
        identity::encrypt(
            key,
            password,
            if password.is_empty() {
                Some(identity::INSECURE_COST)
            } else {
                None
            },
        )
    }
    /// decrypt the result of [lk_key_encrypt]
    pub fn lk_key_decrypt(key: &str, password: &[u8]) -> LkResult<SigningKey> {
        Ok(linkspace_common::identity::decrypt(key, password)?)
    }
    /// read the public key portion of a [lk_key_encrypt] string
    pub fn lk_key_pubkey(key: &str) -> LkResult<PubKey> {
        Ok(linkspace_common::identity::pubkey(key)?.into())
    }

    /** linkspace stored identity
    open (or generate) the key `name` which is also accessible as \[@:name:local\].
    empty name defaults to ( i.e. \[@:me:local\] )
    **/
    #[cfg(feature = "runtime")]
    pub fn lk_key(
        linkspace: &Linkspace,
        password: Option<&[u8]>,
        name: Option<&str>,
        create: bool,
    ) -> LkResult<SigningKey> {
        super::varscope::lk_key(
            super::abe::scope::scope(().into())?,
            linkspace,
            password,
            name,
            create,
        )
    }
}

#[cfg(feature = "runtime")]
pub use runtime::{
    cb::try_cb, lk_get, lk_open, lk_process, lk_process_while, lk_save, lk_stop, lk_watch,
    Linkspace,
};
#[cfg(feature = "runtime")]
/// a runtime to watch for new points from other processes or threads
pub mod runtime {
    /**
    The linkspace runtime.

    It connects to a database and a IPC event source
    Open/create with [lk_open].
    save with [lk_save].
    Use [lk_process] or [lk_process_while] to update the reader
    and get packets with [lk_get], [lk_get_all], and [lk_watch].
    **/

    #[derive(Clone)]
    #[repr(transparent)]
    pub struct Linkspace(pub(crate) LinkspaceImpl);
    use crate::interop::rt_interop::LinkspaceImpl;

    impl std::fmt::Debug for Linkspace {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_tuple("Linkspace").field(&"_").finish()
        }
    }

    use std::time::Instant;

    use cb::PktHandler;
    use linkspace_common::{prelude::QueryIDRef, saturating_cast, saturating_neg_cast};
    use tracing::debug_span;

    use super::*;
    /// open a linkspace runtime.
    ///
    /// will look at `path` | $LK_DIR | '$HOME/linkspace'
    ///
    /// A runtime is used in many arguments.
    /// Most notable to [lk_save], [lk_get], and [lk_watch] packets.
    /// The database is shared across threads and processes.
    /// The runtime (i.e. lk_watch) is not.
    /// The first call (per thread) sets the default instance for functions like [lk_eval] (see [varscope] for more options).
    /// Moving an open runtime across threads is not supported.

    pub fn lk_open(dir: Option<&std::path::Path>, create: bool) -> std::io::Result<Linkspace> {
        let rt = linkspace_common::static_env::open_linkspace_dir(dir, create)?;
        let mut eval_scope = crate::abe::scope::LK_EVAL_SCOPE_RT.borrow_mut();
        if eval_scope.is_none() {
            *eval_scope = Some(rt.clone())
        }
        Ok(Linkspace(rt))
    }
    /*
    /// TODO open a linkspace runtime in memory
    pub fn lk_inmem() -> std::io::Result<Linkspace> {
        todo!()
    }
    */

    /// save a packet. Returns true if new and false if its old.
    pub fn lk_save(lk: &Linkspace, pkt: &dyn NetPkt) -> std::io::Result<bool> {
        lk.0.env().save_dyn_one(pkt).map(|o| o.is_written())
    }
    /// save multiple packets at once - returns the number of new packets written
    pub fn lk_save_all(lk: &Linkspace, pkts: &[&dyn NetPkt]) -> std::io::Result<usize> {
        let (start, excl) = lk_save_all_ext(lk, pkts)?;
        Ok((excl.get() - start.get()) as usize)
    }
    /// returns the range [incusive,exclusive) of recv stamps used to save new packets. total_new = r.1-r.0
    pub fn lk_save_all_ext(
        lk: &Linkspace,
        pkts: &[&dyn NetPkt],
    ) -> std::io::Result<(Stamp, Stamp)> {
        let range = lk.0.env().save_dyn_iter(pkts.iter().copied())?;
        Ok((range.start.into(), range.end.into()))
    }

    /// Run callback for every match for the query in the database.
    /// Break early if the callback returns true.
    /// returns number of matches
    /// if return from break return is -1*Number of matches
    /// If break => number of matches
    /// If no break ( cb only returned false ) => -1 * Number of matches
    pub fn lk_get_all(
        lk: &Linkspace,
        query: &Query,
        cb: &mut dyn FnMut(&dyn NetPkt) -> bool,
    ) -> LkResult<i32> {
        let mut c = 0;
        let r = lk.0.get_reader();
        let mode = query.0.get_mode()?;
        let mut breaks = false;
        for p in r.query(mode, &query.0.predicates, &mut c)? {
            breaks = (cb)(&p);
            if breaks {
                break;
            }
        }
        if breaks {
            return Ok(saturating_neg_cast(c));
        }
        Ok(saturating_cast(c))
    }

    /// get the first result from the database matching the query.
    pub fn lk_get(lk: &Linkspace, query: &Query) -> LkResult<Option<NetPktBox>> {
        lk_get_ref(lk, query, &mut |v| v.as_netbox())
    }
    /** read a single packet directly without copying when possible. **/
    pub fn lk_get_ref<A>(
        lk: &Linkspace,
        query: &Query,
        cb: &mut dyn FnMut(&dyn NetPkt) -> A,
    ) -> LkResult<Option<A>> {
        let mode = query.0.get_mode()?;
        let mut i = 0;
        let reader = lk.0.get_reader();
        let opt_pkt = reader.query(mode, &query.0.predicates, &mut i)?.next();
        Ok(opt_pkt.map(|p| cb(&p)))
    }

    /// todo
    pub fn lk_get_hashes(
        lk: &Linkspace,
        hashes: &[LkHash],
        cb: &mut dyn FnMut(&dyn NetPkt) -> bool,
    ) -> LkResult<i32> {
        let mut c = 0;
        let r = lk.0.get_reader();
        for p in r.get_pkts_by_hash(hashes.iter().copied()) {
            c += 1;
            if (cb)(&p) {
                return Ok(saturating_neg_cast(c));
            }
        }
        Ok(saturating_cast(c))
    }

    /**
    Registers the query under its 'qid' ( .e.g. set by lk_query_parse(q,":qid:myqid") )
    Before returning, calls cb for every packet in the database.
    The absolute return value is the number of times the callback was called.
    A positive value means the callback finished already.

    A 0 or negative value means it is registered and shall be called during
    a [[lk_process]] or [[lk_process_while]] call,
    for every new packet matching the query.

    The watch is finished when
    - the cb returns 'break' (In other languages we map this to the boolean 'true')
    - the predicate set will never match again (e.g. the 'i_*' or 'recv' predicate shall never match again )
    - [[lk_stop]] is called with the matching id

    **/
    pub fn lk_watch(lk: &Linkspace, query: &Query, cb: impl PktHandler + 'static) -> LkResult<i32> {
        lk_watch2(lk, query, cb, debug_span!("lk_watch - (untraced)"))
    }

    /// [lk_watch] with a custom log [tracing::Span]
    /// The span will be entered on every callback.
    /// If you do not care for structured options you can use [vspan] `lk_watch2(.. , .. ,.. , vspan("my function"))`
    pub fn lk_watch2(
        lk: &Linkspace,
        query: &Query,
        cb: impl PktHandler + 'static,
        span: tracing::Span,
    ) -> LkResult<i32> {
        lk.0.watch_query(&query.0, interop::rt_interop::Handler(cb), span)
    }
    /// See [lk_watch2]
    pub fn vspan(name: &str) -> tracing::Span {
        tracing::debug_span!("{}", name)
    }

    /// close lk_watch watches based on the query id ':qid:example' in the query.
    pub fn lk_stop(lk: &Linkspace, id: &[u8], range: bool) {
        if range {
            lk.0.close_range(id)
        } else {
            lk.0.close(id)
        }
    }

    /// process the log of new packets and trigger callbacks. Updates the reader to the latest state.
    pub fn lk_process(lk: &Linkspace) -> Stamp {
        lk.0.process()
    }
    /**
    continuously process callbacks until:
    - timeout time has passed
    - qid = Some and qid is matched at least once => if removed returns 1, if still registered returns -1
    - qid = None => no more callbacks (1)
     **/
    pub fn lk_process_while(
        lk: &Linkspace,
        qid: Option<&QueryIDRef>,
        timeout: Stamp,
    ) -> LkResult<isize> {
        let timeout = (timeout != Stamp::ZERO)
            .then(|| Instant::now() + std::time::Duration::from_micros(timeout.get()));
        _lk_process_while(lk, qid, timeout)
    }
    #[doc(hidden)]
    // simplifies python ffi bindings
    pub fn _lk_process_while(
        lk: &Linkspace,
        qid: Option<&QueryIDRef>,
        timeout: Option<Instant>,
    ) -> LkResult<isize> {
        lk.0.run_while(timeout, qid)
    }

    /// iterate over all active (Qid,Query)
    pub fn lk_list_watches(lk: &Linkspace, cb: &mut dyn FnMut(&[u8], &Query)) {
        for el in lk.0.dbg_watches().entries() {
            cb(&el.query_id, Query::from_impl(&el.query))
        }
    }
    #[derive(Debug)]
    /// miscellaneous information about the runtime
    pub struct LkInfo<'o> {
        /// the kind of runtime in use - currently only known is "lmdb"
        pub kind: &'static str,
        /// the path under which it is saved
        pub dir: &'o std::path::Path,
    }
    /// get [LkInfo] of a linkspace runtime
    pub fn lk_info(lk: &Linkspace) -> LkInfo {
        LkInfo {
            kind: "lmdb",
            dir: lk.0.env().dir(),
        }
    }

    #[cfg(feature = "runtime")]
    /** (rust only) [lk_watch] takes the callback [PktHandler] which are quick to impl with [cb] and [try_cb].
    Other languages should use their own function syntax as argument to lk_watch.
    **/
    pub mod cb {

        use std::ops::{ControlFlow, Try};

        use linkspace_common::prelude::NetPkt;

        /// Callbacks stored in a [Linkspace] instance. use [cb] and [try_cb] to impl from a single function.
        pub trait PktHandler {
            /// Handles an event.
            fn handle_pkt(&mut self, pkt: &dyn NetPkt, lk: &Linkspace) -> ControlFlow<()>;
            /// Called when break, finished, or replaced
            fn stopped(
                &mut self,
                _: Query,
                _: &Linkspace,
                _: StopReason,
                _total_calls: u32,
                _watch_calls: u32,
            ) {
            }
        }
        impl PktHandler for Box<dyn PktHandler> {
            fn handle_pkt(&mut self, pkt: &dyn NetPkt, lk: &Linkspace) -> ControlFlow<()> {
                (**self).handle_pkt(pkt, lk)
            }

            fn stopped(
                &mut self,
                query: Query,
                lk: &Linkspace,
                reason: StopReason,
                total: u32,
                new: u32,
            ) {
                (**self).stopped(query, lk, reason, total, new)
            }
        }
        pub use linkspace_common::runtime::handlers::StopReason;

        use crate::{Linkspace, Query};

        #[derive(Copy, Clone)]
        struct Cb<A> {
            handle_pkt: A,
            //  stopped: B,// unused but might enable later
        }
        /// takes a `fn(&dyn NetPkt,&Linkspace) -> bool[should_continue]` and returns impl [PktHandler]
        pub fn cb(mut handle_pkt: impl FnMut(&dyn NetPkt, &Linkspace) -> bool) -> impl PktHandler {
            Cb {
                handle_pkt: move |pkt: &dyn NetPkt, lk: &Linkspace| {
                    if (handle_pkt)(pkt, lk) {
                        ControlFlow::Break(())
                    } else {
                        ControlFlow::Continue(())
                    }
                },
            }
        }

        /// takes any fn(&dyn NetPkt,&Linkspace) -> Try (e.g. Result or Option) and returns impl [PktHandler] that logs on break
        pub fn try_cb<A, R, E>(mut handle_pkt: A) -> impl PktHandler
        where
            R: Try<Output = (), Residual = E>,
            E: std::fmt::Debug,
            A: for<'a, 'b> FnMut(&'a dyn NetPkt, &'b Linkspace) -> R + 'static,
        {
            Cb {
                handle_pkt: move |pkt: &dyn NetPkt, lk: &Linkspace| {
                    (handle_pkt)(pkt, lk)
                        .branch()
                        .map_break(|brk| tracing::info!(?brk, "break"))
                },
            }
        }

        impl<A> PktHandler for Cb<A>
        where
            A: FnMut(&dyn NetPkt, &Linkspace) -> ControlFlow<()>,
        {
            fn handle_pkt(&mut self, pkt: &dyn NetPkt, lk: &Linkspace) -> ControlFlow<()> {
                (self.handle_pkt)(pkt, lk)
            }
        }
    }
}

/// A set of functions that adhere to conventions
pub mod conventions;
#[cfg(feature = "runtime")]
pub use crate::conventions::pull::lk_pull;

pub use consts::{PRIVATE, PUBLIC};
/// consts for common groups & domains
pub mod consts {
    pub use linkspace_common::core::consts::pkt_consts::*;
    pub use linkspace_common::core::consts::{EXCHANGE_DOMAIN, PRIVATE, PUBLIC, TEST_GROUP};
}

/// misc functions & tools - less stable
pub mod misc {
    pub use linkspace_common::pkt::netpkt::cmp::PktCmp;
    pub use linkspace_common::pkt::netpkt::DEFAULT_ROUTING_BITS;
    pub use linkspace_common::pkt::read;
    pub use linkspace_common::pkt::reroute::{RecvPkt, ReroutePkt, ShareArcPkt};
    pub use linkspace_common::pkt::tree_order::TreeEntry;
    pub use linkspace_common::pkt::FieldEnum;
    pub use linkspace_common::pkt_stream_utils::QuickDedup;

    use linkspace_common::prelude::B64;

    /// Blake3 hash
    pub fn blake3_hash(val: &[u8]) -> B64<[u8; 32]> {
        B64(*linkspace_common::core::crypto::blake3_hash(val).as_bytes())
    }

    /**
    Read bytes as a [0,1) float by reading the first 52 bits.
    Panics if fewer than 8 bytes are supplied
    Primary use is to produces the same 'random' value by using NetPkt::hash or [blake3_hash],
    regardless of language, and without an additional RNG dependencies,
     */
    pub fn bytes2uniform(val: &[u8]) -> f64 {
        let rand_u64 = u64::from_be_bytes(
            val[..8]
                .try_into()
                .expect("bytes2uniform requires at least 8 bytes"),
        );
        let f64_exponent_bits: u64 = 1023u64 << 52;
        // Generate a value in the range [1, 2)
        let value1_2 = f64::from_bits((rand_u64 >> (64 - 52)) | f64_exponent_bits);
        value1_2 - 1.0
    }

    /* TODO: remove
    /**
    Read a float [0,1) into 32 bytes (big endian).
    Its primary use is for the result can be compared to [NetPkt::hash] or [blake3_hash].
    These emulates randomness, regardless of language and without an additional RNG dependencies.
    i.e. if uniform2bytes(0.1) > pkt.hash { println!("This has a 0.1 chance")}
    */
    pub fn uniform2bytes(val:f64) -> anyhow::Result<B64<[u8;32]>>{
        let mut result = B64([0;32]);
        if val < 0.0 || val >= 1.0 { anyhow::bail!("function is inverse of bytes2uniform, thus undefined outside of [0,1)")}
        let bits = (val + 1.0).to_bits();
        let headbytes = (bits << (64-52)).to_be_bytes();
        result.0[0..8].copy_from_slice(&headbytes);
        result
    }
    */
}

/// Functions with a custom eval scope - useful for security or when [lk_open]'ing multiple different runtimes (only partially supported atm)
pub mod varscope {

    use super::*;
    use crate::abe::scope::LkScope;
    use linkspace_common::abe::{eval::eval, parse_abe};

    /// [crate::lk_eval]/[crate::abe::lk_eval_loose] with a custom scope
    pub fn lk_eval(scope: LkScope, expr: &str, loose: bool) -> LkResult<Vec<u8>> {
        let expr = parse_abe(expr, loose)?;
        let val = eval(&scope.as_dyn(), &expr)?;
        Ok(val.concat())
    }
    /// [lk_eval] with a custom scope that errors on bad option & can explicit trigger error on no encoding found.
    pub fn lk_try_encode(
        scope: LkScope,
        bytes: &[u8],
        options: &str,
        ignore_encoder_err: bool,
    ) -> LkResult<String> {
        Ok(linkspace_common::abe::eval::encode(
            &scope.as_dyn(),
            bytes,
            options,
            ignore_encoder_err,
        )?)
    }
    /// custom scope version of [super::lk_query_parse]
    pub fn lk_query_parse(
        scope: LkScope,
        mut query: Query,
        statements: &[&str],
    ) -> LkResult<Query> {
        for stmnt in statements {
            query.0.parse(stmnt.as_bytes(), &scope.as_dyn())?;
        }
        Ok(query)
    }
    #[cfg(feature = "runtime")]
    /// [lk_key] with a custom context
    pub fn lk_key(
        scope: LkScope,
        linkspace: &Linkspace,
        password: Option<&[u8]>,
        name: Option<&str>,
        create: bool,
    ) -> LkResult<SigningKey> {
        use std::borrow::Cow;

        use linkspace_common::{prelude::parse_abe_strict_b, protocols::lns};

        let name = match name {
            Some(v) => Cow::Borrowed(v),
            None => std::env::var("LK_KEYNAME")
                .map(Cow::Owned)
                .unwrap_or(Cow::Borrowed("me:local")),
        };
        let password = match password {
            Some(v) => Cow::Borrowed(v),
            None => match std::env::var("LK_PASS") {
                Ok(abe) => Cow::Owned(
                    eval(&scope.as_dyn(), &parse_abe_strict_b(abe.as_bytes())?)?.concat(),
                ),
                Err(_e) => Cow::Borrowed(&[] as &[u8]),
            },
        };
        let expr = parse_abe_strict_b(name.as_bytes())?;
        let name: lns::name::Name = eval(&scope.as_dyn(), &expr)?.try_into()?;
        match lns::lookup_enckey(&linkspace.0, &name)? {
            Some((_, enckey)) => Ok(linkspace_common::identity::decrypt(&enckey, &password)?),
            None => {
                if create {
                    use super::key::*;
                    let key = lk_keygen();
                    let enckey = super::key::lk_key_encrypt(&key, &password);
                    lns::setup_special_keyclaim(&linkspace.0, name, &enckey, false)?;
                    Ok(key)
                } else {
                    anyhow::bail!("no matching entry found")
                }
            }
        }
    }
}

// Allow for interop when importing linkspace_common
#[doc(hidden)]
pub mod interop;
