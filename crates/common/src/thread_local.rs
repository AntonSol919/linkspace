use std::cell::OnceCell;
use linkspace_pkt::{GroupID, Domain};
use linkspace_pkt::PUBLIC;
use byte_fmt::ab;




#[thread_local]
static GROUP: OnceCell<GroupID> = OnceCell::new();
pub fn set_group(group: GroupID) {
    assert_eq!(
        *GROUP.get_or_init(|| group),
        group,
        "user bug: the default group can only be set once per thread"
    );
}
/** [Thread Local]: get the 'default' group. from [set_group] || $LK_GROUP || `[#:pub]`

If the LK_GROUP expression requires LNS evaluation this will use the thread local linkspace or open the default.
**/
#[cfg(feature="runtime")]
pub fn group() -> GroupID {
    use std::env::*;

    use linkspace_pkt::GroupExpr;
    use crate::{eval::lk_scope, static_env::{LINKSPACE, open_linkspace_dir}};
    *GROUP.get_or_init(|| match std::env::var("LK_GROUP") {
        Err(VarError::NotPresent) => PUBLIC,
        Ok(o) => {
            let expr: GroupExpr = o.parse().expect("cant parse LK_GROUP");
            let scope = lk_scope(
                || {
                    if let Some(o) = LINKSPACE.get() {
                        return Ok(o.clone());
                    }
                    tracing::info!("opening default linkspace to read evaluate LK_GROUP variable");
                    Ok(open_linkspace_dir(None, false)?)
                },
                true,
            );
            expr.eval(&scope).expect("can't eval LK_GROUP")
        }
        _ => panic!("can't read LK_DOMAIN as utf8"),
    })
}
#[cfg(not(feature="runtime"))]
pub fn group() -> GroupID { *GROUP.get_or_init(||PUBLIC)}

#[thread_local]
static DOMAIN: OnceCell<Domain> = OnceCell::new();

/// set the result for [domain]
pub fn set_domain(domain: Domain) {
    assert_eq!(
        *DOMAIN.get_or_init(|| domain),
        domain,
        "user bug: the standard domain can only be set once per thread"
    );
}
/** [Thread Local]: get the 'default' domain. from [set_domain] || $LK_DOMAIN || `[0;16]`

If the LK_DOMAIN expression requires LNS evaluation this will use the thread local linkspace or open the default.
**/
#[cfg(feature="runtime")]
pub fn domain() -> Domain {
    use std::env::*;
    use linkspace_pkt::DomainExpr;

    use crate::{ static_env::{LINKSPACE, open_linkspace_dir}, eval::lk_scope};
    *DOMAIN.get_or_init(|| match std::env::var("LK_DOMAIN") {
        Err(VarError::NotPresent) => ab(b""),
        Ok(o) => {
            let expr: DomainExpr = o.parse().expect("cant parse LK_DOMAIN");
            let scope = lk_scope(
                || {
                    if let Some(o) = LINKSPACE.get() {
                        return Ok(o.clone());
                    }
                    tracing::info!("opening default linkspace to read evaluate LK_DOMAIN variable");
                    Ok(open_linkspace_dir(None, false)?)
                },
                true,
            );
            expr.eval(&scope).expect("can't eval LK_DOMAIN")
        }
        _ => panic!("can't read LK_DOMAIN as utf8"),
    })
}
#[cfg(not(feature="runtime"))]
pub fn domain() -> Domain { *DOMAIN.get_or_init(||ab(b""))}
