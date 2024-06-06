#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use muc::MUCApp;

use eframe::egui;
// fn main() -> Result<(), eframe::Error> {
//     env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
//     let options = eframe::NativeOptions {
//         viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
//         ..Default::default()
//     };
//     eframe::run_native(
//         "muc",
//         options,
//         Box::new(|cc| {
//             #[cfg_attr(not(feature = "serde"), allow(unused_mut))]
//             let mut app = MUCApp::new(cc);
//             #[cfg(feature = "serde")]
//             if let Some(storage) = _cc.storage {
//                 if let Some(state) = eframe::get_value(storage, eframe::APP_KEY) {
//                     app = state;
//                 }
//             }
//             Box::new(app)
//         }),
//     )
// }

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                // NOTE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "muc",
        native_options,
        Box::new(|cc| Box::new(MUCApp::new(cc))),
    )
}


// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|cc| Box::new(muc::MUCApp::new(cc, false))),
            )
            .await
            .expect("failed to start eframe");
    });
}
