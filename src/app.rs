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

use std::fmt::Display;

use egui::text_edit::TextEdit;
use egui::{
    Align, Button, Color32, Context, Frame, Id, InnerResponse, Label, Layout, Margin, Rect,
    Response, RichText, StrokeKind, Ui, UiBuilder,
};
use epaint::{CornerRadius, RectShape};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use strum_macros::EnumIter;

use crate::widgets;

const THIN_SPACE: &str = "\u{2009}";
const GRIP_SYMBOL: &str = "⠿";
const CANCEL_SYMBOL: &str = "🗙";
const CHECK_SYMBOL: &str = "✓";
const TRASH_SYMBOL: &str = "🗑";

/// Time in seconds
const WARN_FADEOUT_TIME: f32 = 1.0;

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

#[cfg(feature = "egui_parley")]
fn regular_font_id(ui: &Ui) -> egui::text::style::FontId {
    egui::text::style::FontId::simple(
        egui::style::TextStyle::Body.resolve(ui.style()).size,
        FontFamily::Named("IBM Plex Sans".into()),
    )
}

#[cfg(feature = "egui_latest")]
fn regular_font_id(ui: &Ui) -> egui::FontId {
    egui::FontId::new(
        egui::style::TextStyle::Body.resolve(ui.style()).size,
        FontFamily::Name("IBM Plex Sans".into()),
    )
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

    let result: Vec<DataRow> = mt940::parse_mt940(&mt940::sanitizers::sanitize(
        &file_str.replace(":61:220229", ":61:220301"),
    ))
    .unwrap_or_else(|e| panic!("{}", e))
    .into_iter()
    .flat_map(DataRow::from_message)
    .collect();

    info!("Parsed {} MT940 rows", result.len());

    result
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
            true => "",
            false => "−",
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
    annotation: Annotation,
}

#[derive(Default)]
struct Annotation {
    derived: Option<Highlightable<String>>,
    manual: Option<String>,
}

impl Annotation {
    fn derived(str: String) -> Self {
        Self {
            derived: Some(Highlightable::new(str)),
            manual: Default::default(),
        }
    }
    /// The final value that is used for e.g. exporting
    fn category(&self) -> Option<String> {
        if let Some(derivedh) = &self.derived {
            Some(derivedh.inner.clone())
        } else if self.manual.is_some() {
            self.manual.as_ref().map(|manual| manual.to_string())
        } else {
            None
        }
    }
    fn ui(
        &mut self,
        ui: &mut Ui,
        known_categories: &indexmap::IndexSet<String>,
        minimap: &mut Minimap,
    ) {
        if self.manual.is_some() {
            if ui.add(Button::new(symbol(CANCEL_SYMBOL))).clicked() {
                self.manual = None;
            };
        }
        let rect = egui::ComboBox::from_label("Annotate by hand")
            .width(0.0)
            .icon(|_, _, _, _| {})
            .selected_text(regular(self.category().as_deref().unwrap_or("")))
            .show_ui(ui, |ui: &mut Ui| {
                for category in known_categories {
                    ui.selectable_value(
                        &mut self.manual,
                        Some(category.clone()),
                        regular(category),
                    );
                }
            })
            .response
            .rect;
        // derived takes precedence, so we strike through the label again
        if self.derived.is_some() && self.manual.is_some() {
            ui.painter().hline(
                rect.x_range(),
                rect.center().y,
                egui::Stroke::new(2.0, Color32::BLACK),
            );
        }
        if let Annotation {
            manual: Some(_),
            derived: Some(hstring),
            ..
        } = self
        {
            hstring.ui(ui, minimap);
        }
    }
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
                    annotation: Default::default(),
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

pub struct App {
    data: Arc<Mutex<Vec<DataRow>>>,
    /// Sync the scroll position so that the data and annotation table appear as one
    data_vertical_scroll_offset: f32,
    rules: Vec<Rule>,
    account_name: String,
    known_categories: indexmap::IndexSet<String>,
    hints: Vec<Hint>,
    minimap: Minimap,
}

impl Default for App {
    fn default() -> Self {
        Self {
            data: Default::default(),
            data_vertical_scroll_offset: 0.0,
            rules: Default::default(),
            account_name: "assets".to_string(),
            known_categories: Default::default(),
            hints: Default::default(),
            minimap: Default::default(),
        }
    }
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
        let mut result = Self {
            rules: vec![
                Rule::complete(
                    Condition::Plain(Comparison {
                        field: Field::Purpose,
                        ctype: ComparisonType::Contains,
                        value: "MINT".to_string(),
                    }),
                    "A".to_string(),
                ),
                Rule::complete(
                    Condition::Plain(Comparison {
                        field: Field::Purpose,
                        ctype: ComparisonType::Contains,
                        value: "spezifiziert".to_string(),
                    }),
                    "B".to_string(),
                ),
                Rule::complete(
                    Condition::Plain(Comparison {
                        field: Field::Purpose,
                        ctype: ComparisonType::Contains,
                        value: "Buchung".to_string(),
                    }),
                    "C".to_string(),
                ),
            ],
            data: Arc::new(Mutex::new(parse_mt940_file(
                // This example file is from
                // https://github.com/svenstaro/mt940-rs/blob/29b547fb062de34cd8f39e9adff9d80dfa64dbdd/tests/data/mt940/full/betterplace/sepa_mt9401.sta
                include_bytes!("../sample_data/mt940.sta"),
            ))),
            ..Default::default()
        };
        #[cfg(not(feature = "demo"))]
        let mut result: App = Default::default();
        result.update_annotations();
        result
    }
    fn file_load_button(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        let widget = |ui: &mut Ui| {
            Frame::NONE.inner_margin(Margin::same(120)).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                ui.add(Label::new(regular("TODO processes bookkeeping transactions from your bank account.\nDrag a MT940 file here or")));
                let button_response = ui.button(bold("pick one."));
                if button_response.clicked() {
                    let task = rfd::AsyncFileDialog::new().pick_file();
                    let data_clone = Arc::clone(&self.data);
                    let ctx_clone = ctx.clone();
                    execute(async move {
                        let file = task.await;
                        if let Some(file) = file {
                            let file_content = file.read().await;
                            let parsed = parse_mt940_file(&file_content);

                            let mut data = data_clone.lock().unwrap();
                            *data = parsed;
                            App::request_update_annotations(&ctx_clone);
                            // Redraw so the user can see the result of file load even when window
                            // isn't active.
                            ctx_clone.request_repaint();
                        }
                    });
                }
                });
            });
        };
        let widget_size = ui
            .scope_builder(UiBuilder::new().invisible(), widget)
            .response
            .rect
            .size();
        let available_size = ui.max_rect().size();
        // Where we'll paint the widget at
        let target_rect = Rect::from_min_size(
            [
                available_size.x / 2.0 - widget_size.x / 2.0,
                available_size.y / 2.0 - widget_size.y / 2.0,
            ]
            .into(),
            widget_size,
        );
        let (hovering, dropped_file) = ctx.input(|i| {
            (
                !i.raw.hovered_files.is_empty(),
                i.raw.dropped_files.first().cloned(),
            )
        });

        if let Some(dropped_file) = dropped_file {
            let mut file_content = Vec::new();

            #[cfg(target_arch = "wasm32")]
            if let Some(bytes) = dropped_file.bytes {
                file_content = bytes.to_vec();
            }

            #[cfg(not(target_arch = "wasm32"))]
            if let Some(path) = dropped_file.path {
                file_content = std::fs::read(&path).expect("Couldn't read file");
            }

            let parsed = parse_mt940_file(&file_content);

            let mut data = self.data.lock().unwrap();
            *data = parsed;
        }

        ui.scope_builder(UiBuilder::new().max_rect(target_rect), |ui| {
            let animation_time = 1.0;
            let animate_factor =
                ctx.animate_bool_with_time("file_load_wave".into(), hovering, animation_time);
            if animate_factor > 0.0 {
                // Show animation
                ctx.request_repaint();
            }
            let intensity = 0.05;
            let period = 10.0;
            let speed = 0.1;
            let time = (ctx.input(|i| i.time) % std::f64::consts::TAU) as f32;
            crate::utils::paint_dashed_rect_shape(
                ui,
                RectShape::stroke(
                    target_rect,
                    CornerRadius::same(50),
                    if hovering {
                        egui::Stroke::new(4.0, Color32::DARK_GRAY)
                    } else {
                        egui::Stroke::new(4.0, Color32::GRAY)
                    },
                    StrokeKind::Middle,
                ),
                10.0,
                10.0,
                |pos| {
                    let center_to_pos = *pos - target_rect.center();
                    *pos + animate_factor
                        * intensity
                        * (((center_to_pos.angle() + (speed * time)) * period).sin())
                        * center_to_pos
                },
            );

            // Paint the inside
            widget(ui);
        });
    }
    /// Annotate data rows
    /// The idea is to not run this every frame
    fn update_annotations(&mut self) {
        for entry in &mut *self.data.lock().unwrap() {
            entry.annotation = Default::default();
            for rule in &self.rules {
                // We only use complete rules for annotation
                if let Rule::Complete { category, .. } = rule {
                    if rule.matches(entry) {
                        entry.annotation = Annotation::derived(category.clone());
                    }
                }
            }
        }
        self.known_categories.clear();
        for rule in &self.rules {
            if let Rule::Complete { category, .. } = rule {
                self.known_categories.insert(category.clone());
            }
        }
    }
    /// Request to update_annotations
    fn request_update_annotations(ctx: &egui::Context) {
        ctx.data_mut(|d| {
            d.insert_temp("update_annotations".into(), true);
        });
    }
    fn minimap(&mut self, ctx: &egui::Context, ui: &mut Ui, row_height: f32) {
        if ctx.input(|i| i.screen_rect().width()) > 600.0 {
            egui::SidePanel::right("minimap")
                .resizable(false)
                .exact_width(200.0)
                .frame(egui::Frame::NONE.fill(ui.visuals().panel_fill))
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
                            // Expand the lines so that the gaps are closed
                            rect_shape.rect = rect_shape
                                .rect
                                .expand2([0.0, (row_height - rect_shape.rect.height())].into());
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

                    ui.with_layout(Layout::bottom_up(egui::Align::Min), |ui| {
                        if ui
                            .add_sized(
                                [ui.available_width(), 0.0],
                                Button::new(
                                    regular("Clear data and load new file").color(Color32::WHITE),
                                )
                                .fill(Color32::BLUE),
                            )
                            .clicked()
                        {
                            self.data.lock().unwrap().clear();
                        }

                        ui.add_space(ui.available_height());
                    });
                });
        }
        // Clear state so that we can write down elements again
        self.minimap.clear();
    }
    fn data_panel(&mut self, ctx: &egui::Context) {
        use egui_extras::{Column, TableBuilder};

        let data_panel_response = egui::TopBottomPanel::top("data_panel")
            .resizable(true)
            .min_height(TRANSACTIONS_HEIGHT_MIN)
            .exact_height(ctx.screen_rect().max.y * 0.5)
            .default_height(ctx.screen_rect().max.y * 0.5)
            .show(ctx, |ui| {
                if self.data.lock().unwrap().is_empty() {
                    self.file_load_button(ctx, ui);
                } else {
                    // Rows should be exactly as high as a button, as that is currently the limiting factor
                    // TODO make this not allocate space
                    let row_height = ui
                        .scope_builder(UiBuilder::new().invisible(), |ui| ui.button(symbol("🗙")))
                        .response
                        .rect
                        .height();

                    // Predraw a shadow that's going to indicate that the data table isn't scrolled
                    // entirely to the right
                    if let Some(edge) = ctx.data(|d| {
                        d.get_temp::<f32>("annotations_edge_for_blur".into())
                    }) {
                        info!("edge: {edge}");
                        let rect = Rect::from_min_max([edge, -171.8].into(), [2338.0, 708.5].into());
                        let shape = Frame::NONE.shadow(egui::Shadow {offset: [-10, 0], blur: 20, spread: 0, color: Color32::LIGHT_GRAY}).paint(rect);
                        ui.painter().add(shape);
                    }

                    self.minimap(ctx, ui, row_height);

                    let mut offset_annotations = 0.0;
                    let mut offset_data = 0.0;

                    let sidepanel_response = egui::SidePanel::right("annotations_sidepanel")
                        .resizable(false)
                        .min_width(200.0)
                        .frame(Frame::NONE.fill(ui.visuals().panel_fill).inner_margin(5.0))
                        .show_inside(ui, |ui| {
                            let annotations_response = TableBuilder::new(ui)
                                .vertical_scroll_offset(self.data_vertical_scroll_offset)
                                // Optional: Hide when data_panel_response doesn't indicate hover
                                .scroll_bar_visibility(egui::containers::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                                .auto_shrink([false, false])
                            .striped(true)
                                .column(Column::auto())
                                .header(20.0, |mut header| {
                                    header.col(|ui| {
                                        ui.label(bold("annotation"));
                                    });
                                })
                                .body(|mut body| {
                                    for entry in &mut *self.data.lock().unwrap() {
                                        body.row(row_height, |mut row| {
                                            // annotation
                                            row.col(|ui| {
                                                ui.horizontal(|ui| {
                                                    if entry.money.inner.credit {
                                                        entry.annotation.ui(
                                                            ui,
                                                            &self.known_categories,
                                                            &mut self.minimap,
                                                        );
                                                        ui.label(regular(&format!(
                                                            "→ {}",
                                                            self.account_name
                                                        )));
                                                    } else {
                                                        ui.label(regular(&format!(
                                                            "{} →",
                                                            self.account_name
                                                        )));
                                                        entry.annotation.ui(
                                                            ui,
                                                            &self.known_categories,
                                                            &mut self.minimap,
                                                        );
                                                    }
                                                });
                                            });
                                        });
                                    }
                                });
                                offset_annotations = annotations_response.state.offset.y;

                        });


                    let horizontal_output = egui::ScrollArea::horizontal().show(ui, |ui| {
                        let table_state = TableBuilder::new(ui)
                            .vertical_scroll_offset(self.data_vertical_scroll_offset)
                            // Let the annotations table show a scrollbar instead
                            .scroll_bar_visibility(egui::containers::scroll_area::ScrollBarVisibility::AlwaysHidden)
                            .auto_shrink([false, false])
                            .striped(true)
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
                                for entry in &mut *self.data.lock().unwrap() {
                                    body.row(row_height, |mut row| {
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
                                    });
                                }
                                self.minimap.frame =
                                    Some(body.ui_mut().min_rect()).filter(|r| r.width() != 0.0);
                            });
                            offset_data = table_state.state.offset.y;

                            if self.data_vertical_scroll_offset != offset_data {
                               self.data_vertical_scroll_offset = offset_data;
                            } else if self.data_vertical_scroll_offset != offset_annotations {
                               self.data_vertical_scroll_offset = offset_annotations;
                            }

                        // For visualising which part of the minimap is visible in the data panel,
                        // we add another shape to signify scroll state
                        self.minimap.visible_rect = Some(table_state.inner_rect);
                    });

                    // If there was a need for scrolling, we render the shadow before everything
                    // next frame
                    if horizontal_output.content_size.x > horizontal_output.inner_rect.width() {
                      ctx.data_mut(|d| {
                          d.insert_temp("annotations_edge_for_blur".into(), sidepanel_response.response.rect.min.x);
                      });
                    } else {
                      ctx.data_mut(|d| {
                          d.remove_temp::<f32>("annotations_edge_for_blur".into());
                      });
                    }

                    if let Some(ref mut visible_rect) = self.minimap.visible_rect {
                        *visible_rect = visible_rect.intersect(horizontal_output.inner_rect);
                    }
                }
            });
        if data_panel_response.response.hovered() {
            self.hints.push(Hint::Transactions);
        }
    }
    fn rules_panel(&mut self, ctx: &egui::Context) {
        let mut a_rule_changed = false;
        // Keep track if there is any rule being edited
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

                    // Delete rules marked as to be deleted
                    self.rules.retain(|r| !r.to_delete());

                    for (i, rule) in self.rules.iter_mut().enumerate() {
                        match *rule {
                            Rule::Complete {
                                enabled, dragged, ..
                            } => {
                                let old_enabled = enabled;
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
                                if dragged == ButtonState::Active {
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
                            Rule::Incomplete { .. } => {
                                a_rule_is_being_edited = true;
                                rule.ui(ui, &mut hovered_condition, i);
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
                        && ui.add(Button::new(regular("+ Add new rule"))).clicked()
                    {
                        info!("Pushed");
                        self.rules
                            .push(Rule::incomplete(Condition::incomplete(), "".to_string()));
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

        // We allow setting a flag to request an update to annotations from e.g. a closure.
        if ctx.data(|d| {
            d.get_temp::<bool>("update_annotations".into())
                .unwrap_or(false)
        }) {
            self.update_annotations();
            ctx.data_mut(|d| {
                d.insert_temp("update_annotations".into(), false);
            });
        }

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
        dragged: ButtonState,
        delete: ButtonState,
    },
    Incomplete {
        condition: Condition,
        category: String,
        /// Try to complete this rule next frame
        try_to_complete: bool,
    },
}

impl Rule {
    fn complete(condition: Condition, category: String) -> Self {
        Self::Complete {
            enabled: true,
            condition,
            category,
            hovered: false,
            dragged: Default::default(),
            delete: Default::default(),
        }
    }
    fn incomplete(condition: Condition, category: String) -> Self {
        Self::Incomplete {
            condition,
            category,
            try_to_complete: false,
        }
    }
    fn try_into_complete(&self) -> Option<Self> {
        if let Self::Incomplete {
            condition,
            category,
            ..
        } = self
        {
            if category.is_empty() {
                return None;
            }
            condition
                .try_into_complete()
                .map(|c| Self::complete(c, category.to_string()))
        } else {
            None
        }
    }
    fn to_delete(&self) -> bool {
        matches!(
            self,
            Self::Complete {
                delete: ButtonState::Active,
                ..
            }
        )
    }
    fn ui(
        &mut self,
        ui: &mut Ui,
        hovered_condition: &mut Option<Condition>,
        rule_i: usize,
    ) -> InnerResponse<Response> {
        let completed_rule = if let Self::Incomplete {
            try_to_complete: true,
            ..
        } = self
        {
            // Try to convert into Rule::Complete
            self.clone().try_into_complete()
        } else {
            None
        };

        let response = match *self {
            Self::Incomplete {
                ref mut category,
                ref mut condition,
                ref mut try_to_complete,
                ..
            } => {
                ui.horizontal_top(|ui| {
                    let mut r = ui.add(Label::new(regular("When")));
                    r |= condition.ui(ui, *try_to_complete, &mut None, Color32::BLACK);
                    r |= ui.add(Label::new(regular("then mark as")));
                    let text_edit_category_response =
                        ui.add(TextEdit::singleline(category).font(regular_font_id(ui)));
                    if category.is_empty() {
                        missing_value_indicator(
                            ui,
                            text_edit_category_response.rect,
                            "missing_category".into(),
                            *try_to_complete,
                        );
                    }
                    r |= text_edit_category_response;

                    // In case anything is missing for completion
                    let (missing_values_text_bg_color, missing_values_text_color) = (
                        animate_color_pulse(
                            ui.ctx(),
                            "missing_values_text_bg".into(),
                            *try_to_complete,
                            Color32::LIGHT_RED,
                            WARN_FADEOUT_TIME,
                        ),
                        animate_color_pulse(
                            ui.ctx(),
                            "missing_values_text".into(),
                            *try_to_complete,
                            Color32::BLACK,
                            4.0 * WARN_FADEOUT_TIME,
                        ),
                    );

                    *try_to_complete = false;

                    let complete_button_response = ui.button(symbol(CHECK_SYMBOL));
                    if complete_button_response.clicked() {
                        *try_to_complete = true;
                    }
                    if completed_rule.is_none() && missing_values_text_color != Color32::TRANSPARENT
                    {
                        r |= ui.add(Label::new(
                            regular("Fill out all fields before creating the rule")
                                .background_color(missing_values_text_bg_color)
                                .color(missing_values_text_color),
                        ));
                    }
                    r |= complete_button_response;
                    r
                })
            }

            Rule::Complete {
                ref mut enabled,
                ref mut dragged,
                ref mut hovered,
                ref mut condition,
                ref mut delete,
                ref category,
                ..
            } => {
                let response = ui.horizontal_top(|ui| {
                    // When being dragged we render the rule in a more subtle color
                    let color = match (&*enabled, &*dragged) {
                        (false, _) | (_, ButtonState::Active) => Color32::GRAY,
                        _ => Color32::PLACEHOLDER,
                    };

                    // grip handle for drag & drop
                    let mut r = {
                        let color = if *dragged == ButtonState::Hovered {
                            Color32::BLACK
                        } else if *hovered {
                            Color32::DARK_GRAY
                        } else {
                            Color32::TRANSPARENT
                        };
                        let grip_response = ui.add(Label::new(symbol(GRIP_SYMBOL).color(color)));
                        grip_response.dnd_set_drag_payload(rule_i);
                        if grip_response.dragged() {
                            *dragged = ButtonState::Active;
                        } else {
                            *dragged = ButtonState::from_response(&grip_response);
                        }
                        grip_response
                    };

                    // delete button
                    {
                        let button_color = if delete == &ButtonState::Hovered {
                            Color32::BLACK
                        } else if *hovered {
                            Color32::DARK_GRAY
                        } else {
                            Color32::TRANSPARENT
                        };
                        let delete_response = ui.add(
                            Button::new(symbol(TRASH_SYMBOL).color(button_color)).frame(false),
                        );
                        *delete = ButtonState::from_response(&delete_response);
                        r |= delete_response;
                    }
                    r |= ui.add(widgets::toggle_switch::toggle(enabled));
                    let mut rule_text = ui.add(Label::new(regular("When").color(color)));
                    rule_text |= condition.ui(ui, false, hovered_condition, color);
                    rule_text |= ui.add(Label::new(regular("then mark as").color(color)));
                    rule_text |= ui.add(Label::new(regular(category).color(color)));
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
        };

        if let Some(completed_rule) = completed_rule {
            *self = completed_rule;
            App::request_update_annotations(ui.ctx());
        }

        response
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

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Copy)]
enum ButtonState {
    #[default]
    None,
    Hovered,
    Active,
}

impl ButtonState {
    fn from_response(response: &Response) -> Self {
        if response.clicked() {
            ButtonState::Active
        } else if response.hovered() {
            ButtonState::Hovered
        } else {
            ButtonState::None
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
        field: Option<Field>,
        ctype: Option<ComparisonType>,
        value: String,
    },
}

impl Condition {
    fn incomplete() -> Self {
        Self::Incomplete {
            field: None,
            ctype: None,
            value: String::new(),
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
    fn try_into_complete(&self) -> Option<Self> {
        match self {
            Condition::Incomplete {
                field: Some(field),
                ctype: Some(ctype),
                value,
            } => {
                if value.is_empty() {
                    None
                } else {
                    Some(Self::Plain(Comparison {
                        field: field.clone(),
                        ctype: ctype.clone(),
                        value: value.to_string(),
                    }))
                }
            }
            Condition::Incomplete { .. } => None,
            c => Some(c.clone()),
        }
    }
    /// try_to_complete: only relevant for Incomplete rules
    fn ui(
        &mut self,
        ui: &mut Ui,
        try_to_complete: bool,
        hovered_condition: &mut Option<Condition>,
        color: Color32,
    ) -> Response {
        match self {
            Self::Plain(Comparison {
                field,
                ctype,
                value,
            }) => {
                let response = ui.add(Label::new(
                    regular(format!("{field} {ctype} \"{value}\"").as_str()).color(color),
                ));
                if response.hovered() {
                    *hovered_condition = Some(self.clone());
                }
                response
            }
            Self::Incomplete {
                field,
                ctype,
                value,
                ..
            } => {
                let mut r;

                // field
                {
                    let mut set_field_to_none = false;
                    r = match field {
                        None => {
                            let r = ui
                                .vertical(|ui| {
                                    if ui.add(Button::new(regular("Name"))).clicked() {
                                        *field = Some(Field::Name);
                                    }
                                    if ui.add(Button::new(regular("Purpose"))).clicked() {
                                        *field = Some(Field::Purpose);
                                    }
                                    if ui.add(Button::new(regular("IBAN"))).clicked() {
                                        *field = Some(Field::Iban);
                                    }
                                })
                                .response;
                            missing_value_indicator(
                                ui,
                                r.rect,
                                "missing_field".into(),
                                try_to_complete,
                            );
                            r
                        }
                        Some(selected_field) => {
                            let mut r = ui.add(Button::new(symbol(CANCEL_SYMBOL)));
                            if r.clicked() {
                                set_field_to_none = true;
                            }
                            r |= ui.add(Label::new(regular(format!("{selected_field}").as_str())));
                            r
                        }
                    };
                    if set_field_to_none {
                        *field = None;
                    }
                }

                // ctype
                {
                    let mut set_ctype_to_none = false;
                    r |= match ctype {
                        None => {
                            let r = ui
                                .vertical(|ui| {
                                    if ui
                                        .button(regular(
                                            format!("{}", ComparisonType::Contains).as_str(),
                                        ))
                                        .clicked()
                                    {
                                        *ctype = Some(ComparisonType::Contains);
                                    }
                                    if ui
                                        .button(regular(
                                            format!("{}", ComparisonType::Exact).as_str(),
                                        ))
                                        .clicked()
                                    {
                                        *ctype = Some(ComparisonType::Exact);
                                    }
                                })
                                .response;
                            missing_value_indicator(
                                ui,
                                r.rect,
                                "missing_ctype".into(),
                                try_to_complete,
                            );
                            r
                        }
                        Some(selected_ctype) => {
                            let mut r = ui.add(Button::new(symbol(CANCEL_SYMBOL)));
                            if r.clicked() {
                                set_ctype_to_none = true;
                            }
                            r |= ui.add(Label::new(regular(format!("{selected_ctype}").as_str())));
                            r
                        }
                    };
                    if set_ctype_to_none {
                        *ctype = None;
                    }
                }

                // value
                {
                    let text_edit_response =
                        ui.add(TextEdit::singleline(&mut (*value)).font(regular_font_id(ui)));
                    if value.is_empty() {
                        missing_value_indicator(
                            ui,
                            text_edit_response.rect,
                            "missing_value".into(),
                            try_to_complete,
                        );
                    }
                    r |= text_edit_response;
                }
                r
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

fn animate_color_pulse(
    ctx: &Context,
    id: Id,
    send_pulse: bool,
    color: Color32,
    time_in_seconds: f32,
) -> Color32 {
    let current_time = ctx.input(|i| i.time) as f32;
    if send_pulse {
        ctx.data_mut(|d| {
            d.insert_temp(id, current_time);
        });
    }
    if let Some(time_of_last_fail) = ctx.data_mut(|d| d.get_temp::<f32>(id)) {
        ctx.request_repaint();
        let time_since_last_fail = current_time - time_of_last_fail;
        let warn_factor = 1.0 - (time_since_last_fail / time_in_seconds);
        if warn_factor > 0.0 {
            Color32::TRANSPARENT.gamma_multiply(1.0 - warn_factor)
                + color.gamma_multiply(warn_factor)
        } else {
            ctx.data_mut(|d| {
                d.remove_temp::<f32>(id);
            });
            Color32::TRANSPARENT
        }
    } else {
        Color32::TRANSPARENT
    }
}

/// When a condition is missing a value and the user want's to complete the Rule, we can show a
/// visual warning which values are missing.
fn missing_value_indicator(ui: &mut Ui, rect: Rect, id: Id, try_to_complete: bool) {
    ui.painter().add(epaint::RectShape::filled(
        rect,
        CornerRadius::default(),
        animate_color_pulse(
            ui.ctx(),
            id,
            try_to_complete,
            Color32::LIGHT_RED,
            WARN_FADEOUT_TIME,
        ),
    ));
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Comparison {
    field: Field,
    ctype: ComparisonType,
    value: String,
}

impl Comparison {
    fn matches(&self, entry: &DataRow) -> bool {
        let data = match (&self.field, entry) {
            (
                Field::Name,
                DataRow {
                    name: Some(Highlightable { inner: name, .. }),
                    ..
                },
            ) => name,
            (
                Field::Purpose,
                DataRow {
                    purpose: Some(Highlightable { inner: purpose, .. }),
                    ..
                },
            ) => purpose,
            (
                Field::Iban,
                DataRow {
                    iban: Some(Highlightable { inner: iban, .. }),
                    ..
                },
            ) => &iban.0,
            _ => {
                return false;
            }
        };
        match self.ctype {
            ComparisonType::Contains => data.contains(&self.value),
            ComparisonType::Exact => *data == self.value,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
enum ComparisonType {
    #[default]
    Exact,
    Contains,
}

impl Display for ComparisonType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exact => write!(f, "is exactly"),
            Self::Contains => write!(f, "contains"),
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

impl Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Name => write!(f, "Name"),
            Self::Purpose => write!(f, "Purpose"),
            Self::Iban => write!(f, "IBAN"),
        }
    }
}
