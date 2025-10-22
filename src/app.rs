use crate::execute;
use egui::text::style::FontFamily;
use egui::{Align, Label, Layout, RichText, Ui};
use log::{error, info, warn};
use std::sync::{Arc, Mutex};

const THIN_SPACE: &str = "\u{2009}";

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

fn regular(text: &str) -> RichText {
    RichText::new(text).family(FontFamily::named("IBM Plex Sans"))
}

fn bold(text: &str) -> RichText {
    RichText::new(text).family(FontFamily::named("IBM Plex Sans Bold"))
}

fn parse_mt940_file(bytes: &[u8]) -> Vec<mt940::Message> {
    let file_str = &String::from_utf8(bytes.to_vec()).unwrap();
    if file_str.contains(":61:220229") {
        warn!("Warning! Had to replace an occurence of an impossible date.",);
    }

    mt940::parse_mt940(&mt940::sanitizers::sanitize(
        &file_str.replace(":61:220229", ":61:220301"),
    ))
    .unwrap_or_else(|e| panic!("{}", e))
}

pub struct App {
    messages: Arc<Mutex<Vec<mt940::Message>>>,
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        info!("Initializing app");
        load_fonts(&cc.egui_ctx);

        // For quicker development speed we load a file as default
        if cfg!(debug_assertions) {
            Self {
                messages: Arc::new(Mutex::new(parse_mt940_file(include_bytes!("../mt940.sta")))),
            }
        } else {
            Self {
                messages: Default::default(),
            }
        }
    }
}

impl eframe::App for App {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // ui.take_available_space();
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
                        let parsed = parse_mt940_file(&file_content);

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

            use egui_extras::{Column, TableBuilder};
            TableBuilder::new(ui)
                .auto_shrink([false, false])
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.label(bold("date"));
                    });
                    header.col(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                            ui.label(bold("amount"));
                        });
                    });
                    header.col(|ui| {
                        ui.label(bold("IBAN"));
                    });
                    header.col(|ui| {
                        ui.label(bold("name"));
                    });
                    header.col(|ui| {
                        ui.label(bold("purpose"));
                    });
                })
                .body(|mut body| {
                    for message in &*self.messages.lock().unwrap() {
                        for statement_line in &message.statement_lines {
                            body.row(0.0, |mut row| {
                                // date
                                row.col(|ui| {
                                    ui.add(
                                        Label::new(regular(
                                            format!("{}", statement_line.value_date).as_str(),
                                        ))
                                        .extend(),
                                    );
                                });
                                // amount
                                row.col(|ui| {
                                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                                        amount(
                                            ui,
                                            statement_line.amount,
                                            &statement_line.ext_debit_credit_indicator,
                                            &message.opening_balance.iso_currency_code,
                                        );
                                    });
                                });
                                // iban
                                row.col(|ui| {
                                    if let Some(mt940::InformationToAccountOwner::Structured {
                                        applicant_iban: Some(iban_str),
                                        ..
                                    }) = &statement_line.information_to_account_owner
                                    {
                                        iban(ui, iban_str);
                                    }
                                });
                                // name
                                row.col(|ui| {
                                    if let Some(mt940::InformationToAccountOwner::Structured {
                                        applicant_name: Some(name_str),
                                        ..
                                    }) = &statement_line.information_to_account_owner
                                    {
                                        ui.add(Label::new(regular(name_str)).extend());
                                    }
                                });
                                // purpose
                                row.col(|ui| {
                                    if let Some(mt940::InformationToAccountOwner::Structured {
                                        purpose: Some(purpose_str),
                                        ..
                                    }) = &statement_line.information_to_account_owner
                                    {
                                        ui.add(Label::new(regular(purpose_str)).extend());
                                    }
                                });
                            });
                        }
                    }
                });
        });
    }
}

fn amount(
    ui: &mut Ui,
    number: rust_decimal::Decimal,
    debit_or_credit: &mt940::ExtDebitOrCredit,
    iso_currency_code: &str,
) {
    let currency_sign = match iso_currency_code {
        "EUR" => "€",
        _ => {
            error!("Unknown currency code {}", iso_currency_code);
            "?"
        }
    };
    let sign = match debit_or_credit {
        mt940::ExtDebitOrCredit::Debit | mt940::ExtDebitOrCredit::ReverseCredit => "−",
        mt940::ExtDebitOrCredit::Credit | mt940::ExtDebitOrCredit::ReverseDebit => "",
    };
    ui.label(regular(
        format!("{sign}{number}{THIN_SPACE}{currency_sign}").as_str(),
    ));
}

fn iban(ui: &mut Ui, iban: &str) {
    if let Ok(iban) = iban.parse::<iban::Iban>() {
        ui.add(Label::new(regular(format!("{iban}").replace(" ", THIN_SPACE).as_str())).extend());
    } else {
        ui.add(Label::new(regular(format!("{iban}{THIN_SPACE}❌").as_str())).extend());
    }
}
