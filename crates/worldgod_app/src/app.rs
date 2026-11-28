//! Приложение: связывает симуляцию, рендер и интерфейс.

use std::time::Duration;

use eframe::egui;
use worldgod_core::{Tool, WorldSize};

use crate::hud;
use crate::map_view;
use crate::state::AppState;
use crate::theme;

/// Главное приложение WorldGod.
#[derive(Debug)]
pub struct WorldGodApp {
    state: AppState,
    last_time: f64,
}

impl WorldGodApp {
    /// Создаёт приложение и настраивает тему.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);
        Self {
            state: AppState::new(WorldSize::MEDIUM, generate_seed()),
            last_time: 0.0,
        }
    }
}

impl eframe::App for WorldGodApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = ctx.input(|input| input.time);
        let dt = if self.last_time <= 0.0 {
            1.0 / 60.0
        } else {
            (now - self.last_time).clamp(0.0, 0.25)
        };
        self.last_time = now;

        self.state.advance(dt);
        handle_shortcuts(ctx, &mut self.state);
        draw_ui(ctx, &mut self.state);

        if self.state.sim.speed.is_paused() {
            ctx.request_repaint_after(Duration::from_millis(180));
        } else {
            ctx.request_repaint();
        }
    }
}

/// Рисует все панели интерфейса. Вынесено отдельно, чтобы тестировать без окна.
pub fn draw_ui(ctx: &egui::Context, state: &mut AppState) {
    egui::TopBottomPanel::top("hud_top").show(ctx, |ui| {
        ui.add_space(3.0);
        hud::top_bar::show(ui, state);
        ui.add_space(3.0);
    });

    // Лоток инструментов — самая нижняя панель.
    egui::TopBottomPanel::bottom("hud_tools").show(ctx, |ui| {
        ui.add_space(3.0);
        hud::tool_tray::show(ui, state);
        ui.add_space(3.0);
    });

    // Карточка выбранного существа — над лотком.
    if state.sim.selected.is_some() {
        egui::TopBottomPanel::bottom("hud_selected").show(ctx, |ui| {
            ui.add_space(3.0);
            hud::inspector::show(ui, state);
            ui.add_space(3.0);
        });
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        map_view::show(ui, state);
    });

    hud::settings::show(ctx, state);
}

fn handle_shortcuts(ctx: &egui::Context, state: &mut AppState) {
    let pressed = |key: egui::Key| ctx.input(|input| input.key_pressed(key));

    if pressed(egui::Key::Space) {
        state.sim.toggle_pause();
    }
    if pressed(egui::Key::Escape) {
        state.sim.select(None);
    }
    if pressed(egui::Key::Home) {
        state.reset_view();
    }

    const KEYS: [egui::Key; 9] = [
        egui::Key::Num1,
        egui::Key::Num2,
        egui::Key::Num3,
        egui::Key::Num4,
        egui::Key::Num5,
        egui::Key::Num6,
        egui::Key::Num7,
        egui::Key::Num8,
        egui::Key::Num9,
    ];

    for (index, key) in KEYS.iter().enumerate() {
        if pressed(*key) {
            if let Some(tool) = Tool::ALL.get(index) {
                // Горячие клавиши открывают вкладку выбранной кисти.
                state.select_tool(*tool);
            }
        }
    }
}

fn generate_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0x5EED_1234_ABCD_0001, |duration| duration.as_nanos() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_ui_renders() {
        let mut state = AppState::new(WorldSize::SMALL, 12);
        let ctx = egui::Context::default();

        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(420.0, 800.0),
                )),
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                draw_ui(ctx, &mut state);
            });
        }

        assert!(state.view_initialized);
    }

    #[test]
    fn shortcuts_are_noop_without_input() {
        let ctx = egui::Context::default();
        let mut state = AppState::new(WorldSize::SMALL, 1);
        let speed_before = state.sim.speed;
        let tool_before = state.tool.tool;

        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            handle_shortcuts(ctx, &mut state);
        });

        assert_eq!(state.sim.speed, speed_before);
        assert_eq!(state.tool.tool, tool_before);
    }

    #[test]
    fn seed_is_non_zero() {
        assert_ne!(generate_seed(), 0);
    }
}
