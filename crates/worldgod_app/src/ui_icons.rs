//! Векторные иконки, нарисованные кодом.
//!
//! Никаких PNG-ассетов: иконки строятся из простых фигур, поэтому выглядят
//! резко на любом DPI и легко перекрашиваются под состояние кнопки.

use eframe::egui;
use egui::{Color32, CornerRadius, Pos2, Rect, Stroke};
use worldgod_core::brush::Tool;
use worldgod_core::sim::Speed;
use worldgod_core::weather::Weather;

/// Иконка инструмента.
pub fn tool_icon(painter: &egui::Painter, rect: Rect, tool: Tool, color: Color32) {
    match tool {
        Tool::Raise => raise_icon(painter, rect, color),
        Tool::Lower => water_icon(painter, rect, color),
        Tool::Forest => forest_icon(painter, rect, color),
        Tool::Sand => sand_icon(painter, rect, color),
        Tool::Mountain => mountain_icon(painter, rect, color),
        Tool::Human => human_icon(painter, rect, color),
        Tool::Soldier => soldier_icon(painter, rect, color),
        Tool::Deer => deer_icon(painter, rect, color),
        Tool::Wolf => wolf_icon(painter, rect, color),
        Tool::Zombie => zombie_icon(painter, rect, color),
        Tool::Inspect => inspect_icon(painter, rect, color),
        Tool::Diplomacy => diplomacy_icon(painter, rect, color),
        Tool::Hurricane | Tool::SolarFlare | Tool::Rain | Tool::Snowfall => {
            let weather = tool.weather().unwrap_or(Weather::Rain);
            weather_icon(painter, rect, weather, color);
        }
    }
}

/// Иконка стихии.
pub fn weather_icon(painter: &egui::Painter, rect: Rect, weather: Weather, color: Color32) {
    match weather {
        Weather::Hurricane => hurricane_icon(painter, rect, color),
        Weather::SolarFlare => solar_flare_icon(painter, rect, color),
        Weather::Rain => rain_icon(painter, rect, color),
        Weather::Snowfall => snowfall_icon(painter, rect, color),
    }
}

/// Ураган: спираль ветра.
fn hurricane_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = point(rect, 0.5, 0.5);
    let radius = rect.width().min(rect.height());
    let mut points = Vec::with_capacity(26);
    for step in 0..26 {
        let t = step as f32 / 25.0;
        let angle = t * std::f32::consts::TAU * 1.6;
        let r = radius * (0.42 - 0.34 * t);
        points.push(egui::pos2(
            center.x + angle.cos() * r,
            center.y + angle.sin() * r,
        ));
    }
    painter.add(egui::Shape::line(points, Stroke::new(2.0_f32, color)));
    painter.circle_filled(center, radius * 0.06, color);
}

/// Вспышка солнца: диск с лучами.
fn solar_flare_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    disc(painter, rect, 0.5, 0.5, 0.17, color);
    for ray in 0..8 {
        let angle = ray as f32 * std::f32::consts::TAU / 8.0;
        let inner = 0.28;
        let outer = if ray % 2 == 0 { 0.46 } else { 0.38 };
        let (sin, cos) = angle.sin_cos();
        stroke_path(
            painter,
            rect,
            &[
                (0.5 + cos * inner, 0.5 + sin * inner),
                (0.5 + cos * outer, 0.5 + sin * outer),
            ],
            2.0,
            color,
        );
    }
}

/// Дождь: облако и капли.
fn rain_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    disc(painter, rect, 0.38, 0.36, 0.17, color);
    disc(painter, rect, 0.6, 0.4, 0.14, color);
    fill(
        painter,
        rect,
        &[(0.2, 0.46), (0.82, 0.46), (0.78, 0.56), (0.24, 0.56)],
        color,
    );
    for drop in 0..3 {
        let x = 0.3 + drop as f32 * 0.2;
        stroke_path(painter, rect, &[(x, 0.64), (x - 0.04, 0.86)], 1.8, color);
    }
}

/// Снегопад: облако и снежинки.
fn snowfall_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    disc(painter, rect, 0.38, 0.34, 0.16, color);
    disc(painter, rect, 0.6, 0.38, 0.13, color);
    fill(
        painter,
        rect,
        &[(0.2, 0.44), (0.82, 0.44), (0.78, 0.54), (0.24, 0.54)],
        color,
    );
    for flake in 0..3 {
        let x = 0.28 + flake as f32 * 0.22;
        let y = 0.68 + (flake % 2) as f32 * 0.1;
        stroke_path(painter, rect, &[(x - 0.06, y), (x + 0.06, y)], 1.5, color);
        stroke_path(painter, rect, &[(x, y - 0.08), (x, y + 0.08)], 1.5, color);
    }
}

/// Иконка «Война / Мир»: два скрещённых флажка на древках.
fn diplomacy_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    // Древко слева и флажок над ним.
    stroke_path(painter, rect, &[(0.34, 0.2), (0.34, 0.82)], 1.6, color);
    fill(painter, rect, &[(0.34, 0.24), (0.66, 0.32), (0.34, 0.44)], color);
    // Древко справа — с флажком другого цвета (приглушённым).
    let dim = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 150);
    stroke_path(painter, rect, &[(0.66, 0.2), (0.66, 0.82)], 1.6, dim);
    fill(painter, rect, &[(0.66, 0.48), (0.34, 0.56), (0.66, 0.68)], dim);
}

/// Иконка скорости времени.
pub fn speed_icon(painter: &egui::Painter, rect: Rect, speed: Speed) {
    let color = crate::theme::ACCENT;
    match speed {
        Speed::Paused => {
            fill(painter, rect, &[(0.34, 0.24), (0.78, 0.5), (0.34, 0.76)], color);
        }
        Speed::Normal => {
            fill(painter, rect, &[(0.26, 0.26), (0.6, 0.5), (0.26, 0.74)], color);
        }
        Speed::Fast => {
            fill(painter, rect, &[(0.2, 0.26), (0.48, 0.5), (0.2, 0.74)], color);
            fill(painter, rect, &[(0.5, 0.26), (0.78, 0.5), (0.5, 0.74)], color);
        }
        Speed::Turbo => {
            fill(painter, rect, &[(0.14, 0.26), (0.38, 0.5), (0.14, 0.74)], color);
            fill(painter, rect, &[(0.38, 0.26), (0.62, 0.5), (0.38, 0.74)], color);
            fill(painter, rect, &[(0.62, 0.26), (0.86, 0.5), (0.62, 0.74)], color);
        }
    }
}

/// Иконка «новая планета».
pub fn planet_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = point(rect, 0.5, 0.5);
    let radius = rect.width().min(rect.height()) * 0.28;
    painter.circle_stroke(center, radius, Stroke::new(1.6_f32, color));
    stroke_path(
        painter,
        rect,
        &[(0.12, 0.58), (0.36, 0.5), (0.64, 0.5), (0.88, 0.42)],
        1.6,
        color,
    );
    painter.circle_filled(point(rect, 0.4, 0.38), radius * 0.22, color);
}

/// Иконка меню.
pub fn menu_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    for y in [0.34_f32, 0.5, 0.66] {
        stroke_path(
            painter,
            rect,
            &[(0.28, y), (0.72, y)],
            2.0,
            color,
        );
    }
}

fn raise_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    fill(
        painter,
        rect,
        &[(0.1, 0.82), (0.42, 0.5), (0.7, 0.82)],
        color,
    );
    stroke_path(
        painter,
        rect,
        &[(0.74, 0.62), (0.74, 0.24)],
        2.0,
        color,
    );
    stroke_path(
        painter,
        rect,
        &[(0.62, 0.36), (0.74, 0.22), (0.86, 0.36)],
        2.0,
        color,
    );
}

fn water_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    for (index, y) in [0.36_f32, 0.52, 0.68].iter().enumerate() {
        let offset = index as f32 * 0.06;
        stroke_path(
            painter,
            rect,
            &[
                (0.12, y + offset),
                (0.28, y - 0.08 + offset),
                (0.44, y + offset),
                (0.6, y - 0.08 + offset),
                (0.76, y + offset),
                (0.88, y - 0.05 + offset),
            ],
            2.0,
            color,
        );
    }
}

fn forest_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    bar(painter, rect, 0.46, 0.66, 0.08, 0.2, color);
    fill(
        painter,
        rect,
        &[(0.5, 0.16), (0.78, 0.52), (0.22, 0.52)],
        color,
    );
    fill(
        painter,
        rect,
        &[(0.5, 0.4), (0.88, 0.7), (0.12, 0.7)],
        color,
    );
}

fn sand_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    fill(
        painter,
        rect,
        &[(0.08, 0.78), (0.34, 0.56), (0.56, 0.72), (0.78, 0.5), (0.94, 0.78)],
        color,
    );
    for (x, y) in [(0.26_f32, 0.32_f32), (0.5, 0.22), (0.72, 0.32)] {
        disc(painter, rect, x, y, 0.05, color);
    }
}

fn mountain_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    fill(
        painter,
        rect,
        &[(0.06, 0.82), (0.4, 0.24), (0.72, 0.82)],
        color,
    );
    fill(
        painter,
        rect,
        &[(0.5, 0.82), (0.72, 0.46), (0.96, 0.82)],
        color,
    );
    fill(
        painter,
        rect,
        &[(0.29, 0.46), (0.4, 0.24), (0.51, 0.46), (0.42, 0.42), (0.36, 0.5)],
        Color32::WHITE,
    );
}

fn human_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    disc(painter, rect, 0.5, 0.26, 0.13, color);
    fill(
        painter,
        rect,
        &[(0.34, 0.86), (0.5, 0.44), (0.66, 0.86)],
        color,
    );
    fill(
        painter,
        rect,
        &[(0.3, 0.44), (0.7, 0.44), (0.66, 0.6), (0.34, 0.6)],
        color,
    );
}

fn soldier_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    // Шлем: купол и козырёк.
    disc(painter, rect, 0.42, 0.24, 0.14, color);
    fill(
        painter,
        rect,
        &[(0.26, 0.32), (0.58, 0.32), (0.56, 0.42), (0.28, 0.42)],
        color,
    );
    // Плечи и корпус.
    fill(
        painter,
        rect,
        &[(0.22, 0.92), (0.34, 0.46), (0.5, 0.46), (0.56, 0.92)],
        color,
    );
    // Щит справа — по нему воина и узнаём.
    fill(
        painter,
        rect,
        &[
            (0.58, 0.46),
            (0.9, 0.46),
            (0.9, 0.72),
            (0.74, 0.92),
            (0.58, 0.72),
        ],
        color,
    );
    stroke_path(
        painter,
        rect,
        &[(0.64, 0.68), (0.84, 0.68)],
        2.0,
        DETAIL,
    );
    stroke_path(
        painter,
        rect,
        &[(0.74, 0.54), (0.74, 0.84)],
        2.0,
        DETAIL,
    );
}

fn deer_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    // Ветвистые рога — главный признак оленя.
    stroke_path(painter, rect, &[(0.44, 0.44), (0.34, 0.12)], 2.0, color);
    stroke_path(painter, rect, &[(0.38, 0.28), (0.2, 0.2)], 1.8, color);
    stroke_path(painter, rect, &[(0.56, 0.44), (0.66, 0.12)], 2.0, color);
    stroke_path(painter, rect, &[(0.62, 0.28), (0.8, 0.2)], 1.8, color);
    // Уши.
    fill(
        painter,
        rect,
        &[(0.28, 0.56), (0.42, 0.4), (0.44, 0.62)],
        color,
    );
    fill(
        painter,
        rect,
        &[(0.72, 0.56), (0.58, 0.4), (0.56, 0.62)],
        color,
    );
    // Морда.
    fill(
        painter,
        rect,
        &[(0.34, 0.62), (0.66, 0.62), (0.58, 0.92), (0.42, 0.92)],
        color,
    );
    disc(painter, rect, 0.42, 0.68, 0.05, DETAIL);
    disc(painter, rect, 0.58, 0.68, 0.05, DETAIL);
}

fn wolf_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    // Уши отдельными треугольниками: общий силуэт с вырезами
    // рисовался бы как выпуклая фигура без ушей.
    fill(
        painter,
        rect,
        &[(0.18, 0.14), (0.42, 0.42), (0.24, 0.48)],
        color,
    );
    fill(
        painter,
        rect,
        &[(0.82, 0.14), (0.58, 0.42), (0.76, 0.48)],
        color,
    );
    // Голова.
    fill(
        painter,
        rect,
        &[
            (0.26, 0.44),
            (0.74, 0.44),
            (0.68, 0.7),
            (0.5, 0.78),
            (0.32, 0.7),
        ],
        color,
    );
    // Морда.
    fill(
        painter,
        rect,
        &[(0.42, 0.72), (0.58, 0.72), (0.5, 0.94)],
        color,
    );
    disc(painter, rect, 0.38, 0.56, 0.055, DETAIL);
    disc(painter, rect, 0.62, 0.56, 0.055, DETAIL);
}

fn zombie_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    // Сгорбленная фигура с вытянутыми вперёд руками.
    disc(painter, rect, 0.3, 0.24, 0.13, color);
    fill(
        painter,
        rect,
        &[(0.16, 0.94), (0.26, 0.42), (0.42, 0.42), (0.46, 0.94)],
        color,
    );
    stroke_path(painter, rect, &[(0.38, 0.48), (0.88, 0.38)], 2.6, color);
    stroke_path(painter, rect, &[(0.38, 0.62), (0.9, 0.62)], 2.6, color);
    disc(painter, rect, 0.32, 0.2, 0.045, DETAIL);
    disc(painter, rect, 0.26, 0.28, 0.045, DETAIL);
}

fn inspect_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = point(rect, 0.44, 0.42);
    let radius = rect.width().min(rect.height()) * 0.24;
    painter.circle_stroke(center, radius, Stroke::new(2.0_f32, color));
    stroke_path(painter, rect, &[(0.62, 0.6), (0.84, 0.82)], 2.6, color);
}

// --- помощники --------------------------------------------------------------

/// Цвет «прорези» внутри иконки: подложка кнопки почти чёрная.
const DETAIL: Color32 = Color32::from_rgb(18, 20, 26);

fn point(rect: Rect, x: f32, y: f32) -> Pos2 {
    egui::pos2(
        rect.left() + rect.width() * x,
        rect.top() + rect.height() * y,
    )
}

fn to_points(rect: Rect, points: &[(f32, f32)]) -> Vec<Pos2> {
    points.iter().map(|(x, y)| point(rect, *x, *y)).collect()
}

fn fill(painter: &egui::Painter, rect: Rect, points: &[(f32, f32)], color: Color32) {
    if points.len() < 3 {
        return;
    }
    painter.add(egui::Shape::convex_polygon(
        to_points(rect, points),
        color,
        Stroke::NONE,
    ));
}

fn stroke_path(painter: &egui::Painter, rect: Rect, points: &[(f32, f32)], width: f32, color: Color32) {
    if points.len() < 2 {
        return;
    }
    painter.add(egui::Shape::line(
        to_points(rect, points),
        Stroke::new(width.max(1.0), color),
    ));
}

fn disc(painter: &egui::Painter, rect: Rect, cx: f32, cy: f32, radius: f32, color: Color32) {
    let scale = rect.width().min(rect.height());
    painter.circle_filled(point(rect, cx, cy), scale * radius, color);
}

fn bar(painter: &egui::Painter, rect: Rect, x: f32, y: f32, width: f32, height: f32, color: Color32) {
    let min = point(rect, x, y);
    let max = point(rect, x + width, y + height);
    painter.rect_filled(
        Rect::from_min_max(min, max),
        CornerRadius::same(2),
        color,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_are_inside_rect() {
        let rect = Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(40.0, 40.0));
        let p = point(rect, 0.0, 0.0);
        assert_eq!(p, egui::pos2(10.0, 20.0));
        let p = point(rect, 1.0, 1.0);
        assert_eq!(p, egui::pos2(50.0, 60.0));
    }

    #[test]
    fn icons_do_not_panic() {
        // Рисуем все иконки в тестовый контекст egui.
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = Rect::from_min_size(ui.min_rect().min, egui::vec2(32.0, 32.0));
                let painter = ui.painter();
                for tool in Tool::ALL {
                    tool_icon(painter, rect, tool, Color32::WHITE);
                }
                for speed in Speed::ALL {
                    speed_icon(painter, rect, speed);
                }
                planet_icon(painter, rect, Color32::WHITE);
                menu_icon(painter, rect, Color32::WHITE);
            });
        });
    }
}
