// SPDX-FileCopyrightText: 2026 Kerstin Humm <kerstin@erictapen.name>
//
// SPDX-License-Identifier: GPL-3.0-or-later

mod app;
pub use app::App;

mod german_sepa;
mod utils;
mod widgets;

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    std::thread::spawn(move || futures::executor::block_on(f));
}
#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}
