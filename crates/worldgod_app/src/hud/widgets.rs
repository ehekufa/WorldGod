//! Мелкие элементы интерфейса: чипы статистики, кнопки, полоски.

use eframe::egui;
use egui::{Align2, Color32, CornerRadius, FontId, Rect, Response, RichText, Sense, Ui};
use worldgod_core::brush::Tool;

use crate::theme;
use crate::ui_icons;

/// Чип статистики: цветная точка, значение и подпись.
pub fn stat_chip(ui: &mut Ui, label: &str, value: &str, color: Color32) {
    let value_font = FontId::proportional(13.5);
    let label_font = FontId::proportional(11.0);

    let value_galley = ui
        .painter()
        .layout_no_wrap(value.to_owned(), value_font, theme::TEXT);
    let label_galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), label_font, theme::TEXT_DIM);

    let dot = 7.0;
    let padding = 8.0;
    let gap = 5.0;
    let width = padding * 2.0 + dot + gap + value_galley.size().x + gap + label_galley.size().x;
    let height = 27.0;

    let (rect, _response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(rect, CornerRadius::same(theme::RADIUS - 4), theme::SURFACE_ALT);
    let center_y = rect.center().y;
    painter.circle_filled(
        egui::pos2(rect.left() + padding + dot / 2.0, center_y),
        dot / 2.0,
        color,
    );

    let mut x = rect.left() + padding + dot + gap;
    painter.galley(
        egui::pos2(x, center_y - value_galley.size().y / 2.0),
        value_galley.clone(),
        theme::TEXT,
    );
    x += value_galley.size().x + gap;
    painter.galley(
        egui::pos2(x, center_y - label_galley.size().y / 2.0),
        label_galley.clone(),
        theme::TEXT_DIM,
    );
}

/// Квадратная кнопка с векторной иконкой.
pub fn icon_button(
    ui: &mut Ui,
    size: f32,
    draw: impl Fn(&egui::Painter, Rect, Color32),
) -> Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), Sense::click());
    let background = if response.is_pointer_button_down_on() {
        theme::SURFACE_ACTIVE
    } else if response.hovered() {
        theme::SURFACE_HOVER
    } else {
        theme::SURFACE_ALT
    };

    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(theme::RADIUS - 2), background);
    draw(&painter.with_clip_rect(rect), rect.shrink(size * 0.26), theme::TEXT);
    response
}

/// Кнопка инструмента: иконка + подпись.
pub fn tool_button(ui: &mut Ui, tool: Tool, selected: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(56.0, 66.0), Sense::click());

    let background = if selected {
        theme::ACCENT_DARK
    } else if response.hovered() {
        theme::SURFACE_HOVER
    } else {
        theme::SURFACE_ALT
    };

    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(theme::RADIUS), background);

    if selected {
        painter.rect_filled(
            rect.shrink(2.0),
            CornerRadius::same(theme::RADIUS - 2),
            theme::ACCENT_DARK,
        );
        painter.rect_filled(
            Rect::from_min_size(
                egui::pos2(rect.left() + 18.0, rect.bottom() - 5.0),
                egui::vec2(rect.width() - 36.0, 2.5),
            ),
            CornerRadius::same(2),
            theme::ACCENT,
        );
    }

    let icon_rect = Rect::from_center_size(
        rect.center() - egui::vec2(0.0, 9.0),
        egui::vec2(26.0, 26.0),
    );
    let icon_color = if selected {
        theme::ACCENT
    } else {
        theme::nation_color(tool.accent_rgb())
    };
    ui_icons::tool_icon(&painter.with_clip_rect(rect), icon_rect, tool, icon_color);

    painter.text(
        egui::pos2(rect.center().x, rect.bottom() - 9.0),
        Align2::CENTER_BOTTOM,
        tool.label_ru(),
        FontId::proportional(11.0),
        if selected { theme::ACCENT } else { theme::TEXT_DIM },
    );

    response
}

/// Тонкая полоска прогресса (здоровье, сытость, дома).
pub fn bar(ui: &mut Ui, fraction: f32, color: Color32, height: f32) {
    let width = ui.available_width();
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(3), theme::SURFACE_ACTIVE);

    let filled = (rect.width() * fraction.clamp(0.0, 1.0)).max(1.0);
    painter.rect_filled(
        Rect::from_min_size(rect.min, egui::vec2(filled, rect.height())),
        CornerRadius::same(3),
        color,
    );
}

/// Заголовок секции.
pub fn section(ui: &mut Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(RichText::new(text).size(12.0).color(theme::TEXT_DIM));
}

/// Крупное значение с подписью в одну строку.
pub fn key_value(ui: &mut Ui, key: &str, value: impl Into<String>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(key).small().color(theme::TEXT_DIM));
        ui.label(RichText::new(value.into()).small().color(theme::TEXT));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widgets_render() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                stat_chip(ui, "люди", "12", theme::SUCCESS);
                icon_button(ui, 34.0, |painter, rect, color| {
                    ui_icons::menu_icon(painter, rect, color)
                });
                tool_button(ui, Tool::Raise, true);
                tool_button(ui, Tool::Human, false);
                bar(ui, 0.5, theme::DANGER, 6.0);
                section(ui, "Секция");
                key_value(ui, "Тик", "42");
            });
        });
    }
}
