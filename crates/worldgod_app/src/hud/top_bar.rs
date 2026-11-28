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
        // Справа живут часы и две кнопки — под чипы оставляем остальное,
        // чтобы на узком экране они не уезжали за край.
        let mut budget = (ui.available_width() - 150.0).max(0.0);
        // Порядок важен: первыми идут главные чипы, хвостовые уходят
        // за край только когда экран совсем узкий.
        let chips: [(&str, &str, &str, u32, Color32); 6] = [
            ("человек", "человека", "людей", stats.population, theme::SUCCESS),
            ("страна", "страны", "стран", stats.nations, theme::ACCENT),
            ("монстр", "монстра", "монстров", stats.monsters, theme::DANGER),
            ("война", "войны", "войн", stats.wars, theme::DANGER),
            ("союз", "союза", "союзов", stats.alliances, theme::SUCCESS),
            ("лодка", "лодки", "лодок", stats.boats, theme::ACCENT),
        ];
        for (one, few, many, value, color) in chips {
            // Войны, союзы и лодки показываем только когда они есть.
            if !matches!(one, "человек" | "страна" | "монстр") && value == 0 {
                continue;
            }
            let label = plural_ru(value, one, few, many);
            let text = value.to_string();
            let width = widgets::stat_chip_width(ui, label, &text);
            if width > budget {
                break;
            }
            budget -= width + 8.0;
            widgets::stat_chip(ui, label, &text, color);
        }
        // Доля суши — не счётчик, поэтому подпись постоянная.
        let land = format!("{:.0}%", stats.land_fraction * 100.0);
        if widgets::stat_chip_width(ui, "суша", &land) <= budget {
            widgets::stat_chip(ui, "суша", &land, theme::WARNING);
            budget -= widgets::stat_chip_width(ui, "суша", &land) + 8.0;
        }
        // Снег показываем, только когда он лежит: «12% снег».
        if stats.snow_tiles > 0 {
            let snow = format!("{:.0}%", stats.snow_fraction * 100.0);
            if widgets::stat_chip_width(ui, "снег", &snow) <= budget {
                widgets::stat_chip(ui, "снег", &snow, theme::SNOW);
            }
        }

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
            ui.spacing_mut().item_spacing.x = 6.0;
            // Точку рисуем сами: символы вроде «⚑» или «⚔» есть не во всех
            // шрифтах и на телефоне превращались в пустой квадратик.
            let (dot, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
            ui.painter().circle_filled(dot.center(), 3.5, color);
            ui.label(RichText::new(&event.text).size(11.5).color(theme::TEXT_DIM));
        });
    }
}

/// Русская форма слова для числа: 1 страна, 2 страны, 5 стран.
fn plural_ru<'a>(n: u32, one: &'a str, few: &'a str, many: &'a str) -> &'a str {
    let tail = n % 100;
    if (11..=14).contains(&tail) {
        return many;
    }
    match n % 10 {
        1 => one,
        2..=4 => few,
        _ => many,
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
        EventKind::Weather => Color32::from_rgb(160, 214, 246),
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
    fn plural_forms() {
        assert_eq!(plural_ru(0, "страна", "страны", "стран"), "стран");
        assert_eq!(plural_ru(1, "страна", "страны", "стран"), "страна");
        assert_eq!(plural_ru(3, "страна", "страны", "стран"), "страны");
        assert_eq!(plural_ru(5, "страна", "страны", "стран"), "стран");
        assert_eq!(plural_ru(11, "страна", "страны", "стран"), "стран");
        assert_eq!(plural_ru(21, "страна", "страны", "стран"), "страна");
        assert_eq!(plural_ru(112, "страна", "страны", "стран"), "стран");
    }

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
