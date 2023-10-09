use std::fmt::{Debug, Display};

use anyhow::Context;

use crate::{ast::Ctr, eval::ABList};

#[derive(Default, Clone)]
/** A generic serialize-deserialize format for an ablist.
e.g.
```text
hello:world/thing
:some:other:options:\f\f\f
/ok
```

Options grow from bottom to top. i.e. reading/selecting starts from the latest added entry.

**/
pub struct ABConf(Vec<ABList>);

pub fn parse_ablist_b(st: &[u8]) -> anyhow::Result<crate::eval::ABList> {
    let abe = crate::ast::parse_abe_strict_b(st)?;
    abe.as_slice()
        .try_into()
        .map_err(|e| anyhow::anyhow!("expr not supported - {e}"))
}

impl ABConf {
    pub const DEFAULT: Self = ABConf(vec![]);

    pub const fn new(values: Vec<ABList>) -> Self {
        ABConf(values)
    }
    pub fn extend(&mut self, tail: Vec<ABList>) {
        self.0.extend(tail)
    }
    pub fn push(&mut self, entry: ABList) {
        self.0.push(entry)
    }
    pub fn get(&self, b: &[u8]) -> Option<Result<&ABList, &ABList>> {
        self.get_checked(&[b]).next()
    }

    /** used to get the first match and return its optional value.
    e.g. get_value(S0:S1:?val)

    S0:S1 => Some(None)
    S0:S1:val => Some(val)

    S0 => next
    S0:S1/ => Err
    S0:S1:val: => Err

    */
    pub fn has_optional_value(
        &self,
        starts_with: &[&[u8]],
    ) -> Option<Result<Option<&[u8]>, &ABList>> {
        match &self.get_checked(starts_with).next()? {
            Ok(lst) => match &lst[starts_with.len() - 1..] {
                [(Some(Ctr::Colon), bytes)] => Some(Ok(Some(bytes.as_slice()))),
                [] => Some(Ok(None)),
                _ => Some(Err(lst)),
            },
            Err(v) => Some(Err(v)),
        }
    }
    /// Iterate over all matches starting with S0:S1:S2:val. Returns Err() if starts_with contains '/'
    pub fn get_checked<'a: 'b, 'b>(
        &'a self,
        starts_with: &'b [&[u8]],
    ) -> impl Iterator<Item = Result<&'a ABList, &'a ABList>> + 'b {
        self.get_inner(starts_with).map(|o| {
            if o.iter()
                .take(starts_with.len())
                .all(|c| matches!(c.0, None | Some(Ctr::Colon)))
            {
                Ok(o)
            } else {
                Err(o)
            }
        })
    }
    pub fn get_inner<'a: 'b, 'b>(
        &'a self,
        starts_with: &'b [&[u8]],
    ) -> impl Iterator<Item = &'a ABList> + 'b {
        self.0.iter().rev().filter(move |a| {
            if a.len() < starts_with.len() {
                return false;
            }
            let ok = starts_with
                .iter()
                .zip(a.iter_bytes())
                .all(|(b, opt)| b == &opt);
            ok
        })
    }
    pub fn try_from(mut inp: &[u8]) -> anyhow::Result<Self> {
        if let Some(r) = inp.strip_prefix(b":%") {
            inp = r.strip_prefix(b"abctxt\n").context("unknown conf format")?;
        }
        let mut abc: Vec<_> = inp
            .split(|c| *c == b'\n')
            .map(|v| parse_ablist_b(v).map_err(|v| anyhow::anyhow!("data contains expr {v:?}")))
            .try_collect()?;
        abc.reverse();
        Ok(ABConf(abc))
    }
}
impl Display for ABConf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for abl in self.0.iter().rev() {
            writeln!(f, "{abl}")?;
        }
        Ok(())
    }
}
impl Debug for ABConf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ABConf")
            .field("reversed_list", &self.0)
            .finish()
    }
}
