//! Окно настроек: новая планета, размер мира, справка по управлению.

use eframe::egui;
use egui::{RichText, Ui};
use worldgod_core::WorldSize;

use crate::state::AppState;
use crate::theme;

/// Показывает окно настроек, если оно открыто.
pub fn show(ctx: &egui::Context, state: &mut AppState) {
    if !state.settings_open {
        return;
    }

    let mut open = state.settings_open;
    egui::Window::new("Настройки мира")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_min_width(300.0);
            ui.label(
                RichText::new("Размер новой планеты")
                    .size(12.0)
                    .color(theme::TEXT_DIM),
            );
            ui.horizontal(|ui| {
                for size in WorldSize::ALL {
                    let selected = state.world_size == size;
                    if ui.selectable_label(selected, size.label_ru()).clicked() {
                        state.world_size = size;
                    }
                }
            });
            ui.label(
                RichText::new(format!(
                    "{} × {} тайлов",
                    state.world_size.width, state.world_size.height
                ))
                .size(11.0)
                .color(theme::TEXT_DIM),
            );

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Новая планета").clicked() {
                    let size = state.world_size;
                    state.new_world(size);
                }
                if ui.button("Вписать карту").clicked() {
                    state.reset_view();
                }
            });

            ui.add_space(6.0);
            ui.checkbox(&mut state.show_events, "Строка событий");
            ui.add_space(4.0);
            ui.separator();

            ui.label(RichText::new("Как играть").size(12.0).color(theme::TEXT_DIM));
            help_line(
                ui,
                "1. Кисть «Суша» — поднимите остров из океана.",
            );
            help_line(ui, "2. «Лес» даёт дерево и еду, «Горы» — неприступные стены.");
            help_line(
                ui,
                "3. Поставьте людей: они собирают дерево, строят дома и основывают страну.",
            );
            help_line(
                ui,
                "4. Волки и зомби охотятся на жителей, воины дают отпор.",
            );
            help_line(
                ui,
                "5. Стихии: ураган и вспышка ломают дома, дождь растит лес, снег замедляет ходьбу.",
            );

            ui.add_space(6.0);
            ui.separator();
            ui.label(RichText::new("Управление").size(12.0).color(theme::TEXT_DIM));
            ui.label(
                RichText::new(
                    "Палец — кисть. Два пальца — панорама и зум.\n\
                     ПК: ЛКМ — кисть, ПКМ/СКМ — панорама, колесо — зум.\n\
                     Пробел — пауза, клавиши 1…9 — инструменты, Esc — снять выбор.\n\
                     Вкладки лотка: Рельеф, Жизнь, Стихии, Прочее.",
                )
                .size(11.0)
                .color(theme::TEXT_DIM),
            );

            ui.add_space(6.0);
            ui.separator();
            ui.label(
                RichText::new(format!(
                    "WorldGod {} · тик {} · {:.0} FPS · зерно {}",
                    worldgod_core::ENGINE_VERSION,
                    state.sim.tick,
                    state.fps.fps(),
                    state.seed
                ))
                .size(10.5)
                .color(theme::TEXT_DIM),
            );
        });

    state.settings_open = open;
}

fn help_line(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).color(theme::TEXT));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_renders_when_open() {
        let mut state = AppState::new(WorldSize::SMALL, 6);
        state.settings_open = true;

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            show(ctx, &mut state);
        });
    }

    #[test]
    fn closed_window_does_nothing() {
        let mut state = AppState::new(WorldSize::SMALL, 6);
        state.settings_open = false;
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            show(ctx, &mut state);
        });
    }
}
