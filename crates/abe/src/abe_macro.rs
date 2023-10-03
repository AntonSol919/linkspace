// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#[macro_export]
macro_rules! abe_item {
    () => {
        core::iter::empty()
    };
    ( : ) => {
        core::iter::once($crate::ast::ABE::Ctr($crate::ast::Ctr::Colon))
    };
    ( / ) => {
        core::iter::once($crate::ast::ABE::Ctr($crate::ast::Ctr::FSlash))
    };
    // ( $e:ident ) => { ABE::Expr(stringify!($e).into())};
    //( $e:expr ) => { core::iter::once($crate::abe::ABE::Expr( $crate::abe::Expr::from($e)))};
    ( $e:expr ) => {
        core::iter::once($crate::ast::ABE::Expr($e.into()))
    };
}
#[macro_export]
macro_rules! abe {
    ( ) => { core::iter::empty::<$crate::ast::ABE>() };
    ( <$t:ty> $($arg:tt)*) => {
        $crate::TypedABE::<$t>::from($crate::abe!($($arg)*).collect()).unwrap()
    };
    ( +( $($inner:tt)* ) $($arg:tt)*) => {
        $($inner)*.into_iter().chain( $crate::abe!($($arg)*))
    };
    ( { $($inner:tt)* } $($arg:tt)*) => {
        core::iter::once($crate::ast::ABE::from_iter($crate::abe!($($inner)*))).chain( $crate::abe!($($arg)*))
    };
    ( $a:tt $($arg:tt)*) => {
        $crate::abe_item!($a).chain( $crate::abe!($($arg)*))
    };
}
#[macro_export]
macro_rules! abev {
    ( $($arg:tt)*) => {
        $crate::abe!($($arg)*).collect::<Vec<$crate::ast::ABE>>()
    };
}

#[test]
fn test() {
    use crate::ast::*;
    let ext: Vec<ABE> = abe!("OK").collect();
    let _v: ABE = abe!( "#" : / : {/} :).collect();

    let _ok = abe!(<Vec<u8>> "prefix" : "=" : );

    let opt1: ABE = abe!( "#" : / : {/} :)
        .chain(ext.clone())
        .collect();
    let v: ABE = abe!( "#" : / : {/} : +(ext.into_iter())).collect();
    assert_eq!(opt1, v);
}
