// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![feature(
    thread_local,
    io_error_other,
    once_cell,
    write_all_vectored,
    control_flow_enum,
    type_alias_impl_trait,
    concat_bytes,
    try_trait_v2,
    strict_provenance
)]
#![doc = include_str!("../readme.md")]
#![doc = r#"

The functions re-exported below is essentially the entire liblinkspace interface.
Bindings in other languges follow the same pattern.

[prelude] includes some additional utilities.
Some internals structs defs are currently leaking and will be removed.
"#]
use std::ops::ControlFlow;

use linkspace_common::{pkt, runtime::handlers::StopReason};

pub type LkError = anyhow::Error;
pub type LkResult<T = ()> = std::result::Result<T, LkError>;

pub mod prelude {
    pub use super::*;
    pub use linkspace_common::{
        bytefmt::{endian_types, AB, B64},
        core::{env::queries::RecvPktPtr},
        pkt::{
            ab, as_abtxt, ipath1, ipath_buf, now, try_ab, Domain,  GroupID, LkHash, IPath,
            IPathBuf, IPathC, Link, NetFlags, NetPkt, NetPktArc, NetPktBox, NetPktExt,
            NetPktHeader, NetPktParts, NetPktPtr, PktTypeFlags, Point, PointExt, Ptr, PubKey,
            SPath, SPathBuf, SigningExt, SigningKey, Stamp, Tag,
        },
    };
}
use prelude::*;

pub use prelude::SigningKey;

/// Callbacks stored in a [Linkspace] instance. use [misc::cb] to impl from function
pub trait PktHandler {
    // if returns some, periodcially check to see if the handler can be closed.
    //fn checkup(&mut self) -> Option<ControlFlow<()>>{None}
    /// Handles an event.
    fn handle_pkt(&mut self, pkt: &dyn NetPkt, lk: &Linkspace) -> ControlFlow<()>;
    /// Called when break, finished, or replaced
    fn stopped(&mut self, query: Query, lk: &Linkspace, reason: StopReason, total: u32, new: u32);
}
impl PktHandler for Box<dyn PktHandler>{
    fn handle_pkt(&mut self, pkt: &dyn NetPkt, lk: &Linkspace) -> ControlFlow<()> {
        (**self).handle_pkt(pkt, lk)
    }

    fn stopped(&mut self, query: Query, lk: &Linkspace, reason: StopReason, total: u32, new: u32) {
        (**self).stopped(query, lk, reason, total, new)
    }
}

pub use point::{lk_datapoint, lk_keypoint, lk_linkpoint};
pub mod point {
    use anyhow::Context;
    use std::io;

    use super::*;
    /**

    create a datapoint with upto MAX_CONTENT_SIZE bytes and wrap it as a [NetPktBox]

    ```
    # use liblinkspace::{*,prelude::*,abe::*};
    # fn main() -> LkResult{
    let datap = lk_datapoint(b"Some data")?;
    assert_eq!(datap.hash().to_string(), "ay01_aEzVcp0scyCgKqfugoQSXGW4iefLgAZRxRp9sY");
    assert_eq!(datap.data() , b"Some data");
    # Ok(())}
    ```

    **/
    pub fn lk_datapoint<'o>(data: &'o [u8]) -> LkResult<NetPktBox> {
        lk_datapoint_ref(data).map(|v| v.as_netbox())
    }
    pub fn lk_datapoint_ref<'o>(data: &'o [u8]) -> LkResult<NetPktParts<'o>> {
        Ok(pkt::try_datapoint_ref(data, pkt::NetOpts::Default)?.into())
    }
    /**

    create a new linkpoint [NetPktBox]
    ```
    # use liblinkspace::{*,prelude::{*,endian_types::*},abe::*};
    # fn main() -> LkResult{

    let datap = lk_datapoint(b"this is a datapoint")?;
    let path = ipath_buf(&[b"hello",b"world"]);
    let links = [
        Link{tag: ab(b"a datapoint"),ptr:datap.hash()},
        Link{tag: ab(b"another tag"),ptr:PUBLIC_GROUP}
    ];
    let data = b"extra data for the linkpoint";
    let create = Some(U64::new(0)); // None == Some(now()).
    let linkpoint = lk_linkpoint(ab(b"mydomain"),PUBLIC_GROUP,&path,&links,data,create)?;

    assert_eq!(linkpoint.hash().to_string(), "zvyWklJrmEHBQfYBLxYh7Gh-3YOTCFRgyuXaGl6-xt8");
    assert_eq!(linkpoint.data(), data);
    assert_eq!(*linkpoint.get_group(), PUBLIC_GROUP); 

    # Ok(())}

    ```
    **/
    pub fn lk_linkpoint(
        domain: Domain,
        group: GroupID,
        path: &IPath,
        links: &[Link],
        data: &[u8],
        create_stamp: Option<Stamp>,
    ) -> LkResult<NetPktBox> {
        lk_linkpoint_ref(domain, group, path, links, data, create_stamp).map(|v| v.as_netbox())
    }
    pub fn lk_linkpoint_ref<'o>(
        domain: Domain,
        group: GroupID,
        path: &'o IPath,
        links: &'o [Link],
        data: &'o [u8],
        create_stamp: Option<Stamp>,
    ) -> LkResult<NetPktParts<'o>> {
        Ok(pkt::try_linkpoint_ref(
            group,
            domain,
            path,
            links,
            data,
            create_stamp.unwrap_or_else(pkt::now),
            pkt::NetOpts::Default,
        )?)
    }
    /// create a keypoint and wrap it as a [NetPktBox]. i.e. a signed [lk_linkpoint]
    pub fn lk_keypoint(
        domain: Domain,
        group: GroupID,
        path: &IPath,
        links: &[Link],
        data: &[u8],
        create_stamp: Option<Stamp>,
        signkey: &SigningKey,
    ) -> LkResult<NetPktBox> {
        lk_keypoint_ref(domain, group, path, links, data, create_stamp, signkey)
            .map(|v| v.as_netbox())
    }
    pub fn lk_keypoint_ref<'o>(
        domain: Domain,
        group: GroupID,
        path: &'o IPath,
        links: &'o [Link],
        data: &'o [u8],
        create_stamp: Option<Stamp>,
        signkey: &SigningKey,
    ) -> LkResult<NetPktParts<'o>> {
        let create_stamp = create_stamp.unwrap_or_else(now);
        Ok(pkt::try_keypoint_ref(
            group,
            domain,
            path,
            links,
            data,
            create_stamp,
            signkey,
            pkt::NetOpts::Default,
        )?)
    }

    pub fn lk_read(buf: &[u8], validate: bool, allow_private: bool) -> LkResult<NetPktBox> {
        let pkt = super::misc::reader::parse_netpkt(buf, validate, allow_private)?
            .context("incomplete pkt")?;
        Ok(pkt.as_netbox())
    }

    pub fn lk_read_ref(buf: &[u8], validate: bool, allow_private: bool) -> LkResult<NetPktParts> {
        let pkt = super::misc::reader::parse_netpkt(buf, validate, allow_private)?
            .context("incomplete pkt")?;
        Ok(unsafe { pkt.get() }.as_netparts())
    }

    pub fn lk_write(p: &dyn NetPkt, out: &mut dyn io::Write) -> io::Result<()> {
        let mut segments = p.byte_segments().io_slices();
        out.write_all_vectored(&mut segments)
    }
}

pub use abe::{lk_encode, lk_eval,lk_split_abe};
/** ascii byte expression utilities

ABE is a byte templating language.
See guide#ABE to understand its use and indepth explanation.
 **/
pub mod abe {
    use super::*;
    pub use linkspace_common::pkt::repr::{DEFAULT_PKT};
    use linkspace_common::{
        abe::abtxt::as_abtxt,
        prelude::{abtxt::CtrChar, ast::split_abe},
    };
    /**
    Evaluate an expression and return the bytes

    Optionally add a `pkt` as a context.
    Refuses '\n' and '\t', and returns delimiters ':' and '/' as plain bytes.
    See [lk_split_abe] for different delimiter behaviour

    ```
    # use liblinkspace::{*,prelude::*,abe::*};
    # fn main() -> LkResult{
    assert_eq!( lk_eval( "abc" ,None)?, b"abc" );
    assert_eq!( lk_eval( r#"\0\x01\xff"# ,None)?, &[0,1,255] ); // "\\0\\x01\\xff"

    assert_eq!( lk_eval( r#"\n\t\:\/\\\{\}"# ,None)?, &[b'\n',b'\t',b':',b'/',b'\\',b'{',b'}']);


    assert_eq!( lk_eval( "The '' returns its first {:argument}" , None)?,b"The '' returns its first argument");

    assert_eq!( lk_eval( "Bytes{: are}{: joined}" , None)?,b"Bytes are joined");

    let result = lk_eval( "Nest expressions {u8:65} == {u8:{:65}} == \x41" , None)?;
    assert_eq!(result,   b"Nest expressions A == A == A");

    let result = lk_eval( "Use result as first argument with '/' {u8:65} == {:65/u8} == \x41" , None)?;
    assert_eq!(result,   b"Use result as first argument with '/' A == A == A");

    let lp : NetPktBox = lk_linkpoint(ab(b"mydomain"),PUBLIC_GROUP,IPath::empty(),&[],&[],None)?;

    assert_eq!( lk_eval( "{hash}" , Some(&lp))?,&*lp.hash());
    let by_arg   = lk_eval( "{hash:str}", Some(&lp))?;
    let by_apply = lk_eval( "{hash/?b}", Some(&lp))?;
    let as_field = lp.hash().b64().into_bytes();
    assert_eq!( by_arg, by_apply);
    assert_eq!( by_arg, as_field);

    # Ok(())
    # }
    ```

    A list of functions can be found with ```lk_eval("{help}")```
    **/
    pub fn lk_eval(expr: &str, pkt: Option<&dyn NetPkt>) -> LkResult<Vec<u8>> {
        varctx::lk_eval(ctx::ctx(pkt), expr)
    }
    /**
    Exec callback for each expr between control characters (':', '/', '\n', '\t').
    The last delimiter can be '\0'.
    ```
    # use liblinkspace::abe::lk_split_abe;
    let mut v = vec![];
    lk_split_abe("this:is/the:example\nnewline",b"/",|expr,ctr| { v.push((expr,ctr)); true} );
    assert_eq!(v,&[("this",b':'), ("is/the",b':'), ("example",b'\n'),("newline",0)])
    ```
     **/
    pub fn lk_split_abe<'o>(
        expr: &'o str,
        exclude_ctr: &[u8],
        mut per_comp: impl FnMut(&'o str, u8) -> bool,
    ) -> LkResult<()> {
        let plain = exclude_ctr
            .iter()
            .filter_map(|v| CtrChar::try_from_char(*v))
            .fold(0, |a, b| a ^ b as u32);
        for (c, d) in split_abe(expr, plain, 0)? {
            if !per_comp(c, d) {
                return Ok(());
            }
        }
        Ok(())
    }
    /**
    encode bytes as an abe that evaluate back to bytes.

    accepts a list of func to try and encode.
    This function can also be used as the evaluator '{/?:..}'.
    ```
    # use liblinkspace::{*,abe::*};
    # fn main() -> LkResult{
    let bytes= lk_eval("{u32:8}",None)?;
    assert_eq!(bytes,&[0,0,0,8]);
    assert_eq!(lk_encode(&bytes,""), r#"\0\0\0\x08"#);
    assert_eq!(lk_encode(&bytes,"u32"), "{u32:8}");

    // This function can also be called with the encode '/?' efunc
    assert_eq!(lk_eval(r#"{/?:\0\0\0{u8:8}:u32}"#,None)?,b"{u32:8}");

    // the options are a list of '/' seperated functions
    // In this example 'u32' wont fit, LNS '#' lookup will succeed, if not the encoding would be base64

    let mut public_grp = PUBLIC_GROUP;
    assert_eq!(lk_encode(&*public_grp,"u32/#/b"), "{#:pub}");

    # Ok(())
    # }
    ```
    encode doesn't error, it falls back on plain abtxt.
    this variant also swallows bad options, see [lk_try_encode] to avoid doing so.
    **/
    pub fn lk_encode(bytes: &[u8], options: &str) -> String {
        varctx::lk_try_encode(ctx::ctx(None), bytes, options)
            .unwrap_or_else(|_v| as_abtxt(bytes).to_string())
    }
    /// [lk_encode] with Err on wrong options
    pub fn lk_try_encode(bytes: &[u8], options: &str) -> LkResult<String> {
        Ok(varctx::lk_try_encode(ctx::ctx(None), bytes, options)?)
    }
    /*
    /// set default lk_eval context usually set by [lk_open] and
    pub fn lk_set_eval_ctx(rt:Option<&Linkspace>,pkt:Option<&NetPktPtr>) {
        *LK_EVAL_CTX_RT.borrow_mut() = rt.cloned();
        *LK_EVAL_CTX_PKT.borrow_mut() = pkt.map(|v| v.as_netarc());
    }
    */

    /// Custom context for use in [varctx]
    pub mod ctx {
        #[thread_local]
        pub static LK_EVAL_CTX_RT: RefCell<Option<Linkspace>> = RefCell::new(None);

        use linkspace_common::abe::eval::{EvalCtx, Scope};
        use linkspace_common::prelude::NetPkt;
        use linkspace_common::runtime::Linkspace;
        use std::cell::RefCell;
        type StdCtx<'o> = impl Scope + 'o;
        /// Create a new context for use in [crate::varctx] with [empty_ctx], [core_ctx], [ctx], or [lk_ctx] (default)
        pub struct LkCtx<'o>(pub(crate) InlineCtx<'o>);
        // we optimise for the instance where contains _ctx, but we expose several other context situations
        pub(crate) enum InlineCtx<'o> {
            Std(StdCtx<'o>),
            // TODO UserCb
            Core,
            Empty,
        }

        pub fn ctx<'o>(pkt: Option<&'o dyn NetPkt>) -> LkCtx<'o> {
            _ctx(None, pkt)
        }
        pub const fn core_ctx() -> LkCtx<'static> {
            LkCtx(InlineCtx::Core)
        }
        pub const fn empty_ctx() -> LkCtx<'static> {
            LkCtx(InlineCtx::Empty)
        }
        pub fn lk_ctx<'o>(lk: Option<&'o Linkspace>, pkt: Option<&'o dyn NetPkt>) -> LkCtx<'o> {
            _ctx(Some(lk), pkt)
        }
        // the basic context.
        fn _ctx<'o>(lk: Option<Option<&'o Linkspace>>, pkt: Option<&'o dyn NetPkt>) -> LkCtx<'o> {
            use linkspace_common::core::eval::EVAL0_1;
            use linkspace_common::eval::std_ctx_v;
            use linkspace_common::pkt::eval::opt_pkt_ctx;
            let get = move || {
                match lk {
                    None => LK_EVAL_CTX_RT.borrow().as_ref().cloned(),
                    Some(v) => v.cloned(),
                }
                .ok_or_else(|| std::io::Error::other("No runtime argument given"))
            };
            LkCtx(InlineCtx::Std(
                opt_pkt_ctx(std_ctx_v(get, EVAL0_1), pkt.map(|v| v as &dyn NetPkt)).scope,
            ))
        }
        impl<'o> LkCtx<'o> {
            pub(crate) fn as_dyn(&self) -> EvalCtx<&(dyn Scope + 'o)> {
                match &self.0 {
                    InlineCtx::Std(scope) => EvalCtx { scope },
                    InlineCtx::Core => EvalCtx {
                        scope: &linkspace_common::prelude::EVAL_SCOPE,
                    },
                    InlineCtx::Empty => EvalCtx { scope: &() },
                }
            }
        }
    }
}

pub use query::{lk_query, lk_query_parse, lk_query_print, lk_query_push, Query};
pub mod query {
    /**
    A set of predicates and options used to select packets

    The supported predicate fields are found in [PredicateType].
    The known options are found in [KnownOptions].

    ```
    # use liblinkspace::{*,prelude::*,abe::*};
    # fn main() -> LkResult{

    let mut query = lk_query();

    // Add an entire set
    let query_str = "

    group:=:{#:pub}
    domain:=:{a:hello}
    prefix:=:/some/path

    :watch:default
    ";
    lk_query_parse(&mut query,query_str)?;

    // conflicting requirements clash
    let result = lk_query_parse(&mut query, "path_len:=:\0");
    assert!( result.is_err());

    // You can add a single statement directly
    lk_query_push(&mut query,"create","<", &*now())?;
    // They are effectilvy merged if they overlap
    lk_query_push(&mut query,"create","<",&lk_eval("{now:-1D}",None)?)?;

    // to inspect it
    println!("{}",lk_query_print(&query,false));

    # Ok(())}
    ```

    **/
    #[derive(Default, Clone)]
    pub struct Query(pub(crate) linkspace_common::core::query::Query);

    impl std::fmt::Display for Query {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }
    use std::ops::ControlFlow;

    pub use linkspace_common::core::predicate::predicate_type::PredicateType;
    pub use linkspace_common::core::query::KnownOptions;
    use linkspace_common::prelude::{ExtPredicate, PktPredicates};

    use super::*;
    /// Create a new [Query]
    pub fn lk_query() -> Query {
        Query(Default::default())
    }
    /// Add a single statement to a [Query], potentially skipping an encode step.
    ///
    /// Also accepts options in the form ```lk_query_push(q,"","mode",b"tree-asc")```;
    pub fn lk_query_push(query: &mut Query, field: &str, test: &str, val: &[u8]) -> LkResult<bool> {
        if field.is_empty() {
            query.0.add_option(test, &[val]);
            return Ok(false);
        }
        let epre = ExtPredicate {
            kind: field.parse()?,
            op: test.parse()?,
            val: val.to_vec().into(),
        };
        let mut tmp = query.0.predicates.clone();
        tmp.add_ext_predicate(epre)?;
        let changed = tmp == query.0.predicates;
        query.0.predicates = tmp;
        Ok(changed)
    }
    /// Add multiple ABE encoded statements to a [Query]
    pub fn lk_query_parse(query: &mut Query, expr: &str) -> LkResult<bool> {
        varctx::lk_query_parse(crate::abe::ctx::ctx(None), query, expr)
    }
    /// Clear a [Query] for reuse
    pub fn lk_query_clear(query: &mut Query) {
        //if fields.is_some() || keep_options { todo!()}
        query.0.predicates = PktPredicates::DEFAULT;
        query.0.options.clear();
    }
    /// Get the string representation of a [Query]
    pub fn lk_query_print(query: &Query, as_expr: bool) -> String {
        query.0.to_str(as_expr)
    }

    /// Compile a [Query] into a function which tests packets to deteremine if they match - WARN - slow and subject to change.
    pub fn lk_query_compile(
        q: Query,
    ) -> LkResult<Box<dyn FnMut(&dyn NetPkt) -> (bool, ControlFlow<()>)>> {
        Ok(q.0.compile()?)
    }
}

pub use key::lk_key;
pub mod key {
    use super::prelude::*;
    use super::LkResult;
    use linkspace_common::identity;

    pub fn lk_keystr(key: &SigningKey, password: &[u8]) -> String {
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
    pub fn lk_keyopen(key: &str, password: &[u8]) -> LkResult<SigningKey> {
        Ok(linkspace_common::identity::decrypt(key, password)?)
    }
    pub fn lk_keygen() -> SigningKey {
        SigningKey::generate()
    }

    /** linkspace stored identity

    open (or generate) the key `name` which is also accessible as {@local:name}.
    empty name defaults to 'me' ( i.e. {@local:name} )
    **/
    pub fn lk_key(
        linkspace: &Linkspace,
        password: &[u8],
        name: &str,
        new: bool,
    ) -> LkResult<SigningKey> {
        let name = if name.is_empty() { "me" } else { name };
        let e = ["{local:{:", name, "}::*=:enckey/readhash:data}"].concat();
        match crate::lk_eval(&e, None) {
            Ok(b) => {
                let st = std::str::from_utf8(&b)?;
                Ok(linkspace_common::identity::decrypt(st, password)?)
            }
            Err(_) => {
                if new {
                    use super::key::*;
                    let key = lk_keygen();
                    let enckey = super::key::lk_keystr(&key, password);
                    use std::env::{args, current_dir, current_exe};
                    let notes = format!(
                        "exec:{:?}\ndir:{:?}\nargs:{:?}",
                        current_exe(),
                        current_dir(),
                        args()
                    );
                    linkspace_common::protocols::lns::local::setup_local_key(
                        &linkspace.0,
                        name,
                        &enckey,
                        &notes.as_bytes(),
                    )?;
                    Ok(key)
                } else {
                    anyhow::bail!("no matching entry found")
                }
            }
        }
    }
}

pub use linkspace::{
    lk_get, lk_open, lk_process, lk_process_while, lk_save, lk_stop, lk_watch, Linkspace,
};
pub mod linkspace {
    /**
    The linkspace runtime.

    It connects to a database and a IPC event source
    Open or create with [lk_open].
    This can be called multiple times across threads and processes.
    save with [lk_save] and get packets with [lk_get],
    register callbacks with [lk_watch], and process new data with [lk_process] or [lk_process_while]
    It is possible to nest lk_watch and lk_get calls.
     **/
    #[derive(Clone)]
    #[repr(transparent)]
    pub struct Linkspace(pub(crate) linkspace_common::runtime::Linkspace);

    use tracing::debug_span;

    use super::*;
    /// open a linkspace runtime.
    ///
    /// will look at `path` or $LINKSPACE or '$HOME'
    /// and open 'PATH/linkspace' unless the basename of PATH is linkspacel 'linkspace'
    ///
    /// A runtime is used to [lk_save] and [lk_get] packets.
    /// [lk_watch] also reacts to new packets to saved by this or other processes/thread  when running [lk_process] or [lk_process_while]
    /// You can open the same instance in multiple threads (sharing their db session & ipc ) and across multiple processes.
    /// moving an open linkspace across threads is not supported.
    pub fn lk_open(path: Option<&std::path::Path>, create: bool) -> std::io::Result<Linkspace> {
        let rt = linkspace_common::static_env::open_linkspace_root(path, create)?;
        let mut eval_ctx = crate::abe::ctx::LK_EVAL_CTX_RT.borrow_mut();
        if eval_ctx.is_none() {
            *eval_ctx = Some(rt.clone())
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
        linkspace_common::core::env::write_trait::save_pkt(&mut lk.0.get_writer(), pkt)
    }
    /// [lk_watch] but only for currently indexed packets.
    /// Terminates early when `cb` returns false
    pub fn lk_get_all(
        lk: &Linkspace,
        query: &Query,
        cb: &mut dyn FnMut(&dyn NetPkt) -> bool,
    ) -> LkResult<u32> {
        let mut c = 0;
        let r = lk.0.get_reader();
        let mode = query.0.get_mode()?;
        for p in r.query(mode, &query.0.predicates, &mut c)? {
            let cont = (cb)(&p);
            if !cont {
                break;
            }
        }
        Ok(c)
    }

    /// get a single packet
    pub fn lk_get(lk: &Linkspace, query: &Query) -> LkResult<Option<NetPktBox>> {
        lk_get_ref(lk, query, &mut |v| v.as_netbox())
    }
    /** read a single packet directly from disk.
    This means that [NetPkt::net_header_mut] is unavailable.
    You can wrap it in a [crate::misc::ReroutePkt] to change this or [NetPkt::as_netbox] to allocate and mutate.
    **/
    pub fn lk_get_ref<A>(
        lk: &Linkspace,
        query: &Query,
        cb: &mut dyn FnMut(RecvPktPtr) -> A,
    ) -> LkResult<Option<A>> {
        let mode = query.0.get_mode()?;
        let mut i = 0;
        let reader = lk.0.get_reader();
        let opt_pkt = reader.query(mode, &query.0.predicates, &mut i)?.next();
        Ok(opt_pkt.map(|v| (cb)(v)))
    }
    /**
    watch packets matching the query - both already in the db and new packets on arrival

    Calls `cb` for each matching packet.
    If the `query` contains the watch option ( e.g. ':watch:example' ) the `cb` is also called for all new packets during [[lk_process]] and [[lk_process_while]].
    The watch is dropped when
    - the cb returns 'break' ( usually false )
    - [[lk_stop]] is called with the matching id
    - the predicate set will never match again ( 'i' counters and recv )

    returns the number matches in the local index.
    i.e. the number of times cb was called immediatly.
    **/
    pub fn lk_watch(lk: &Linkspace, query: &Query, cb: impl PktHandler + 'static) -> LkResult<u32> {
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
    ) -> LkResult<u32> {
        Ok(lk.0.watch_query(&query.0, interop::Handler(cb), span)?)
    }
    /// See [lk_watch2]
    pub fn vspan(name: &str) -> tracing::Span {
        tracing::debug_span!("{}", name)
    }

    /// close lk_watch watches based on the watch id ':watch:example' in the query.
    pub fn lk_stop(rt: &Linkspace, id: &[u8], range: bool) {
        if range {
            rt.0.close_range(id)
        } else {
            rt.0.close(id)
        }
    }

    /// process the log of new packets and trigger callbacks
    pub fn lk_process(rt: &Linkspace) -> Stamp {
        rt.0.process()
    }
    /** process the log of new packets continiously

    will return when:
    - max_wait has elapsed between new packets - return false
      e.g. lk_eval("{s:+1M}") or 0u64 to ignore
    - untill time has been reached - returns false
      e.g. lk_eval("{now:+1M}") or 0u64 to ignore
    - no more watch callbacks exists - returns true
    **/
    pub fn lk_process_while(lk: &Linkspace, max_wait: Stamp, untill: Stamp) -> LkResult<bool> {
        let max_wait =
            (max_wait != Stamp::ZERO).then_some(std::time::Duration::from_micros(max_wait.get()));
        let untill = (untill != Stamp::ZERO).then(|| pkt::as_instance(untill).into());
        lk.0.run_while(max_wait, untill)
    }

    pub struct LkInfo<'o> {
        pub path: &'o std::path::Path,
    }
    pub fn lk_info(lk: &Linkspace) -> LkInfo {
        LkInfo {
            path: lk.0.env().location(),
        }
    }
}

/// A set of functions that adhere to conventions
pub mod conventions;
pub use conventions::lk_pull;

pub use consts::{PUBLIC_GROUP,LOCAL_ONLY_GROUP};
pub mod consts {
    pub use linkspace_common::core::consts::pkt_consts::*;
    pub use linkspace_common::core::consts::{
        PUBLIC_GROUP,
        LOCAL_ONLY_GROUP,
        EXCHANGE_DOMAIN
    };
    
}
pub use misc::{cb};
pub mod misc {
    use std::ops::{ControlFlow, Try};

    pub use linkspace_common::core::env::tree_key::TreeEntry;
    pub use linkspace_common::pkt::netpkt::DEFAULT_ROUTING_BITS;
    pub use linkspace_common::pkt::reroute::{RecvPkt, ReroutePkt, ShareArcPkt};
    pub use linkspace_common::pkt_reader as reader;
    pub use linkspace_common::pkt::FieldEnum;
    use linkspace_common::prelude::NetPkt;
    pub use linkspace_common::runtime::handlers::StopReason;

    use crate::{Linkspace, PktHandler, Query};

    pub struct Cb<A,B>{
        pub handle_pkt: A,
        pub stopped:B
    }
    pub fn nop_stopped(_:Query,_:&Linkspace,_:StopReason,_:u32,_:u32){}
    pub fn cb<A,R,E>(mut handle_pkt:A) -> Cb<impl FnMut(&dyn NetPkt, & Linkspace) -> ControlFlow<()>+'static,fn(Query,&Linkspace,StopReason,u32,u32)>
        where
        R: Try<Output = (), Residual = E>,
        E: std::fmt::Debug,
        A: for<'a,'b>FnMut(&'a dyn NetPkt,&'b Linkspace) -> R + 'static,
    {
        Cb{handle_pkt: move |pkt:&dyn NetPkt,lk:&Linkspace| {
            (handle_pkt)(pkt,lk).branch().map_break(|brk| {tracing::info!(?brk,"break")})
        },stopped:nop_stopped}
    }

    impl<A,B> PktHandler for Cb<A,B> where
        A: FnMut(&dyn NetPkt,&Linkspace) -> ControlFlow<()>,
        B: FnMut(Query,&Linkspace,StopReason, u32, u32) // TODO could be FnOnce
        {
        fn handle_pkt(&mut self, pkt: &dyn NetPkt, lk: &Linkspace) -> ControlFlow<()> {
            (self.handle_pkt)(pkt,lk)
        }

        fn stopped(&mut self, query: crate::Query, lk: &Linkspace, reason: StopReason, total: u32, new: u32) {
            (self.stopped)(query,lk,reason,total,new)
        }
    }

}

/// Functions with a custom eval context
pub mod varctx {
    use super::*;
    use crate::abe::ctx::LkCtx;

    pub fn lk_eval(ctx: LkCtx, expr: &str) -> LkResult<Vec<u8>> {
        use linkspace_common::abe::{eval::eval, parse_abe};
        let expr = parse_abe(expr)?;
        let val = eval(&ctx.as_dyn(), &expr)?;
        Ok(val.concat())
    }
    pub fn lk_try_encode(ctx: LkCtx, bytes: &[u8], options: &str) -> LkResult<String> {
        Ok(linkspace_common::abe::eval::encode(
            &ctx.as_dyn(),
            bytes,
            options,
        )?)
    }
    /// custom ctx version of [super::lk_query_parse]
    pub fn lk_query_parse(ctx: LkCtx, pred: &mut Query, expr: &str) -> LkResult<bool> {
        let changed = pred.0.parse(expr.as_bytes(), &ctx.as_dyn())?;
        Ok(changed)
    }
}

// Allow for interop when importing linkspace_common
#[doc(hidden)]
pub mod interop;
