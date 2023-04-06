// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![feature(try_blocks,  thread_local, array_chunks)]
pub mod utils;

use image::{GenericImage, GenericImageWatch, ImageOutputFormat, RgbaImage};
pub use liblinkspace;
use liblinkspace::query::lk_hash_query;
pub use utils::ImgHandle;

use liblinkspace::conventions::LkKeyFlags;
use liblinkspace::{prelude::*, *};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::ControlFlow;
use std::path::Path;
use std::rc::Rc;
const IB: Domain = ab(b"imageboard");
pub const W: u32 = 3508;
pub const H: u32 = 2480;
pub const SW: u32 = 842;
pub const SH: u32 = 595;
/*
Users create a keypoint.
each tag is the 'title' and each ptr is their last read
Their latest indicates what they are engaged in.

*/
const ACTIVITY: IPathC<17> = ipath1(b"activity");
const BOARDS: IPathC<15> = ipath1(b"boards");

pub type Pixel = image::Rgba<u8>;
pub struct Klets {
    pub lk: Linkspace,
    pub key: SigningKey,
    pub group: GroupID,
    pub painter: ImgHandle,
    pub canvas: ImgHandle,
    pub tux: ImgHandle,
    pub state: Rc<RefCell<State>>,
}
#[derive(Default)]
pub struct State {
    pub late_data: Vec<ImgEntry>,
    pub boards: HashMap<String, Rc<RefCell<ImageStream>>>,
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
// Order works because the first fields stamp + link_idx create the order we want
pub struct ImgEntry {
    pub stamp: Stamp,
    pub link_idx: usize,
    pub origin: Ptr,
    pub image_hash: Ptr,
    pub tag: TagInfo,
}
#[derive(Default)]
pub struct ImageStream {
    pub painting: Vec<ImgEntry>,
    pub pending: Vec<ImgEntry>,
}
impl ImageStream {
    pub fn add(&mut self, pkt: &NetPktPtr) {
        let origin = pkt.hash();
        let stamp = *pkt.get_create_stamp();
        let new_entries =
            pkt.get_links()
                .into_iter()
                .enumerate()
                .filter_map(|(link_idx, Link { tag, ptr })| {
                    Some(ImgEntry {
                        tag: TagInfo::try_from(*tag).ok()?,
                        stamp,
                        origin,
                        link_idx,
                        image_hash: *ptr,
                    })
                });
        self.pending.extend(new_entries);
        self.pending.sort() //im lazy
    }
    pub fn process_pending(&mut self) -> &[ImgEntry] {
        let start = match self.pending.first() {
            Some(v) => v,
            None => return &[],
        };
        let pt = match self.painting.binary_search(start) {
            Ok(v) => v,
            Err(v) => v,
        };
        let mut todo = self.painting.split_off(pt);
        todo.extend_from_slice(&std::mem::take(&mut self.pending));
        todo.sort();
        self.painting.extend_from_slice(&todo);
        &self.painting[pt..]
    }
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Place {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Place {
    fn pack(self) -> [u8; 8] {
        let p = [self.x, self.y, self.w, self.h].map(u16::to_be_bytes);
        unsafe { std::mem::transmute(p) }
    }
    fn unpack(v: [u8; 8]) -> Self {
        let v: [[u8; 2]; 4] = unsafe { std::mem::transmute(v) };
        let [x, y, w, h] = v.map(u16::from_be_bytes);
        Place { x, y, w, h }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TagInfo {
    pub place: Place,
    pub rest: AB<[u8; 8]>,
}
impl From<Tag> for TagInfo {
    fn from(value: Tag) -> Self {
        let [place, rest]: [[u8; 8]; 2] = unsafe { std::mem::transmute_copy(&value) };
        TagInfo {
            place: Place::unpack(place),
            rest: AB(rest),
        }
    }
}
impl Into<Tag> for TagInfo {
    fn into(self) -> Tag {
        let mut t = Tag::default();
        t.0[..8].copy_from_slice(&self.place.pack());
        t.0[8..].copy_from_slice(&self.rest.0);
        t
    }
}

pub fn cont() -> ControlFlow<()> {
    ControlFlow::Continue(())
}
impl Klets {
    pub fn init(path: Option<&Path>, group: GroupID, id: &str, password: &str) -> LkResult<Klets> {
        let lk = lk_open(path, true)?;
        let key = lk_key(&lk, &lk_eval(&password, None)?, &id, LkKeyFlags::NEW)?;
        let tux = image::io::Reader::open("../../imageboard/tux.png")
            .unwrap()
            .decode()
            .unwrap()
            .into_rgba8()
            .into();
        let canvas: ImgHandle = RgbaImage::new(W, H).into();
        let painter = canvas.clone();
        let state = Rc::new(RefCell::new(State {
            late_data: vec![],
            boards: Default::default(),
        }));
        let klets = Klets {
            painter,
            canvas,
            lk,
            key,
            state,
            group,
            tux,
        };
        let pt = lk_keypoint(
            IB,
            klets.group,
            &ACTIVITY,
            &[],
            &[],
            Some(0.into()),
            &klets.key,
        )
        .unwrap();
        lk_save(&klets.lk, &*pt).unwrap();
        //        klets.watch_key(klets.key.pubkey());
        Ok(klets)
    }
    pub fn place_image(&self, board: &str, place: Place) -> LkResult<()> {
        let mut buf = [0u8; MAX_DATA_SIZE];
        let img = self
            .painter
            .watch(
                place.x as u32,
                place.y as u32,
                place.w as u32,
                place.h as u32,
            )
            .to_image();
        let len = {
            let mut c = std::io::Cursor::new(&mut buf as &mut [u8]);
            img.write_to(&mut c, ImageOutputFormat::Png)?;
            c.position()
        };
        let dp = lk_datapoint(&buf[..len as usize])?;
        let links = [Link {
            tag: TagInfo {
                place,
                rest: ab(b"png"),
            }
            .into(),
            ptr: dp.hash(),
        }];
        let lp = lk_linkpoint(
            IB,
            self.group,
            &BOARDS.as_ref().to_owned().append(board.as_bytes()),
            &links,
            &[],
            None,
        )?;
        lk_save(&self.lk, &dp)?;
        lk_save(&self.lk, &lp)?;
        Ok(())
    }
    pub fn load_board(&self, board: &str) -> LkResult<Rc<RefCell<ImageStream>>> {
        if let Some(v) = self.state.borrow_mut().boards.get(board) {
            return Ok(v.clone());
        }
        let querystr = [
            "domain:=:imageboard\n",
            "group:=:{#:pub}\n",
            //"i_index:<:{u32:1}\n", // only the latest is enough
            "prefix:=:",
            &BOARDS.into_ipathbuf().append(board.as_bytes()).to_string(),
            "\n",
        ]
        .concat();
        let query = lk_query(&querystr)?;
        let board = self
            .state
            .borrow_mut()
            .boards
            .entry(board.to_owned())
            .or_default()
            .clone();
        let board2 = board.clone();
        lk_watch(&self.lk, query, move |pkt: PktSlot, _: &Linkspace| {
            board.borrow_mut().add(&pkt);
            cont()
        })?;
        Ok(board2)
    }
    pub fn load_entry(&self, e: &ImgEntry) -> LkResult<Option<RgbaImage>> {
        let query = lk_hash_query(e.image_hash);
        match lk_get(&self.lk, &query) {
            Ok(Some(p)) => {
                let img = image::load_from_memory(p.data())?.into_rgba8().into();
                Ok(Some(img))
            }
            _ => {
                todo!()
                // lk_pull(&self.lk, query, ttl)
            }
        }
    }

    pub fn clear_painter(&mut self) -> &RgbaImage {
        *self.painter = RgbaImage::new(W, H);
        &self.painter
    }
    pub fn painter_example(&mut self) -> &RgbaImage {
        imageproc::drawing::draw_cross_mut(&mut *self.painter, [255, 0, 0, 255].into(), 30, 30);
        self.painter.copy_from(&*self.tux, 50, 50).unwrap();
        &self.painter
    }
    pub fn save_painter(&self, board: &str) -> LkResult<()> {
        let place = find_bb(&self.painter);
        self.place_image(board, place)
    }
}

pub fn find_bb(img: &RgbaImage) -> Place {
    let mut minx = u16::try_from(img.width()).unwrap();
    let mut miny = u16::try_from(img.height()).unwrap();
    let (mut maxx, mut maxy) = (0u16, 0u16);
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] > 3 {
            maxx = maxx.max(x as u16);
            maxy = maxy.max(y as u16);
            minx = minx.min(x as u16);
            miny = miny.min(y as u16);
        }
    }
    Place {
        x: minx,
        y: miny,
        w: maxx.saturating_sub(minx),
        h: maxy.saturating_sub(miny),
    }
}
