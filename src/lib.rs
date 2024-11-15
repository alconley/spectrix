#![warn(clippy::all, rust_2018_idioms)]

#[cfg(not(target_arch = "wasm32"))]
pub mod egui_plot_stuff;
#[cfg(not(target_arch = "wasm32"))]
pub mod fitter;
#[cfg(not(target_arch = "wasm32"))]
pub mod histoer;
#[cfg(not(target_arch = "wasm32"))]
pub mod histogram_scripter;
#[cfg(not(target_arch = "wasm32"))]
pub mod util;

pub mod ui;
