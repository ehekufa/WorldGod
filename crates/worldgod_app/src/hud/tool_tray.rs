//! Лоток инструментов внизу экрана: кисти, существа и размер кисти.

use eframe::egui;
use egui::{RichText, Ui};
use worldgod_core::{Tool, ToolConfig};

use crate::hud::widgets;
use crate::state::AppState;
use crate::theme;

/// Рисует лоток инструментов.
pub fn show(ui: &mut Ui, state: &mut AppState) {
    if state.sim.selected.is_some() && state.tool.tool == Tool::Inspect {
        // Выбрали существо — подсказываем, как снять выбор.
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Существо выбрано · тапните по пустому месту, чтобы снять выбор")
                    .size(11.0)
                    .color(theme::TEXT_DIM),
            );
        });
    }

    egui::ScrollArea::horizontal()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for tool in Tool::ALL {
                    let selected = state.tool.tool == tool;
                    let response = widgets::tool_button(ui, tool, selected);
                    if response.clicked() {
                        state.tool.tool = tool;
                        if tool != Tool::Inspect {
                            state.sim.select(None);
                        }
                    }
                    response.on_hover_text(tool.hint_ru());
                }
            });
        });

    if state.tool.tool.is_terrain() {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Радиус").size(11.5).color(theme::TEXT_DIM));
            let mut radius = state.tool.clamped_radius();
            let slider = egui::Slider::new(&mut radius, ToolConfig::MIN_RADIUS..=ToolConfig::MAX_RADIUS)
                .show_value(true);
            if ui.add(slider).changed() {
                state.tool.radius = radius;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(state.status_line())
                        .size(11.0)
                        .color(theme::TEXT_DIM),
                );
            });
        });
    } else {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(state.status_line())
                    .size(11.0)
                    .color(theme::TEXT_DIM),
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldgod_core::WorldSize;

    #[test]
    fn tray_renders_for_every_tool() {
        let mut state = AppState::new(WorldSize::SMALL, 3);
        let ctx = egui::Context::default();
        for tool in Tool::ALL {
            state.tool.tool = tool;
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::TopBottomPanel::bottom("test").show(ctx, |ui| {
                    show(ui, &mut state);
                });
            });
        }
    }
}
