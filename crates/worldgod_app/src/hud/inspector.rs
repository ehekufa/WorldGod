//! Карточка выбранного существа.

use eframe::egui;
use egui::{Align, Layout, RichText, Ui};

use crate::hud::widgets;
use crate::state::AppState;
use crate::theme;

/// Рисует панель выбранного существа (если что-то выбрано).
pub fn show(ui: &mut Ui, state: &mut AppState) {
    // Копируем нужные поля, чтобы не держать заимствование мира.
    let Some(info) = state.sim.selected_entity().map(|entity| {
        (
            entity.species,
            entity.nation,
            entity.health_fraction(),
            entity.hunger_fraction(),
            entity.intent.label_ru(),
            entity.kills,
            entity.hp,
        )
    }) else {
        return;
    };

    let (species, nation, health, hunger, intent, kills, hp) = info;
    let nation_name = state.sim.nation_name(nation).to_string();
    let nation_color = theme::nation_color(state.sim.nation_color(nation));

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(species.name_ru())
                .strong()
                .size(15.0)
                .color(theme::TEXT),
        );
        ui.label(RichText::new(nation_name).size(11.5).color(nation_color));

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.small_button("Скрыть").clicked() {
                state.sim.select(None);
            }
            ui.label(
                RichText::new(format!("{intent} · убийств: {kills}"))
                    .size(11.0)
                    .color(theme::TEXT_DIM),
            );
        });
    });

    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Жизнь").size(11.0).color(theme::TEXT_DIM));
        ui.label(
            RichText::new(format!("{hp} / {}", species.stats().max_hp))
                .size(11.0)
                .color(theme::TEXT),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!("Сытость {:.0}%", (1.0 - hunger) * 100.0))
                    .size(11.0)
                    .color(theme::TEXT_DIM),
            );
        });
    });
    widgets::bar(ui, health, health_color(health), 6.0);
}

fn health_color(health: f32) -> egui::Color32 {
    if health > 0.6 {
        theme::SUCCESS
    } else if health > 0.3 {
        theme::WARNING
    } else {
        theme::DANGER
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldgod_core::{Species, TilePos, WorldSize};

    #[test]
    fn inspector_renders_when_something_is_selected() {
        let mut state = AppState::new(WorldSize::SMALL, 8);
        state
            .sim
            .apply_tool(worldgod_core::ToolConfig::new(worldgod_core::Tool::Raise, 6), TilePos::new(60, 60));
        let id = state.sim.spawn(Species::Human, TilePos::new(60, 60));
        state.sim.select(id);

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::TopBottomPanel::bottom("test").show(ctx, |ui| {
                show(ui, &mut state);
            });
        });
    }

    #[test]
    fn health_colors_are_distinct() {
        assert_ne!(health_color(0.9), health_color(0.1));
        assert_eq!(health_color(0.9), theme::SUCCESS);
    }
}
