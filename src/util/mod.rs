#[cfg(not(target_arch = "wasm32"))]
pub mod lazyframer;
#[cfg(not(target_arch = "wasm32"))]
pub mod processer;
#[cfg(not(target_arch = "wasm32"))]
pub mod workspacer;
