use log::{info, warn};
use std::sync::{Arc, Mutex};

use egui::RichText;
use egui::text::style::FontFamily;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };
    eframe::run_native(
        "implementation",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    log::info!("Starting app");

    let document = web_sys::window()
        .expect("No window")
        .document()
        .expect("No document");

    let canvas = document
        .get_element_by_id("the_canvas_id")
        .expect("Failed to find the_canvas_id")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("the_canvas_id was not a HtmlCanvasElement");

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(App::new(cc)))),
            )
            .await
            .expect("failed to start eframe");
    });
}

fn load_fonts(ctx: &egui::Context) {
    use egui::{FontData, FontDefinitions};
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "IBM Plex Sans".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/IBMPlexSans-Regular.otf")).into(),
    );
    fonts.font_data.insert(
        "IBM Plex Sans Bold".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/IBMPlexSans-Bold.otf")).into(),
    );
    fonts.font_data.insert(
        "IBM Plex Sans Bold Italic".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/IBMPlexSans-BoldItalic.otf")).into(),
    );
    fonts.font_data.insert(
        "IBM Plex Sans ExtraLight".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/IBMPlexSans-ExtraLight.otf")).into(),
    );
    // We don't register default font so that non-explicit font use is noticed.
    ctx.set_fonts(fonts);
}

pub struct App {
    messages: Arc<Mutex<Vec<mt940::Message>>>,
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        info!("Initializing app");
        load_fonts(&cc.egui_ctx);
        Self {
            messages: Default::default(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    std::thread::spawn(move || futures::executor::block_on(f));
}
#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

impl eframe::App for App {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            if self.messages.lock().unwrap().is_empty() {
                if ui
                    .button(
                        RichText::new("Pick MT940 file")
                            .family(FontFamily::named("IBM Plex Sans ExtraLight")),
                    )
                    .clicked()
                {
                    let task = rfd::AsyncFileDialog::new().pick_file();
                    let messages_clone = Arc::clone(&self.messages);
                    let ctx_clone = ctx.clone();
                    execute(async move {
                        let file = task.await;
                        if let Some(file) = file {
                            let file_content = file.read().await;
                            info!("File loaded");
                            let file_str = &String::from_utf8(file_content).unwrap();
                            if file_str.contains(":61:220229") {
                                warn!(
                                    "Warning! Had to replace on occurence to an impossible date.",
                                );
                            }

                            let parsed = mt940::parse_mt940(&mt940::sanitizers::sanitize(
                                &file_str.replace(":61:220229", ":61:220301"),
                            ))
                            .unwrap_or_else(|e| panic!("{}", e));

                            info!("Parsed {} MT940 messages", parsed.len());
                            let mut messages = messages_clone.lock().unwrap();
                            *messages = parsed;
                            // Redraw so the user can see the result of file load even when window
                            // isn't active.
                            ctx_clone.request_repaint();
                        }
                    });
                }
            } else {
            }
            ui.label(
                RichText::new(format!("{}", self.messages.lock().unwrap().len()))
                    .family(FontFamily::named("IBM Plex Sans ExtraLight")),
            );
            ui.label(
                RichText::new("Bold Text")
                    .family(FontFamily::named("IBM Plex Sans ExtraLight"))
                    .size(50.0),
            );
        });
    }
}
