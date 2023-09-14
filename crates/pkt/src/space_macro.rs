// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#[macro_export]
macro_rules! sp_iter_match {
	  ($e:expr,[]) => {
		    if $e.next().is_none(){
            ()
        }else {
            None?
        }
	  };
	  ($e:expr,[ .. ]) => {
		    $e.space()
	  };
    ($e:expr,[ $s:ty]) => {
        {
            if let Some(v) = $e.next(){
                if $e.next().is_some() { None?}
                (<$s>::try_from(v).ok()?)
            }else {
                None?
            }
        }
    };
	  ($e:expr,[ $s:ty, $($tail:tt),*]) => {
        {
            if let Some(v) = $e.next(){
                (<$s>::try_from(v).ok()? , sp_iter_match!($e,[$($tail),*]))
            }else {
                None?
            }
        }
    };
    ($e:expr,[ $s:expr ]) => {
        {
            if let Some(v) = $e.next(){
                if $e.next().is_some() { None?}
                if v == AsRef::<[u8]>::as_ref($s){
                    ()
                }else { None?}
            }else { None?}
        }
	  };
	  ($e:expr,[ $s:expr , $($tail:tt),*]) => {
        {
            if let Some(v) = $e.next(){
                if v == AsRef::<[u8]>::as_ref($s){
                    sp_iter_match!($e,[$($tail),*])
                }else { None? }
            }else { None?}
        }
	  };
}

#[macro_export]
macro_rules! sp_match {
	  ($e:expr, [ $($tail:tt),*]) => {
        {
            let r : Option<_> = try {
                let mut iter = $e.iter();
                sp_iter_match!(iter,[$($tail),*])
            };
            r
        }
	  };
}
#[test]
fn test() {
    use crate::*;

    let buf = space_buf(&[b"hello", b"world"]);
    let v: Option<()> = sp_match!(buf, [b"hello", b"world"]);
    assert!(v.is_some());

    let v: Option<()> = sp_match!(buf, [b"hell2o", b"world"]);
    assert!(v.is_none());

    let v: Option<([u8; 5], ())> = sp_match!(buf, [[u8; 5], b"world"]);
    assert_eq!(v, Some((*b"hello", ())));

    let v: Option<[u8; 5]> = sp_match!(buf, [[u8; 5]]);
    assert!(v.is_none());
    let v: Option<([u8; 5], ())> = sp_match!(buf, [[u8; 5], b"world", b"test"]);
    assert!(v.is_none());

    let v: Option<([u8; 5], &Space)> = sp_match!(buf, [[u8; 5], ..]);
    let sp = v.unwrap().1;
    assert!(sp_match!(sp, [b"world"]).is_some());

    let v: Option<([u8; 5], [u8; 4])> = sp_match!(buf, [[u8; 5], [u8; 4]]);
    assert!(v.is_none());

    let v: Option<([u8; 5], [u8; 5])> = sp_match!(buf, [[u8; 5], [u8; 5]]);
    assert!(v.is_some());
}
