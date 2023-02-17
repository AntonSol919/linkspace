// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
pub mod app;
pub mod body;
mod themes;
fn main() {
    let mut v = std::env::var_os("RUST_LOG").unwrap_or("".into());
    v.push("egui=warn,eframe=warn");
    std::env::set_var("RUST_LOG", &v);
    tracing_subscriber::fmt::init();

    let mut native_options = eframe::NativeOptions::default();
    native_options.initial_window_size = Some([1000.0, 1000.0].into());
    eframe::run_native(
        "klets egui",
        native_options,
        Box::new(|cc| Box::new(app::KletsApp::new(cc))),
    );
}
