//! Верхняя панель: сводка по миру, скорость времени и кнопка настроек.

use eframe::egui;
use egui::{Align, Color32, Layout, RichText, Ui};
use worldgod_core::{EventKind, Speed};

use crate::hud::widgets;
use crate::state::AppState;
use crate::theme;
use crate::ui_icons;

/// Рисует верхнюю панель.
pub fn show(ui: &mut Ui, state: &mut AppState) {
    let stats = state.sim.stats;

    ui.horizontal(|ui| {
        widgets::stat_chip(ui, "люди", &stats.population.to_string(), theme::SUCCESS);
        widgets::stat_chip(ui, "страны", &stats.nations.to_string(), theme::ACCENT);
        widgets::stat_chip(ui, "монстры", &stats.monsters.to_string(), theme::DANGER);
        if stats.wars > 0 {
            widgets::stat_chip(ui, "войны", &stats.wars.to_string(), theme::DANGER);
        }
        if stats.alliances > 0 {
            widgets::stat_chip(ui, "союзы", &stats.alliances.to_string(), theme::SUCCESS);
        }
        if stats.boats > 0 {
            widgets::stat_chip(ui, "лодки", &stats.boats.to_string(), theme::ACCENT);
        }
        widgets::stat_chip(
            ui,
            "суша",
            &format!("{:.0}%", stats.land_fraction * 100.0),
            theme::WARNING,
        );

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let settings = widgets::icon_button(ui, 34.0, |painter, rect, color| {
                ui_icons::menu_icon(painter, rect, color)
            });
            if settings.clicked() {
                state.settings_open = true;
            }
            settings.on_hover_text("Настройки мира");

            let speed = state.sim.speed;
            let speed_response = widgets::icon_button(ui, 34.0, |painter, rect, _color| {
                ui_icons::speed_icon(painter, rect, speed);
            });
            if speed_response.clicked() {
                state.sim.cycle_speed();
            }
            speed_response.on_hover_text(format!("Скорость: {}", speed.label_ru()));

            ui.label(
                RichText::new(state.world_clock())
                    .monospace()
                    .size(12.0)
                    .color(theme::TEXT_DIM),
            );
        });
    });

    if state.show_events {
        let Some(event) = state.sim.events.last() else {
            return;
        };
        let color = event_color(event.kind);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.label(RichText::new(event.kind.prefix()).size(11.0).color(color));
            ui.label(RichText::new(&event.text).size(11.5).color(theme::TEXT_DIM));
        });
    }
}

fn event_color(kind: EventKind) -> Color32 {
    match kind {
        EventKind::Nation => theme::ACCENT,
        EventKind::Build => theme::WARNING,
        EventKind::Birth => theme::SUCCESS,
        EventKind::Death => theme::DANGER,
        EventKind::Combat => Color32::from_rgb(255, 150, 120),
        EventKind::Nature => Color32::from_rgb(150, 200, 220),
    }
}

/// Подпись текущей скорости (для подсказок).
pub fn speed_hint(speed: Speed) -> &'static str {
    speed.label_ru()
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldgod_core::WorldSize;

    #[test]
    fn top_bar_renders() {
        let mut state = AppState::new(WorldSize::SMALL, 2);
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::TopBottomPanel::top("test").show(ctx, |ui| {
                show(ui, &mut state);
            });
        });
    }

    #[test]
    fn speed_hints_exist() {
        for speed in Speed::ALL {
            assert!(!speed_hint(speed).is_empty());
        }
    }
}
