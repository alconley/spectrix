#![warn(clippy::all, rust_2018_idioms)]

#[cfg(not(target_arch = "wasm32"))]
mod app;
#[cfg(not(target_arch = "wasm32"))]
pub mod cutter;
#[cfg(not(target_arch = "wasm32"))]
mod egui_plot_stuff;
#[cfg(not(target_arch = "wasm32"))]
mod fitter;
#[cfg(not(target_arch = "wasm32"))]
pub mod histoer;
#[cfg(not(target_arch = "wasm32"))]
mod lazyframer;
#[cfg(not(target_arch = "wasm32"))]
pub mod processer;
#[cfg(not(target_arch = "wasm32"))]
pub mod workspacer;
#[cfg(not(target_arch = "wasm32"))]
pub use app::NATApp;

#[cfg(not(target_arch = "wasm32"))]
pub mod pane;
#[cfg(not(target_arch = "wasm32"))]
pub mod tree;

#[cfg(target_arch = "wasm32")]
mod app_web;
#[cfg(target_arch = "wasm32")]
pub use app_web::NATApp;
