#![warn(clippy::all, rust_2018_idioms)]

// this is annoying that i have to put it before everyone...
#[cfg(not(target_arch = "wasm32"))]
mod app;
#[cfg(not(target_arch = "wasm32"))]
mod channel_data;
#[cfg(not(target_arch = "wasm32"))]
mod channel_map;
#[cfg(not(target_arch = "wasm32"))]
mod compass_data;
#[cfg(not(target_arch = "wasm32"))]
mod compass_file;
#[cfg(not(target_arch = "wasm32"))]
mod compass_run;
#[cfg(not(target_arch = "wasm32"))]
mod error;
#[cfg(not(target_arch = "wasm32"))]
mod event_builder;
#[cfg(not(target_arch = "wasm32"))]
mod kinematics;
#[cfg(not(target_arch = "wasm32"))]
mod nuclear_data;
#[cfg(not(target_arch = "wasm32"))]
mod scaler_list;
#[cfg(not(target_arch = "wasm32"))]
mod shift_map;
#[cfg(not(target_arch = "wasm32"))]
mod used_size;
#[cfg(not(target_arch = "wasm32"))]
mod ws;
#[cfg(not(target_arch = "wasm32"))]
pub use app::EVBApp;

#[cfg(target_arch = "wasm32")]
mod app_web;
#[cfg(target_arch = "wasm32")]
pub use app_web::EVBApp;
