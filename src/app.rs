#[cfg(feature = "egui_parley")]
use eframe_parley as eframe;
#[cfg(feature = "egui_parley")]
use egui_extras_parley as egui_extras;
#[cfg(feature = "egui_parley")]
use egui_parley as egui;
#[cfg(feature = "egui_parley")]
use emath_parley as emath;

#[cfg(feature = "egui_latest")]
use eframe_latest as eframe;
#[cfg(feature = "egui_latest")]
use egui_extras_latest as egui_extras;
#[cfg(feature = "egui_latest")]
use egui_latest as egui;
#[cfg(feature = "egui_latest")]
use emath_latest as emath;

use crate::execute;
#[cfg(feature = "egui_latest")]
use egui::FontFamily;
#[cfg(feature = "egui_parley")]
use egui::text::style::FontFamily;

use egui::{Align, Color32, InnerResponse, Label, Layout, Response, RichText, Ui};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use strum_macros::EnumIter;

const THIN_SPACE: &str = "\u{2009}";
// const GRIP_SYMBOL: &str = "⠿";
const GRIP_SYMBOL: &str = "G";

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

    #[cfg(feature = "egui_latest")]
    {
        fonts
            .families
            .entry(egui::FontFamily::Name("IBM Plex Sans".into()))
            .or_default()
            .push("IBM Plex Sans".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Name("IBM Plex Sans Bold".into()))
            .or_default()
            .push("IBM Plex Sans Bold".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Name("IBM Plex Sans Bold Italic".into()))
            .or_default()
            .push("IBM Plex Sans Bold Italic".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Name("IBM Plex Sans ExtraLight".into()))
            .or_default()
            .push("IBM Plex Sans ExtraLight".to_owned());
    }

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

fn parse_mt940_file(bytes: &[u8]) -> Vec<DataRow> {
    let file_str = &String::from_utf8(bytes.to_vec()).unwrap();
    if file_str.contains(":61:220229") {
        warn!("Warning! Had to replace an occurence of an impossible date.",);
    }

    mt940::parse_mt940(&mt940::sanitizers::sanitize(
        &file_str.replace(":61:220229", ":61:220301"),
    ))
    .unwrap_or_else(|e| panic!("{}", e))
    .into_iter()
    .flat_map(DataRow::from_message)
    .collect()
}

struct Money {
    amount: rust_decimal::Decimal,
    iso_currency_code: String,
    credit: bool,
}

impl Renderable for Money {
    fn ui(&self, ui: &mut Ui, style: SelectStyle) {
        let currency_sign = match self.iso_currency_code.as_str() {
            "EUR" => "€",
            c => {
                error!("Unknown currency code {}", c);
                "?"
            }
        };
        let sign = match self.credit {
            true => "−",
            false => "",
        };
        let number = self.amount;
        ui.label(
            regular(format!("{sign}{number}{THIN_SPACE}{currency_sign}").as_str())
                .background_color(style.color()),
        );
    }
}

/// Distinction for how hovering over one element should highlight others
#[derive(Default, Clone, Copy)]
enum SelectStyle {
    /// The element is not affected at all
    #[default]
    Unaffected,
    /// The element is directly hovered over
    Hovered,
    /// Related to what is hovered on, first degree
    Related1,
    /// Related to what is hovered on, second degree
    Related2,
    /// Suggestion for user input, e.g. for suggesting a column name while creating a rule
    Suggestion,
}

impl SelectStyle {
    fn color(&self) -> egui::Color32 {
        use egui::Color32;
        match self {
            Self::Unaffected => Color32::TRANSPARENT,
            Self::Hovered => Color32::DARK_BLUE,
            Self::Related1 => Color32::BLUE,
            Self::Related2 => Color32::LIGHT_BLUE,
            Self::Suggestion => Color32::YELLOW,
        }
    }
}

/// Wrapper type so we can annotate wether a cell is highlighted
struct Highlightable<T: Renderable> {
    inner: T,
    style: SelectStyle,
}

impl<T: Renderable> Highlightable<T> {
    fn new(inner: T) -> Self {
        Self {
            inner,
            style: Default::default(),
        }
    }
    fn into_inner(self) -> T {
        self.inner
    }
    fn ui(&self, ui: &mut Ui) {
        self.inner.ui(ui, self.style);
    }
}

trait Renderable {
    fn ui(&self, ui: &mut Ui, style: SelectStyle);
}

impl Renderable for String {
    fn ui(&self, ui: &mut Ui, style: SelectStyle) {
        ui.add(Label::new(regular(self).background_color(style.color())).extend());
    }
}

impl Renderable for chrono::NaiveDate {
    fn ui(&self, ui: &mut Ui, style: SelectStyle) {
        ui.add(
            Label::new(regular(format!("{}", self).as_str()).background_color(style.color()))
                .extend(),
        );
    }
}

/// Wrapper type for an Iban
struct RawIban(String);

impl Renderable for RawIban {
    fn ui(&self, ui: &mut Ui, style: SelectStyle) {
        if let Ok(iban) = self.0.parse::<iban::Iban>() {
            ui.add(
                Label::new(
                    regular(format!("{iban}").replace(" ", THIN_SPACE).as_str())
                        .background_color(style.color()),
                )
                .extend(),
            );
        } else {
            ui.add(
                Label::new(
                    regular(format!("{}{THIN_SPACE}❌", self.0).as_str())
                        .background_color(style.color()),
                )
                .extend(),
            );
        }
    }
}

struct DataRow {
    date: Highlightable<chrono::NaiveDate>,
    money: Highlightable<Money>,
    iban: Option<Highlightable<RawIban>>,
    name: Option<Highlightable<String>>,
    purpose: Option<Highlightable<String>>,
    annotation: Option<String>,
}

impl DataRow {
    fn from_message(message: mt940::Message) -> Vec<Self> {
        use mt940::ExtDebitOrCredit;

        let iso_currency_code = message.opening_balance.iso_currency_code;
        message
            .statement_lines
            .into_iter()
            .map(|sl| {
                let credit = sl.ext_debit_credit_indicator == ExtDebitOrCredit::Credit
                    || sl.ext_debit_credit_indicator == ExtDebitOrCredit::ReverseDebit;
                let (name, iban, purpose) = match sl.information_to_account_owner {
                    Some(mt940::InformationToAccountOwner::Structured {
                        applicant_name,
                        applicant_iban,
                        purpose,
                        ..
                    }) => (applicant_name, applicant_iban, purpose),
                    None | Some(mt940::InformationToAccountOwner::Plain(_)) => (None, None, None),
                };
                DataRow {
                    date: Highlightable::new(sl.value_date),
                    money: Highlightable::new(Money {
                        amount: sl.amount,
                        iso_currency_code: iso_currency_code.clone(),
                        credit,
                    }),
                    iban: iban.map(RawIban).map(Highlightable::new),
                    name: name.map(Highlightable::new),
                    purpose: purpose.map(Highlightable::new),
                    annotation: None,
                }
            })
            .collect()
    }
    fn set_style_to_every_field(&mut self, style: SelectStyle) {
        self.date.style = style;
        self.money.style = style;
        if let Some(v) = self.iban.as_mut() {
            v.style = style;
        }
        if let Some(v) = self.name.as_mut() {
            v.style = style;
        }
        if let Some(v) = self.purpose.as_mut() {
            v.style = style;
        }
    }

    fn highlight(&mut self, rule: &Option<Rule>, condition: &Option<Condition>) {
        if rule.as_ref().is_some_and(|r| r.condition.matches(self)) {
            self.set_style_to_every_field(SelectStyle::Related2);
        } else if condition.as_ref().is_some_and(|c| c.matches(self)) {
            if let Some(Condition::Plain(comp)) = condition {
                match &comp.field {
                    Field::Name => {
                        if let Some(v) = self.name.as_mut() {
                            v.style = SelectStyle::Related1;
                        }
                    }
                    Field::Purpose => {
                        if let Some(v) = self.purpose.as_mut() {
                            v.style = SelectStyle::Related1;
                        }
                    }
                    Field::Iban => {
                        if let Some(v) = self.iban.as_mut() {
                            v.style = SelectStyle::Related1;
                        }
                    }
                }
            }
        } else {
            self.clear_highlight();
        }
    }
    fn clear_highlight(&mut self) {
        self.set_style_to_every_field(Default::default());
    }
}

#[derive(Default)]
pub struct App {
    data: Arc<Mutex<Vec<DataRow>>>,
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
            let data = parse_mt940_file(
                // This example file is from
                // https://github.com/svenstaro/mt940-rs/blob/29b547fb062de34cd8f39e9adff9d80dfa64dbdd/tests/data/mt940/full/betterplace/sepa_mt9401.sta
                include_bytes!("../sample_data/mt940.sta"),
            );
            Self {
                data: Arc::new(Mutex::new(data)),
                //rules: serde_json::from_slice(include_bytes!("../rules.json")).unwrap(),
                rules: vec![
                    Rule::new(
                        Condition::Plain(Comparison {
                            field: Field::Purpose,
                            ctype: ComparisonType::Contains("MINT".to_string()),
                        }),
                        "A".to_string(),
                    ),
                    Rule::new(
                        Condition::Plain(Comparison {
                            field: Field::Purpose,
                            ctype: ComparisonType::Contains("spezifiziert".to_string()),
                        }),
                        "B".to_string(),
                    ),
                    Rule::new(
                        Condition::Plain(Comparison {
                            field: Field::Purpose,
                            ctype: ComparisonType::Contains("Buchung".to_string()),
                        }),
                        "C".to_string(),
                    ),
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
    fn file_load_button(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        if self.data.lock().unwrap().is_empty() && ui.button(bold("Pick MT940 file")).clicked() {
            let task = rfd::AsyncFileDialog::new().pick_file();
            let data_clone = Arc::clone(&self.data);
            let ctx_clone = ctx.clone();
            execute(async move {
                let file = task.await;
                if let Some(file) = file {
                    let file_content = file.read().await;
                    info!("File loaded");
                    let parsed = parse_mt940_file(&file_content);
                    info!("Parsed {} MT940 rows", parsed.len());

                    let mut data = data_clone.lock().unwrap();
                    *data = parsed;
                    // Redraw so the user can see the result of file load even when window
                    // isn't active.
                    ctx_clone.request_repaint();
                }
            });
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
                        self.file_load_button(ctx, ui);

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
                                for entry in &*self.data.lock().unwrap() {
                                    body.row(0.0, |mut row| {
                                        // date
                                        row.col(|ui| {
                                            entry.date.ui(ui);
                                        });
                                        // amount
                                        row.col(|ui| {
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Min),
                                                |ui| {
                                                    entry.money.ui(ui);
                                                },
                                            );
                                        });
                                        // iban
                                        row.col(|ui| {
                                            if let Some(ibanh) = &entry.iban {
                                                ibanh.ui(ui);
                                            }
                                        });
                                        // name
                                        row.col(|ui| {
                                            if let Some(nameh) = &entry.name {
                                                nameh.ui(ui);
                                            }
                                        });
                                        // purpose
                                        row.col(|ui| {
                                            if let Some(purposeh) = &entry.purpose {
                                                purposeh.ui(ui);
                                            }
                                        });
                                    });
                                }
                            });
                    });
            });
        if response.response.hovered() {
            self.hints.push(Hint::Transactions);
        }
    }
    fn rules_panel(&mut self, ctx: &egui::Context) {
        let mut hovered_rule: Option<Rule> = None;
        let mut hovered_condition: Option<Condition> = None;

        let response = egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(Label::new(bold("rules")));
            egui::ScrollArea::both().show(ui, |ui| {
                // Store a potential drag&drop release event
                let mut from_to = None;

                for (i, rule) in self.rules.iter_mut().enumerate() {
                    let egui::InnerResponse {
                        inner: rule_response,
                        response,
                    } = rule.ui(ui, &mut hovered_condition, i);
                    if rule_response.hovered() {
                        hovered_rule = Some(rule.clone());
                    }

                    // If rule is being dragged we draw a tooltip at the cursor
                    if rule.dragged {
                        info!("Dragging");
                        let tooltip_layer_id =
                            egui::LayerId::new(egui::Order::Tooltip, format!("rule{}", i).into());
                        let egui::InnerResponse { inner: _, response } = ui.scope_builder(
                            egui::UiBuilder::new().layer_id(tooltip_layer_id),
                            |ui| {
                                ui.add(Label::new(regular(
                                    format!("dragging placeholder for rule {i}").as_str(),
                                )));
                            },
                        );
                        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                            let delta = pointer_pos - response.rect.center();
                            ui.ctx().transform_layer_shapes(
                                tooltip_layer_id,
                                emath::TSTransform::from_translation(delta),
                            );
                        }
                    }

                    // In case we are dragging over this rule
                    if let (Some(pointer), Some(hovered_payload)) = (
                        ui.input(|i| i.pointer.interact_pos()),
                        response.dnd_hover_payload::<usize>(),
                    ) {
                        let rect = response.rect;
                        let stroke = egui::Stroke::new(1.0, Color32::BLUE);
                        let i_to_insert_into = if *hovered_payload == i {
                            // We are dragging onto ourselves and do nothing
                            i
                        } else if pointer.y < rect.center().y {
                            // Dragging above
                            ui.painter().hline(rect.x_range(), rect.top(), stroke);
                            i
                        } else {
                            // Dragging below
                            ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                            i + 1
                        };
                        // In case we dropped onto this rule
                        if let Some(dropped_payload) = response.dnd_release_payload() {
                            from_to = Some((*dropped_payload, i_to_insert_into));
                        }
                    }
                }

                if let Some((from, to)) = from_to {
                    info!("From {from} to {to}");
                    if from != to {
                        let rule = self.rules.remove(from);
                        self.rules.insert(to.min(self.rules.len()), rule);
                    }
                }
            });
        });
        if response.response.hovered() {
            self.hints.push(Hint::Rules);
        }

        // might be inefficient?
        for entry in &mut *self.data.lock().unwrap() {
            entry.highlight(&hovered_rule, &hovered_condition);
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

        #[cfg(debug_assertions)]
        ctx.set_debug_on_hover(true);
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

/// When purpose contains "Cafe" then it is "expenses:4650bewirtungskosten"
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Rule {
    condition: Condition,
    category: String,
    hovered: bool,
    dragged: bool,
}

impl Rule {
    fn new(condition: Condition, category: String) -> Self {
        Rule {
            condition,
            category,
            hovered: false,
            dragged: false,
        }
    }
    fn ui(
        &mut self,
        ui: &mut Ui,
        hovered_condition: &mut Option<Condition>,
        rule_i: usize,
    ) -> InnerResponse<Response> {
        // When being dragged we render the rule in a more subtle color
        let color = if self.dragged {
            Color32::GRAY
        } else {
            Color32::PLACEHOLDER
        };
        let response = ui.horizontal(|ui| {
            let mut r = ui.add(Label::new(if self.hovered {
                bold(GRIP_SYMBOL).color(color)
            } else {
                regular(GRIP_SYMBOL).color(color)
            }));
            r.dnd_set_drag_payload(rule_i);
            self.dragged = r.dragged();
            r |= ui.add(Label::new(regular("When").color(color)));
            r |= self.condition.ui(ui, hovered_condition, color);
            r |= ui.add(Label::new(regular(&self.category).color(color)));
            r
        });
        self.hovered = response.inner.hovered();
        response
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, EnumIter)]
enum Condition {
    Plain(Comparison),
    Not(Box<Condition>),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
}

impl Condition {
    fn matches(&self, entry: &DataRow) -> bool {
        match self {
            Self::Plain(comparison) => comparison.matches(entry),
            Self::Not(comparison) => !comparison.matches(entry),
            Self::And(c1, c2) => c1.matches(entry) && c2.matches(entry),
            Self::Or(c1, c2) => c1.matches(entry) || c2.matches(entry),
        }
    }
    fn ui(
        &self,
        ui: &mut Ui,
        hovered_condition: &mut Option<Condition>,
        color: Color32,
    ) -> Response {
        let response = match self {
            Condition::Plain(Comparison {
                field,
                ctype: ComparisonType::Contains(value),
            }) => {
                let response = ui.add(Label::new(
                    regular(format!("{field:?} contains \"{value}\"").as_str()).color(color),
                ));
                if response.hovered() {
                    *hovered_condition = Some(self.clone());
                    info!("hovering condition {self:?}");
                }
                response
            }
            _ => {
                unimplemented!()
            }
        };
        response
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
    fn matches(&self, entry: &DataRow) -> bool {
        match (&self.field, entry) {
            (
                Field::Name,
                DataRow {
                    name: Some(Highlightable { inner: name, .. }),
                    ..
                },
            ) => self.ctype.matches(name),
            (
                Field::Purpose,
                DataRow {
                    purpose: Some(Highlightable { inner: purpose, .. }),
                    ..
                },
            ) => self.ctype.matches(purpose),
            (
                Field::Iban,
                DataRow {
                    iban: Some(Highlightable { inner: iban, .. }),
                    ..
                },
            ) => self.ctype.matches(&iban.0),
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
