use crate::execute;
use egui::RichText;
use egui::text::style::FontFamily;
use log::{info, warn};
use std::sync::{Arc, Mutex};

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

impl eframe::App for App {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            if self.messages.lock().unwrap().is_empty()
                && ui
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
                            warn!("Warning! Had to replace on occurence to an impossible date.",);
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
