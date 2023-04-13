// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::app::*;
use egui::{vec2, CentralPanel, Image, Rect, SidePanel, TextureHandle};
use imageboard_rs::{linkspace::prelude::Ptr, ImageStream, Place};

pub struct Body {
    pub show_boards_panel: bool,
    pub show_event_panel: bool,
    pub active_board: String,
    pub loaded_boards: HashMap<String, ImgBoard>,
    pub textures: HashMap<Ptr, TextureHandle>,
    pub painter: TextureHandle,
    pub place: Place,
}

pub struct ImgBoard {
    state: Rc<RefCell<ImageStream>>,
}
impl Body {
    pub fn new(painter: TextureHandle) -> Self {
        Body {
            textures: HashMap::new(),
            painter,
            place: Place::default(),
            show_boards_panel: true,
            show_event_panel: true,
            active_board: "hello".into(),
            loaded_boards: HashMap::new(),
        }
    }
}

impl State for Body {
    fn update(&mut self, common: &mut Common, ctx: &egui::Context, frame: &mut eframe::Frame) {
        common.painter.set(&mut self.painter);
        let board = self
            .loaded_boards
            .entry(self.active_board.clone())
            .or_insert_with(|| ImgBoard {
                state: common.klets.load_board(&self.active_board).unwrap(),
            });
        {
            let st = std::mem::take(&mut common.klets.state.borrow_mut().late_data);
            let mut imglst = board.state.borrow_mut();
            let new = imglst.process_pending();
            for e in st.iter().chain(new.iter()) {
                if let Some(Some(img)) = common.init.err(common.klets.load_entry(e)) {
                    let texture = ctx.load_texture(
                        e.image_hash.to_string(),
                        as_img(&img),
                        Default::default(),
                    );
                    self.textures.insert(e.image_hash, texture);
                }
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.close();
                    }
                });

                ui.toggle_value(&mut self.show_boards_panel, "board");
                ui.toggle_value(&mut self.show_event_panel, "event");
                ui.menu_button("about", |ui| {
                    eprintln!("{:?}", common.init);
                    ui.label(&common.init.id);
                    ui.label(&common.init.path);
                })
            });
        });
        SidePanel::left("events").show_animated(ctx, self.show_event_panel, |_ui| {});
        CentralPanel::default().show(ctx, |ui| {
            if ui.small_button("clear").clicked() {
                common.klets.clear_painter().set(&mut self.painter);
            }
            if ui.small_button("test").clicked() {
                common.klets.painter_example().set(&mut self.painter);
            }
            if ui.small_button("bb").clicked() {
                let s = common.klets.save_painter(&self.active_board);
                common.init.err(s);
                common.klets.clear_painter();
            }

            for i in board.state.borrow().painting.iter() {
                if let Some(tex) = self.textures.get(&i.image_hash) {
                    let Place { x, y, w, h } = i.tag.place;
                    let rect = Rect::from_min_size(
                        [x as f32, y as f32].into(),
                        [w as f32, h as f32].into(),
                    );
                    Image::new(tex.id(), rect.size()).paint_at(ui, rect);
                }
            }

            ui.image(&self.painter, SMALL);
        });
    }
}
