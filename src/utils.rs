// SPDX-FileCopyrightText: 2026 Kerstin Humm <kerstin@erictapen.name>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use egui::Ui;
use emath::Pos2;

fn interpolate(path: &mut Vec<Pos2>, start: &Pos2, end: &Pos2) {
    const STEP_SIZE: f32 = 4.0;
    let dist = start.distance(*end);
    for i in 0..((dist / STEP_SIZE) as i64) {
        let f: f32 = i as f32 / (dist / STEP_SIZE);
        path.push(emath::pos2(
            emath::lerp(start.x..=end.x, f),
            emath::lerp(start.y..=end.y, f),
        ));
    }
}

/// Take a RectShape and paint it with a dashed outline
///
/// TODO Doesn't respect stuff like stroke_kind or fill.
///
/// distortion is a function to do fancy things with the path before it's dashed
pub fn paint_dashed_rect_shape<F>(
    ui: &mut Ui,
    rect_shape: epaint::RectShape,
    dash_length: f32,
    gap_length: f32,
    mut distortion: F,
) where
    F: FnMut(&mut emath::Pos2) -> emath::Pos2,
{
    let mut path = Vec::new();
    let min = rect_shape.rect.min;
    let max = rect_shape.rect.max;
    let cr: epaint::CornerRadiusF32 = rect_shape.corner_radius.into();

    let mut se = Vec::new();
    epaint::tessellator::path::add_circle_quadrant(
        &mut se,
        emath::pos2(max.x - cr.se, max.y - cr.se),
        cr.se,
        0.0,
    );
    let mut sw = Vec::new();
    epaint::tessellator::path::add_circle_quadrant(
        &mut sw,
        emath::pos2(min.x + cr.sw, max.y - cr.sw),
        cr.sw,
        1.0,
    );
    let mut nw = Vec::new();
    epaint::tessellator::path::add_circle_quadrant(
        &mut nw,
        emath::pos2(min.x + cr.nw, min.y + cr.nw),
        cr.nw,
        2.0,
    );
    let mut ne = Vec::new();
    epaint::tessellator::path::add_circle_quadrant(
        &mut ne,
        emath::pos2(max.x - cr.ne, min.y + cr.ne),
        cr.ne,
        3.0,
    );

    path.append(&mut se);
    if let (Some(start), Some(end)) = (path.last().cloned(), sw.first()) {
        interpolate(&mut path, &start, end);
    }
    path.append(&mut sw);
    if let (Some(start), Some(end)) = (path.last().cloned(), nw.first()) {
        interpolate(&mut path, &start, end);
    }
    path.append(&mut nw);
    if let (Some(start), Some(end)) = (path.last().cloned(), ne.first()) {
        interpolate(&mut path, &start, end);
    }
    path.append(&mut ne);
    if let (Some(start), Some(end)) = (path.last().cloned(), path.first().cloned()) {
        interpolate(&mut path, &start, &end);
    }

    for pos in path.iter_mut() {
        *pos = distortion(pos);
    }
    for shape in epaint::Shape::dashed_line(&path, rect_shape.stroke, dash_length, gap_length) {
        ui.painter().add(shape);
    }
    // Shape::dashed_line doesn't connect the start and end point of the line…
    if let [first, .., last] = path[..] {
        for shape in
            epaint::Shape::dashed_line(&[last, first], rect_shape.stroke, dash_length, gap_length)
        {
            ui.painter().add(shape);
        }
    }
}

/// Helper to make it easier to accumulate a Option<Response> without an initial value
pub fn option_bitor_assign<T: std::ops::BitOrAssign>(opt: &mut Option<T>, value: T) {
    if let Some(existing) = opt.as_mut() {
        *existing |= value;
    } else {
        *opt = Some(value);
    }
}
