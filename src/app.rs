#[cfg(feature = "egui_parley")]
use eframe_parley as eframe;
#[cfg(feature = "egui_parley")]
use egui_extras_parley as egui_extras;
#[cfg(feature = "egui_parley")]
use egui_parley as egui;

#[cfg(feature = "egui_latest")]
use eframe_latest as eframe;
#[cfg(feature = "egui_latest")]
use egui_extras_latest as egui_extras;
#[cfg(feature = "egui_latest")]
use egui_latest as egui;

use crate::execute;
#[cfg(feature = "egui_latest")]
use egui::FontFamily;
#[cfg(feature = "egui_parley")]
use egui::text::style::FontFamily;
use egui::{Align, Frame, Label, Layout, RichText, Ui};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use strum_macros::EnumIter;

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
    #[cfg(feature = "egui_parley")]
    let family = FontFamily::Named("IBM Plex Sans".into());
    #[cfg(feature = "egui_latest")]
    let family = FontFamily::Name("IBM Plex Sans".into());
    RichText::new(text).family(family)
}

fn bold(text: &str) -> RichText {
    #[cfg(feature = "egui_parley")]
    let family = FontFamily::Named("IBM Plex Sans Bold".into());
    #[cfg(feature = "egui_latest")]
    let family = FontFamily::Name("IBM Plex Sans Bold".into());
    RichText::new(text).family(family)
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

#[derive(Default)]
pub struct App {
    messages: Arc<Mutex<Vec<mt940::Message>>>,
    rules: Vec<Rule>,
    hovered_rule: Option<usize>,
    hints: Vec<Hint>,
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        info!("Initializing app");
        load_fonts(&cc.egui_ctx);

        // For quicker development speed we load a file as default
        #[cfg(feature = "demo")]
        {
            Self {
                messages: Arc::new(Mutex::new(parse_mt940_file(
                    // This example file is from
                    // https://github.com/svenstaro/mt940-rs/blob/29b547fb062de34cd8f39e9adff9d80dfa64dbdd/tests/data/mt940/full/betterplace/sepa_mt9401.sta
                    include_bytes!("../sample_data/mt940.sta"),
                ))),
                //rules: serde_json::from_slice(include_bytes!("../rules.json")).unwrap(),
                rules: vec![
                    Rule {
                        condition: Condition::Plain(Comparison {
                            field: Field::Purpose,
                            ctype: ComparisonType::Contains("cafe".to_string()),
                        }),
                        category: "expenses:4650bewirtungskosten".to_string(),
                    },
                    Rule {
                        condition: Condition::Plain(Comparison {
                            field: Field::Purpose,
                            ctype: ComparisonType::Contains("Anlage".to_string()),
                        }),
                        category: "expenses:4650bewirtungskosten".to_string(),
                    },
                ],
                hovered_rule: Default::default(),
                hints: Default::default(),
            }
        }
        #[cfg(not(feature = "demo"))]
        {
            Default::default()
        }
    }
    fn data_panel(&mut self, ctx: &egui::Context) {
        let response = egui::TopBottomPanel::top("data_panel")
            .resizable(true)
            .min_height(TRANSACTIONS_HEIGHT_MIN)
            .default_height(ctx.screen_rect().max.y * 0.5)
            .show(ctx, |ui| {
                egui::ScrollArea::both()
                    .min_scrolled_height(TRANSACTIONS_HEIGHT_MIN)
                    .show(ui, |ui| {
                        if self.messages.lock().unwrap().is_empty()
                            && ui.button(bold("Pick MT940 file")).clicked()
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
                                        let highlight_row = if let Some(rule_i) = self.hovered_rule
                                        {
                                            self.rules[rule_i].condition.matches(statement_line)
                                        } else {
                                            false
                                        };
                                        body.row(0.0, |mut row| {
                                            row.set_selected(highlight_row);
                                            // date
                                            row.col(|ui| {
                                                ui.add(
                                                    Label::new(regular(
                                                        format!("{}", statement_line.value_date)
                                                            .as_str(),
                                                    ))
                                                    .extend(),
                                                );
                                            });
                                            // amount
                                            row.col(|ui| {
                                                ui.with_layout(
                                                    Layout::right_to_left(Align::Min),
                                                    |ui| {
                                                        amount(
                                                            ui,
                                                            statement_line.amount,
                                                            &statement_line
                                                                .ext_debit_credit_indicator,
                                                            &message
                                                                .opening_balance
                                                                .iso_currency_code,
                                                        );
                                                    },
                                                );
                                            });
                                            // iban
                                            row.col(|ui| {
                                                if let Some(
                                                    mt940::InformationToAccountOwner::Structured {
                                                        applicant_iban: Some(iban_str),
                                                        ..
                                                    },
                                                ) = &statement_line.information_to_account_owner
                                                {
                                                    iban(ui, iban_str);
                                                }
                                            });
                                            // name
                                            row.col(|ui| {
                                                if let Some(
                                                    mt940::InformationToAccountOwner::Structured {
                                                        applicant_name: Some(name_str),
                                                        ..
                                                    },
                                                ) = &statement_line.information_to_account_owner
                                                {
                                                    ui.add(Label::new(regular(name_str)).extend());
                                                }
                                            });
                                            // purpose
                                            row.col(|ui| {
                                                if let Some(
                                                    mt940::InformationToAccountOwner::Structured {
                                                        purpose: Some(purpose_str),
                                                        ..
                                                    },
                                                ) = &statement_line.information_to_account_owner
                                                {
                                                    ui.add(
                                                        Label::new(regular(purpose_str)).extend(),
                                                    );
                                                }
                                            });
                                        });
                                    }
                                }
                            });
                    });
            });
        if response.response.hovered() {
            self.hints.push(Hint::Transactions);
        }
    }
    fn rules_panel(&mut self, ctx: &egui::Context) {
        let response = egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.add(Label::new(bold("rules")));
                let frame = Frame::default().inner_margin(4.0);
                let mut from = None;
                let mut to = None;
                let (_, _) = ui.dnd_drop_zone::<usize, ()>(frame, |ui| {
                    for (i, r) in self.rules.clone().into_iter().enumerate() {
                        let response = ui
                            .dnd_drag_source(egui::Id::new(("draggable_rule", i)), i, |ui| {
                                rule(ui, &r);
                            })
                            .response;
                        if response.hovered() {
                            self.hovered_rule = Some(i);
                        } else if Some(i) == self.hovered_rule {
                            self.hovered_rule = None;
                        }
                        // Detect drops onto this item:
                        if let (Some(pointer), Some(hovered_payload)) = (
                            ui.input(|i| i.pointer.interact_pos()),
                            response.dnd_hover_payload::<usize>(),
                        ) {
                            let stroke = egui::Stroke::new(1.0, egui::Color32::RED);
                            let rect = response.rect;
                            let insert_row_id = if *hovered_payload == i {
                                // We dragged onto ourselves
                                ui.painter().hline(rect.x_range(), rect.center().y, stroke);
                                i
                            } else if pointer.y < rect.center().y {
                                // Above us
                                ui.painter().hline(rect.x_range(), rect.top(), stroke);
                                i
                            } else {
                                // Below us
                                ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                                i + 1
                            };
                            from = Some(hovered_payload.clone());
                            to = Some(insert_row_id);
                        }
                    }
                });
                if let (Some(from), Some(to)) = (from, to) {
                    ui.add(Label::new(regular(
                        format!("from {} to {}", from, to).as_str(),
                    )));
                    let rule = self.rules.remove(*from);
                    self.rules.insert(std::cmp::min(to, self.rules.len()), rule);
                }
            });
        });
        if response.response.hovered() {
            self.hints.push(Hint::Rules);
        }
    }
    fn bottom_bar(&mut self, ctx: &egui::Context) {
        egui::Area::new("bottom_bar".into())
            .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(0.0, 0.0))
            .order(egui::Order::Foreground) // ensure it draws above other panels
            .show(ctx, |ui| {
                ui.set_width(ctx.screen_rect().width()); // span full width
                ui.set_height(BOTTOM_BAR_HEIGHT);

                ui.horizontal(|ui| {
                    if let Some(hint) = self.hints.last() {
                        hint.view(ui);
                    }
                });
            });
    }
}

/// The minimal allowed height of the transactions panel
const TRANSACTIONS_HEIGHT_MIN: f32 = 50.0;

/// The fixed height of the context bar
const BOTTOM_BAR_HEIGHT: f32 = 25.0;

impl eframe::App for App {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.hints.clear();

        self.data_panel(ctx);
        self.rules_panel(ctx);
        self.bottom_bar(ctx);
    }
}

// The interface element at the bottom of the screen that shows helpful text depending on which
// element is hovered.
#[derive(Default)]
enum Hint {
    #[default]
    None,
    Transactions,
    Rules,
}

impl Hint {
    fn view(&self, ui: &mut Ui) {
        let text = match self {
            Self::None => "",
            Self::Transactions => "Transactions",
            Self::Rules => "Rules",
        };
        ui.add(Label::new(regular(text)));
    }
}

fn rule(ui: &mut Ui, rule: &Rule) {
    ui.horizontal(|ui| {
        ui.add(Label::new(regular("When")));
        condition(ui, &rule.condition);
        ui.add(Label::new(regular(&rule.category)));
    });
}

fn condition(ui: &mut Ui, condition: &Condition) {
    match condition {
        Condition::Plain(Comparison {
            field,
            ctype: ComparisonType::Contains(value),
        }) => {
            ui.add(Label::new(regular(
                format!("{field:?} contains \"{value}\"").as_str(),
            )));
        }
        _ => {
            unimplemented!()
        }
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

/// When purpose contains "Cafe" then it is "expenses:4650bewirtungskosten"
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Rule {
    condition: Condition,
    category: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, EnumIter)]
enum Condition {
    Plain(Comparison),
    Not(Comparison),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
}

impl Condition {
    fn matches(&self, sl: &mt940::StatementLine) -> bool {
        match self {
            Self::Plain(comparison) => comparison.matches(sl),
            Self::Not(comparison) => !comparison.matches(sl),
            Self::And(c1, c2) => c1.matches(sl) && c2.matches(sl),
            Self::Or(c1, c2) => c1.matches(sl) || c2.matches(sl),
        }
    }
}

impl Default for Condition {
    fn default() -> Self {
        Self::Plain(Default::default())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Comparison {
    field: Field,
    ctype: ComparisonType,
}

impl Comparison {
    fn matches(&self, sl: &mt940::StatementLine) -> bool {
        match (&self.field, sl) {
            (
                Field::Name,
                mt940::StatementLine {
                    information_to_account_owner:
                        Some(mt940::InformationToAccountOwner::Structured {
                            applicant_name: Some(name),
                            ..
                        }),
                    ..
                },
            ) => self.ctype.matches(name),
            (
                Field::Purpose,
                mt940::StatementLine {
                    information_to_account_owner:
                        Some(mt940::InformationToAccountOwner::Structured {
                            purpose: Some(purpose),
                            ..
                        }),
                    ..
                },
            ) => self.ctype.matches(purpose),
            (
                Field::Iban,
                mt940::StatementLine {
                    information_to_account_owner:
                        Some(mt940::InformationToAccountOwner::Structured {
                            applicant_iban: Some(iban),
                            ..
                        }),
                    ..
                },
            ) => self.ctype.matches(iban),
            _ => false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum ComparisonType {
    Exact(String),
    Contains(String),
}

impl Default for ComparisonType {
    fn default() -> Self {
        Self::Exact(Default::default())
    }
}

impl ComparisonType {
    fn matches(&self, value: &str) -> bool {
        match self {
            Self::Contains(str) => value.contains(str),
            Self::Exact(str) => value == str,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
enum Field {
    #[default]
    Name,
    Purpose,
    Iban,
}
