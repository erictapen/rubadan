#[cfg(feature = "egui_parley")]
use eframe_parley as eframe;
#[cfg(feature = "egui_parley")]
use egui_extras_parley as egui_extras;
#[cfg(feature = "egui_parley")]
use egui_parley as egui;
#[cfg(feature = "egui_parley")]
use emath_parley as emath;
#[cfg(feature = "egui_parley")]
use epaint_parley as epaint;

#[cfg(feature = "egui_latest")]
use eframe_latest as eframe;
#[cfg(feature = "egui_latest")]
use egui_extras_latest as egui_extras;
#[cfg(feature = "egui_latest")]
use egui_latest as egui;
#[cfg(feature = "egui_latest")]
use emath_latest as emath;
#[cfg(feature = "egui_latest")]
use epaint_latest as epaint;

use crate::execute;
#[cfg(feature = "egui_latest")]
use egui::FontFamily;
#[cfg(feature = "egui_parley")]
use egui::text::style::FontFamily;

use egui::{
    Align, Button, Color32, Frame, InnerResponse, Label, Layout, Margin, Rect, Response, RichText,
    Ui, UiBuilder,
};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use strum_macros::EnumIter;

use crate::widgets;

const THIN_SPACE: &str = "\u{2009}";
const GRIP_SYMBOL: &str = "⠿";

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
    fonts.font_data.insert(
        "Noto Sans Symbols2".to_owned(),
        FontData::from_static(include_bytes!(
            "../assets/fonts/NotoSansSymbols2-Regular.otf"
        ))
        .into(),
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
        fonts
            .families
            .entry(egui::FontFamily::Name("Noto Sans Symbols2".into()))
            .or_default()
            .push("Noto Sans Symbols2".to_owned());
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

fn symbol(text: &str) -> RichText {
    #[cfg(feature = "egui_parley")]
    let family = FontFamily::Named("Noto Sans Symbols2".into());
    #[cfg(feature = "egui_latest")]
    let family = FontFamily::Name("Noto Sans Symbols2".into());
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
    fn ui(&self, ui: &mut Ui, style: SelectStyle, minimap: &mut Minimap) {
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
        let response = ui.label(
            regular(format!("{sign}{number}{THIN_SPACE}{currency_sign}").as_str())
                .background_color(style.color()),
        );
        minimap.push(response.rect, style);
    }
}

/// Distinction for how hovering over one element should highlight others
#[derive(Default, Clone, Copy, PartialEq)]
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
    fn color_minimap(&self) -> egui::Color32 {
        use egui::Color32;
        if self == &Self::Unaffected {
            // We want unaffected text to still be visible in the minimap
            Color32::GRAY
        } else {
            self.color()
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
    fn ui(&self, ui: &mut Ui, minimap: &mut Minimap) {
        self.inner.ui(ui, self.style, minimap);
    }
}

trait Renderable {
    /// Draw something and insert the resulting RectShape into the minimap
    fn ui(&self, ui: &mut Ui, style: SelectStyle, minimap: &mut Minimap);
}

impl Renderable for String {
    fn ui(&self, ui: &mut Ui, style: SelectStyle, minimap: &mut Minimap) {
        let response = ui.add(Label::new(regular(self).background_color(style.color())).extend());
        minimap.push(response.rect, style);
    }
}

impl Renderable for chrono::NaiveDate {
    fn ui(&self, ui: &mut Ui, style: SelectStyle, minimap: &mut Minimap) {
        let response = ui.add(
            Label::new(regular(format!("{}", self).as_str()).background_color(style.color()))
                .extend(),
        );
        minimap.push(response.rect, style);
    }
}

/// Wrapper type for an Iban
struct RawIban(String);

impl Renderable for RawIban {
    fn ui(&self, ui: &mut Ui, style: SelectStyle, minimap: &mut Minimap) {
        let response;
        if let Ok(iban) = self.0.parse::<iban::Iban>() {
            response = ui.add(
                Label::new(
                    regular(format!("{iban}").replace(" ", THIN_SPACE).as_str())
                        .background_color(style.color()),
                )
                .extend(),
            );
        } else {
            response = ui.add(
                Label::new(
                    regular(format!("{}{THIN_SPACE}❌", self.0).as_str())
                        .background_color(style.color()),
                )
                .extend(),
            );
        }
        minimap.push(response.rect, style);
    }
}

struct DataRow {
    date: Highlightable<chrono::NaiveDate>,
    money: Highlightable<Money>,
    iban: Option<Highlightable<RawIban>>,
    name: Option<Highlightable<String>>,
    purpose: Option<Highlightable<String>>,
    annotation: Option<Highlightable<String>>,
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

    /// # Arguments
    /// * `rule` - The rule that is being hovered
    /// * `condition` - In case we just hover over a condition inside the a Rule
    fn highlight(&mut self, rule: &Option<Rule>, condition: &Option<Condition>) {
        self.clear_highlight();
        if let Some(Rule::Complete { condition, .. }) = rule {
            // We highlight even when the rule is disabled, since we still give the user feedback for
            // what would happen if they'd enable it
            if condition.matches(self) {
                self.set_style_to_every_field(SelectStyle::Related2);
            }
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
        }
    }
    fn clear_highlight(&mut self) {
        self.set_style_to_every_field(Default::default());
    }
}

/// All the information that we need to paint the minimap. Needs to be persisted as we draw the
/// rest of the data panel before the minimap
#[derive(Default)]
struct Minimap {
    /// The entire space
    frame: Option<egui::Rect>,
    elements: Vec<epaint::RectShape>,
    /// The portion that is visible when scrolling
    visible_rect: Option<egui::Rect>,
}

impl Minimap {
    fn push(&mut self, rect: egui::Rect, style: SelectStyle) {
        self.elements.push(epaint::RectShape::filled(
            rect,
            epaint::CornerRadius::ZERO,
            style.color_minimap(),
        ));
    }
    fn clear(&mut self) {
        self.frame = None;
        self.elements.clear();
        self.visible_rect = None;
    }
}

#[derive(Default)]
pub struct App {
    data: Arc<Mutex<Vec<DataRow>>>,
    rules: Vec<Rule>,
    hints: Vec<Hint>,
    minimap: Minimap,
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        info!("Initializing app");
        load_fonts(&cc.egui_ctx);

        // Force lightmode theme for now until we have darkmode colors
        cc.egui_ctx.set_theme(egui::Theme::Light);

        // For quicker development speed we load a file as default
        #[cfg(feature = "demo")]
        let mut result = {
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
                hints: Default::default(),
                minimap: Default::default(),
            }
        };
        #[cfg(not(feature = "demo"))]
        let mut result: App = { Default::default() };
        result.update_annotations();
        result
    }
    fn file_load_button(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        let widget = |ui: &mut Ui| {
            Frame::NONE
                .stroke(egui::Stroke::new(1.0, Color32::BLUE))
                .inner_margin(Margin::same(120))
                .show(ui, |ui| {
                    let button_response = ui.button(bold("Pick MT940 file"));
                    if button_response.clicked() {
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
                });
        };
        let widget_size = ui
            .scope_builder(UiBuilder::new().invisible(), widget)
            .response
            .rect
            .size();
        let available_size = ui.max_rect().size();
        ui.allocate_ui_at_rect(
            Rect::from_min_size(
                [
                    available_size.x / 2.0 - widget_size.x / 2.0,
                    available_size.y / 2.0 - widget_size.y / 2.0,
                ]
                .into(),
                widget_size,
            ),
            widget,
        );
    }
    /// Annotate data rows
    fn update_annotations(&mut self) {
        for entry in &mut *self.data.lock().unwrap() {
            entry.annotation = None;
            for rule in &self.rules {
                // We only use complete rules for annotation
                if let Rule::Complete { category, .. } = rule {
                    if rule.matches(entry) {
                        entry.annotation = Some(Highlightable::new(category.clone()));
                    }
                }
            }
        }
    }
    fn minimap(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        if ctx.input(|i| i.screen_rect().width()) > 600.0 {
            egui::SidePanel::right("minimap")
                .resizable(false)
                .exact_width(200.0)
                .frame(egui::Frame::NONE.fill(egui::Color32::WHITE))
                .show_inside(ui, |ui| {
                    if let (Some(from), Some(mut visible_rect)) =
                        (self.minimap.frame, self.minimap.visible_rect)
                    {
                        let mut to = ui.max_rect();
                        to.set_height(to.width() * from.aspect_ratio());
                        let transform = emath::RectTransform::from_to(from, to);
                        visible_rect = transform.transform_rect(visible_rect);
                        // We try to keep the visible_rect inside the minimap
                        let y_offset = (visible_rect.max.y - ui.min_rect().height()).max(0.0);

                        for mut rect_shape in self.minimap.elements.drain(..) {
                            rect_shape.rect = transform.transform_rect(rect_shape.rect);
                            rect_shape.rect = rect_shape.rect.translate([0.0, -y_offset].into());
                            ui.painter().add(rect_shape);
                        }

                        visible_rect = visible_rect.translate([0.0, -y_offset].into());
                        ui.painter().add(epaint::RectShape::filled(
                            visible_rect,
                            epaint::CornerRadius::same(5),
                            Color32::LIGHT_GRAY.linear_multiply(0.5),
                        ));
                    }
                });
        }
        // Clear state so that we can write down elements again
        self.minimap.clear();
    }
    fn data_panel(&mut self, ctx: &egui::Context) {
        let table_response = egui::TopBottomPanel::top("data_panel")
            .resizable(true)
            .min_height(TRANSACTIONS_HEIGHT_MIN)
            .exact_height(ctx.screen_rect().max.y * 0.5)
            .default_height(ctx.screen_rect().max.y * 0.5)
            .show(ctx, |ui| {
                if self.data.lock().unwrap().is_empty() {
                    self.file_load_button(ctx, ui);
                } else {
                    self.minimap(ctx, ui);

                    let horizontal_state = egui::ScrollArea::horizontal().show(ui, |ui| {
                        use egui_extras::{Column, TableBuilder};
                        let table_state = TableBuilder::new(ui)
                            .auto_shrink([false, false])
                            .column(Column::auto())
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
                                header.col(|ui| {
                                    ui.label(bold("annotation"));
                                });
                            })
                            .body(|mut body| {
                                for entry in &*self.data.lock().unwrap() {
                                    body.row(0.0, |mut row| {
                                        // date
                                        row.col(|ui| {
                                            entry.date.ui(ui, &mut self.minimap);
                                        });
                                        // amount
                                        row.col(|ui| {
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Min),
                                                |ui| {
                                                    entry.money.ui(ui, &mut self.minimap);
                                                },
                                            );
                                        });
                                        // iban
                                        row.col(|ui| {
                                            if let Some(ibanh) = &entry.iban {
                                                ibanh.ui(ui, &mut self.minimap);
                                            }
                                        });
                                        // name
                                        row.col(|ui| {
                                            if let Some(nameh) = &entry.name {
                                                nameh.ui(ui, &mut self.minimap);
                                            }
                                        });
                                        // purpose
                                        row.col(|ui| {
                                            if let Some(purposeh) = &entry.purpose {
                                                purposeh.ui(ui, &mut self.minimap);
                                            }
                                        });
                                        // annotation
                                        row.col(|ui| {
                                            if let Some(annotationh) = &entry.annotation {
                                                annotationh.ui(ui, &mut self.minimap);
                                            }
                                        });
                                    });
                                }
                                self.minimap.frame = Some(body.ui_mut().min_rect());
                            });

                        // For visualising which part of the minimap is visible in the data panel,
                        // we add another shape to signify scroll state
                        self.minimap.visible_rect = Some(table_state.inner_rect);
                    });
                    if let Some(ref mut visible_rect) = self.minimap.visible_rect {
                        *visible_rect = visible_rect.intersect(horizontal_state.inner_rect);
                    }
                }
            });
        if table_response.response.hovered() {
            self.hints.push(Hint::Transactions);
        }
    }
    fn rules_panel(&mut self, ctx: &egui::Context) {
        let mut a_rule_changed = false;
        let mut a_rule_is_being_edited = false;

        let mut hovered_rule: Option<Rule> = None;
        let mut hovered_condition: Option<Condition> = None;

        let response = egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(Label::new(bold("rules")));
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let dragging_pointer: Option<egui::Pos2> = ui
                        .input(|i| i.pointer.interact_pos())
                        .filter(|_| egui::DragAndDrop::has_payload_of_type::<usize>(ctx));
                    let mut last_rule_center = None;
                    let rules_len = self.rules.len();
                    let mut drop_to = None;

                    for (i, rule) in self.rules.iter_mut().enumerate() {
                        match &*rule {
                            &Rule::Complete {
                                enabled, dragged, ..
                            } => {
                                let old_enabled = enabled.clone();
                                let egui::InnerResponse {
                                    inner: rule_response,
                                    response,
                                } = rule.ui(ui, &mut hovered_condition, i);
                                if let Rule::Complete { enabled, .. } = rule {
                                    a_rule_changed |= old_enabled != *enabled;
                                }
                                if rule_response.hovered() {
                                    hovered_rule = Some(rule.clone());
                                }

                                // In case there is any rule being dragged we preview the drop position
                                if let Some(pointer) = dragging_pointer {
                                    let stroke = egui::Stroke::new(1.0, Color32::BLUE);
                                    let rect = response.rect;
                                    let current_center = rect.center().y;
                                    // First rule being drawn and the pointer is above it or it's inbetween
                                    // the most recent one and this one
                                    if last_rule_center.unwrap_or(0.0) <= pointer.y
                                        && pointer.y < current_center
                                    {
                                        if let Some(last_rule_center) = last_rule_center {
                                            ui.painter().hline(
                                                rect.x_range(),
                                                egui::Rangef::new(
                                                    last_rule_center,
                                                    rect.center().y,
                                                )
                                                .center(),
                                                stroke,
                                            );
                                        } else {
                                            ui.painter().hline(rect.x_range(), rect.top(), stroke);
                                        }
                                        drop_to = Some(i);
                                    }
                                    // pointer is after the last rule
                                    else if rules_len - 1 == i && current_center < pointer.y {
                                        ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                                        drop_to = Some(i);
                                    }
                                }

                                last_rule_center = Some(response.rect.center().y);

                                // If the rule itself is being dragged we draw a tooltip at the cursor
                                // FIXME for some reason this adds a newline like gap after the rule
                                if dragged {
                                    let tooltip_layer_id = egui::LayerId::new(
                                        egui::Order::Tooltip,
                                        format!("rule{}", i).into(),
                                    );
                                    let egui::InnerResponse { inner: _, response } = ui
                                        .scope_builder(
                                            egui::UiBuilder::new().layer_id(tooltip_layer_id),
                                            |ui| {
                                                ui.add(Label::new(regular(
                                                    format!("dragging placeholder for rule {i}")
                                                        .as_str(),
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
                            }
                            &Rule::Incomplete { .. } => {
                                a_rule_is_being_edited = true;
                            }
                        }
                    }

                    // In case a rule was dragdropped this frame
                    if let (true, Some(from), Some(to)) = (
                        ctx.input(|i| i.pointer.any_released()),
                        egui::DragAndDrop::payload::<usize>(ctx),
                        drop_to,
                    ) {
                        let rule = self.rules.remove(*from);
                        self.rules.insert(to.min(self.rules.len()), rule);
                        a_rule_changed = true;
                    }

                    if !a_rule_is_being_edited
                        && ui.add(Button::new(regular("+ Add Rule"))).clicked()
                    {
                        self.rules.push(Rule::Incomplete {
                            condition: Condition::incomplete(),
                            category: None,
                        });
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

        if a_rule_changed {
            info!("At least one rule changed this frame");
            self.update_annotations();
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

        // We turn off text selection globally in case the user is dragging
        // something
        if egui::DragAndDrop::has_any_payload(ctx) {
            ctx.style_mut(|style| {
                style.interaction.selectable_labels = false;
            });
        } else {
            ctx.style_mut(|style| {
                style.interaction.selectable_labels = true;
            });
        }

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
enum Rule {
    Complete {
        enabled: bool,
        condition: Condition,
        category: String,
        hovered: bool,
        dragged: bool,
    },
    Incomplete {
        condition: Condition,
        category: Option<String>,
    },
}

impl Rule {
    fn new(condition: Condition, category: String) -> Self {
        Rule::Complete {
            enabled: true,
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
        match self {
            &mut Rule::Complete {
                ref mut enabled,
                mut dragged,
                ref mut hovered,
                ref condition,
                ref category,
                ..
            } => {
                // When being dragged we render the rule in a more subtle color
                let color = match (&*enabled, dragged) {
                    (false, _) | (_, true) => Color32::GRAY,
                    (_, false) => Color32::PLACEHOLDER,
                };
                let response = ui.horizontal(|ui| {
                    let mut r = ui.add(Label::new(if *hovered {
                        symbol(GRIP_SYMBOL).color(Color32::DARK_GRAY)
                    } else {
                        symbol(GRIP_SYMBOL).color(Color32::TRANSPARENT)
                    }));
                    r.dnd_set_drag_payload(rule_i);
                    dragged = r.dragged();
                    r |= ui.add(widgets::toggle_switch::toggle(enabled));
                    let mut rule_text = ui.add(Label::new(regular("When").color(color)));
                    rule_text |= condition.ui(ui, hovered_condition, color);
                    rule_text |= ui.add(Label::new(regular(&category).color(color)));
                    if !*enabled {
                        ui.painter().hline(
                            rule_text.rect.x_range(),
                            rule_text.rect.center().y,
                            egui::Stroke::new(1.0, Color32::LIGHT_GRAY),
                        );
                    }
                    r |= rule_text;
                    r
                });
                *hovered = response.inner.hovered();
                response
            }

            Self::Incomplete { .. } => {
                let response = ui.horizontal(|ui| {
                    let mut r = ui.add(Label::new(regular("wip")));
                    r
                });
                response
            }
        }
    }
    fn matches(&self, entry: &DataRow) -> bool {
        match self {
            Rule::Complete {
                enabled, condition, ..
            } => *enabled && condition.matches(entry),
            Rule::Incomplete { condition, .. } => condition.matches(entry),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, EnumIter)]
enum Operand {
    Plain,
    Not,
    And,
    Or,
}

#[derive(Serialize, Deserialize, Debug, Clone, EnumIter)]
enum Condition {
    Plain(Comparison),
    Not(Box<Condition>),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Incomplete {
        arg1: Option<Box<Condition>>,
        operand: Option<Operand>,
        arg2: Option<Box<Condition>>,
    },
}

impl Condition {
    fn incomplete() -> Self {
        Self::Incomplete {
            arg1: None,
            operand: None,
            arg2: None,
        }
    }
    fn matches(&self, entry: &DataRow) -> bool {
        match self {
            Self::Plain(comparison) => comparison.matches(entry),
            Self::Not(comparison) => !comparison.matches(entry),
            Self::And(c1, c2) => c1.matches(entry) && c2.matches(entry),
            Self::Or(c1, c2) => c1.matches(entry) || c2.matches(entry),
            Self::Incomplete { .. } => false,
        }
    }
    fn ui(
        &self,
        ui: &mut Ui,
        hovered_condition: &mut Option<Condition>,
        color: Color32,
    ) -> Response {
        match self {
            Condition::Plain(Comparison {
                field,
                ctype: ComparisonType::Contains(value),
            }) => {
                let response = ui.add(Label::new(
                    regular(format!("{field:?} contains \"{value}\"").as_str()).color(color),
                ));
                if response.hovered() {
                    *hovered_condition = Some(self.clone());
                }
                response
            }
            _ => {
                unimplemented!()
            }
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
