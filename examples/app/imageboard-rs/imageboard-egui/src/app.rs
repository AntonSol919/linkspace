// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::{body::Body, themes};
use egui::{ColorImage, TextEdit, TextureHandle, Vec2, Widget};
use image::RgbaImage;
use imageboard_rs::Klets;
use imageboard_rs::{
    liblinkspace::{self as lk, lk_encode, lk_process, linkspace::lk_info},
    ImgHandle,
};
use lk::prelude::*;
use std::{fmt::Debug, ops::Deref, path::Path};

//const _SIZE  : Vec2 = Vec2{x : imageboard_rs::W as f32, y: imageboard_rs::H as f32};
pub const SMALL: Vec2 = Vec2 {
    x: imageboard_rs::SW as f32,
    y: imageboard_rs::SH as f32,
};
pub trait State {
    fn update(&mut self, common: &mut Common, ctx: &egui::Context, _frame: &mut eframe::Frame);
}

pub struct KletsApp {
    pub state: Vec<Box<dyn State>>,
    pub common: Result<Common, Init>,
}
#[derive(Default, Debug)]
pub struct Init {
    pub path: String,
    pub init_linkspace: bool,
    pub id: String,
    pass: String,
    keygen: bool,
    pub error: String,
}
impl Init {
    pub fn err<V, T: Debug>(&mut self, r: Result<V, T>) -> Option<V> {
        match r {
            Ok(v) => Some(v),
            Err(e) => {
                self.error = format!("{:#?}", e);
                eprintln!("{}", self.error);
                None
            }
        }
    }
}

pub struct Common {
    pub drop_state: bool,
    pub push_state: Vec<Box<dyn State>>,
    pub klets: Klets,
    #[allow(dead_code)]
    pub init: Init,
}
impl Deref for Common {
    type Target = Klets;

    fn deref(&self) -> &Self::Target {
        &self.klets
    }
}
impl KletsApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_pixels_per_point(4.0);
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        themes::set_theme(&cc.egui_ctx, themes::FRAPPE);
        KletsApp {
            state: vec![],
            common: Err(Init::default()),
        }
    }
}

impl eframe::App for KletsApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        fn err(err: &mut String, ctx: &egui::Context) {
            if err.is_empty() {
                return;
            }
            egui::TopBottomPanel::bottom("error").show(ctx, |ui| {
                if ui.selectable_label(false, format!("Error {err}")).clicked() {
                    err.clear();
                }
            });
        }
        let KletsApp { state, common } = self;
        *common = match std::mem::replace(common, Err(Init::default())) {
            Ok(mut c) => {
                lk_process(&c.klets.lk);
                err(&mut c.init.error, ctx);
                let old_state = std::mem::take(state);
                for mut s in old_state {
                    s.update(&mut c, ctx, frame);
                    if !std::mem::take(&mut c.drop_state) {
                        state.push(s);
                    }
                    state.extend(c.push_state.drain(..));
                }

                Ok(c)
            }
            Err(mut init) => {
                egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                    // The top panel is often a good place for a menu bar:
                    egui::menu::bar(ui, |ui| {
                        ui.menu_button("File", |ui| {
                            if ui.button("Quit").clicked() {
                                frame.close();
                            }
                        });
                    });
                });
                egui::CentralPanel::default()
                    .show(ctx, |ui| {
                        let Init {
                            path,
                            init_linkspace,
                            id,
                            pass,
                            keygen,
                            error,
                        } = &mut init;
                        err(error, ctx);
                        ui.label("Path");
                        TextEdit::singleline(path)
                            .hint_text("$HOME/linkspace")
                            .ui(ui);
                        ui.toggle_value(init_linkspace, "create");
                        ui.label("id");
                        TextEdit::singleline(path).hint_text("me").ui(ui);
                        ui.label("password");
                        ui.text_edit_singleline(pass);
                        ui.toggle_value(keygen, "keygen");
                        if true || ui.button("Start").clicked() {
                            let path = if path.is_empty() {
                                None
                            } else {
                                Some(Path::new(&path))
                            };
                            match Klets::init(path, PUBLIC, id, pass) {
                                Ok(klets) => {
                                    *error = String::new();
                                    let painter = ctx.load_texture(
                                        "\tpainter",
                                        as_img(&klets.painter),
                                        Default::default(),
                                    );
                                    state.push(Box::new(Body::new(painter)));
                                    init.path = lk_info(&klets.lk).path.to_owned();
                                    init.id =
                                        lk_encode(&*klets.key.pubkey(), "@/@local/#/#local/b");
                                    return Ok(Common {
                                        init,
                                        klets,
                                        drop_state: false,
                                        push_state: vec![],
                                    });
                                }
                                Err(err) => *error = format!("{err:#?}"),
                            }
                        }
                        Err(init)
                    })
                    .inner
            }
        };
    }
}

pub fn as_img(image: &RgbaImage) -> ColorImage {
    let size = [image.width() as _, image.height() as _];
    let pixels = image.as_flat_samples();
    egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice())
}
pub trait InTexture {
    fn set(&self, handle: &mut TextureHandle);
}
impl InTexture for RgbaImage {
    fn set(&self, handle: &mut TextureHandle) {
        handle.set(as_img(self), Default::default())
    }
}
impl InTexture for ImgHandle {
    fn set(&self, handle: &mut TextureHandle) {
        if let Some(v) = self.take_update() {
            v.set(handle)
        }
    }
}
