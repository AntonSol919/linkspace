// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{cell::Cell, ops::DerefMut};

use image::RgbaImage;

#[derive(Clone)]
pub struct ImgHandle {
    image: RgbaImage,
    updated: Cell<bool>,
}
impl From<RgbaImage> for ImgHandle {
    fn from(image: RgbaImage) -> Self {
        ImgHandle {
            image,
            updated: Cell::new(true),
        }
    }
}
impl ImgHandle {
    pub fn take_update(&self) -> Option<&RgbaImage> {
        self.updated.take().then_some(&self.image)
    }
}
use std::ops::Deref;
impl Deref for ImgHandle {
    type Target = RgbaImage;
    fn deref(&self) -> &RgbaImage {
        &self.image
    }
}
impl DerefMut for ImgHandle {
    fn deref_mut(&mut self) -> &mut RgbaImage {
        self.updated.set(true);
        &mut self.image
    }
}
