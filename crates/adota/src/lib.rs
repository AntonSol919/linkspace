// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![feature(try_trait_v2, control_flow_enum, exit_status_error, write_all_vectored)]
/**
adoto - and-dot-or
A file/folder logical expression format.
# Example 1

./hello
./world.one
./world.two

adoto = hello && ( world.one || world.two )

#Example 2
Nesting works as expected
If we add:
./subdir/f1
./subdir/file.opt1
./subdir/file.opt1
adoto = hello && (world.one || world.two ) && ( subdir/f1 && ( subdir/file.opt1 || subdir/file.opt2 ) )

#Example 3
You can combine options and subdirs
./world.one
./world.two/f2
./world.two/f3
adoto = (world.one || (world.two/f2 && world.two/f3)

#example 4
If you use the same prefix for an AND and OR they are not combined
./world
./world.one
./world.two
adoto = world && ( world.one || world.two)


JSON encoding works as follows.
Of note is that the OR constructs are still just encoded as '.'

Or they can be nested
{
 "hello": VAL
 "world.one": VAL,
 "world.two":VAL
 "dir" : {
   "this":VAL,
   "that":VAL
   "either.0":VAL,
   "either.1":VAL
 }
}


**/
use std::{collections::HashMap, fmt::Display, ops::Try, path::PathBuf, rc::Rc};

use anyhow::Context;
use pretty::RcDoc;
use serde::*;

pub type ID = Rc<[String]>;
/// optimized for evaluation
#[derive(Debug)]
pub enum Expr<T> {
    And(Vec<Expr<T>>),
    Or(Vec<Expr<T>>),
    Val { id: ID, val: T },
}

pub type ObjTree<T> = HashMap<String, Node<T>>;
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Node<T> {
    Val(T),
    Map(ObjTree<T>),
}
impl<T> Default for Node<T> {
    fn default() -> Self {
        Node::Map(HashMap::new())
    }
}
fn insert<T>(path: &[String], v: &mut Node<T>, val: T) {
    match path.split_first() {
        Some((h, tail)) => {
            if let Node::Map(m) = v {
                let ptr = m.entry(h.clone()).or_default();
                insert(tail, ptr, val)
            } else {
                panic!()
            }
        }
        None => *v = Node::Val(val),
    }
}

impl<T> Expr<T> {
    pub fn serdify(self) -> Node<T> {
        let mut map = Node::default();
        self.consume(&mut |id, val| {
            insert(&*id, &mut map, val);
        });
        map
    }
    pub fn to_doc(&self) -> RcDoc<()>
    where
        T: Display,
    {
        match self {
            Expr::And(lst) => RcDoc::text("(")
                .append(
                    RcDoc::intersperse(
                        lst.iter().map(Self::to_doc),
                        RcDoc::softline().append(" && "),
                    )
                    .group()
                    .nest(2),
                )
                .append(RcDoc::softline().append(")")),
            Expr::Or(lst) => RcDoc::text("(")
                .append(
                    RcDoc::intersperse(
                        lst.iter().map(Self::to_doc),
                        RcDoc::softline().append(" || "),
                    )
                    .group()
                    .nest(2),
                )
                .append(RcDoc::softline().append(")")),
            Expr::Val { id: _, ref val } => RcDoc::text(val.to_string()),
        }
    }
    pub fn as_ref(&self) -> Expr<&T> {
        match self {
            Expr::And(v) => Expr::And(v.iter().map(Self::as_ref).collect()),
            Expr::Or(v) => Expr::Or(v.iter().map(Self::as_ref).collect()),
            Expr::Val { id, ref val } => Expr::Val {
                id: id.clone(),
                val,
            },
        }
    }
    pub fn consume(self, func: &mut impl FnMut(ID, T)) {
        match self {
            Expr::Val { id, val } => func(id, val),
            Expr::And(lst) => lst.into_iter().for_each(|v| v.consume(func)),
            Expr::Or(lst) => lst.into_iter().for_each(|v| v.consume(func)),
        }
    }
    pub fn map<X>(self, func: &mut impl FnMut(&ID, T) -> X) -> Expr<X> {
        match self {
            Expr::Val { id, val } => Expr::Val {
                id: id.clone(),
                val: func(&id, val),
            },
            Expr::And(lst) => Expr::And(lst.into_iter().map(|v| v.map(func)).collect()),
            Expr::Or(lst) => Expr::Or(lst.into_iter().map(|v| v.map(func)).collect()),
        }
    }
    pub fn simplify(self) -> Option<Self> {
        match self {
            Expr::And(lst) => {
                let mut lst: Vec<Self> = lst.into_iter().filter_map(Self::simplify).collect();
                if lst.len() > 1 {
                    Some(Expr::And(lst))
                } else {
                    lst.pop()
                }
            }
            Expr::Or(lst) => {
                let mut lst: Vec<Self> = lst.into_iter().filter_map(Self::simplify).collect();
                if lst.len() > 1 {
                    Some(Expr::Or(lst))
                } else {
                    lst.pop()
                }
            }
            Expr::Val { id, val } => Some(Expr::Val { id, val }),
        }
    }
    pub fn eval<R, O>(self, func: &mut impl FnMut(&ID, T) -> R) -> Option<Expr<O>>
    where
        R: Try<Output = O>,
    {
        match self {
            Expr::And(lst) => lst
                .into_iter()
                .map(|e| e.eval(func))
                .collect::<Option<Vec<Expr<_>>>>()
                .map(Expr::And),
            Expr::Or(lst) => lst
                .into_iter()
                .flat_map(|e| e.eval(func))
                .next()
                .map(|or| Expr::Or(vec![or])),
            Expr::Val { id, val } => func(&id, val)
                .branch()
                .continue_value()
                .map(|val| Expr::Val { id, val }),
        }
    }
}

impl Expr<PathBuf> {
    pub fn read_dir(path: PathBuf) -> anyhow::Result<Self> {
        Self::read_dir_(path, &mut vec![])
    }
    pub fn read_dir_(path: PathBuf, at: &mut Vec<String>) -> anyhow::Result<Self> {
        if path.is_file() {
            return Ok(Expr::Val {
                id: Rc::from(at.clone()),
                val: path,
            });
        }
        let mut map: HashMap<String, Vec<Self>> = HashMap::new();
        for el in path.read_dir()? {
            let e = el?;
            let filename = e.file_name();
            let name = filename.to_str().context("invalid str")?;
            at.push(name.to_string());
            let mut it = name.split(".");
            let and = it.next().context("Bad filename")?;
            map.entry(and.to_owned())
                .or_default()
                .push(Self::read_dir_(e.path(), at)?);
            at.pop();
        }
        let lst = map.into_values().map(Expr::Or).collect();
        Ok(Expr::And(lst))
    }
}
