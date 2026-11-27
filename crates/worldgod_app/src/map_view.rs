//! Область карты: текстура мира, жесты (панорама, зум) и кисть.

use eframe::egui;
use egui::{Color32, CornerRadius, Rect, Sense, Stroke};
use worldgod_core::{TilePos, Vec2 as CoreVec2};

use crate::state::AppState;
use crate::theme;

/// Что произошло на карте за кадр.
#[derive(Clone, Copy, Debug, Default)]
pub struct MapResponse {
    /// Тайл под пальцем/курсором.
    pub hovered: Option<TilePos>,
    /// Сколько тайлов изменила кисть.
    pub painted: u32,
}

/// Рисует карту и обрабатывает ввод.
pub fn show(ui: &mut egui::Ui, state: &mut AppState) -> MapResponse {
    let rect = ui.max_rect();
    let response = ui.allocate_rect(rect, Sense::click_and_drag());

    let world = state.sim.size();
    state
        .view
        .set_viewport(CoreVec2::new(rect.left(), rect.top()), CoreVec2::new(rect.width(), rect.height()));

    if !state.view_initialized && rect.width() > 1.0 && rect.height() > 1.0 {
        state.view.fit(world);
        state.view_initialized = true;
    }

    ui.painter()
        .with_clip_rect(rect)
        .rect_filled(rect, CornerRadius::ZERO, theme::BACKGROUND);

    let (hovered, painted) = handle_input(ui, &response, rect, state);

    state
        .textures
        .ensure(ui.ctx(), &state.sim, state.alpha, state.time, state.texture_dirty);
    state.texture_dirty = false;

    paint_texture(ui, rect, state);
    paint_brush_cursor(ui, rect, state, hovered);

    MapResponse { hovered, painted }
}

fn handle_input(
    ui: &egui::Ui,
    response: &egui::Response,
    rect: Rect,
    state: &mut AppState,
) -> (Option<TilePos>, u32) {
    let world = state.sim.size();
    let pointer = ui.input(|input| input.pointer.interact_pos());
    let multi = ui.input(|input| {
        input
            .multi_touch()
            .map(|touch| (touch.zoom_delta, touch.translation_delta, touch.num_touches))
    });
    let zoom_delta = ui.input(|input| input.zoom_delta());
    let scroll = ui.input(|input| input.smooth_scroll_delta);

    let anchor = match pointer {
        Some(pos) => CoreVec2::new(pos.x, pos.y),
        None => CoreVec2::new(rect.center().x, rect.center().y),
    };

    // Два пальца: панорама + щипковый зум.
    if let Some((zoom, translation, touches)) = multi {
        if touches >= 2 {
            if translation.length() > 0.0 {
                state
                    .view
                    .pan_pixels(CoreVec2::new(translation.x, translation.y), world);
            }
            if (zoom - 1.0).abs() > 0.001 {
                state.view.zoom_at(anchor, zoom, world);
            }
            return (None, 0);
        }
    }

    // Колесо и трекпад (десктоп).
    if response.hovered() {
        if (zoom_delta - 1.0).abs() > 0.001 {
            state.view.zoom_at(anchor, zoom_delta, world);
        } else if scroll.length() > 0.0 {
            state
                .view
                .pan_pixels(CoreVec2::new(scroll.x, scroll.y), world);
        }
    }

    // Вторая/средняя кнопка — панорама.
    if response.dragged_by(egui::PointerButton::Secondary)
        || response.dragged_by(egui::PointerButton::Middle)
    {
        let delta = response.drag_delta();
        state
            .view
            .pan_pixels(CoreVec2::new(delta.x, delta.y), world);
        return (None, 0);
    }

    // Основной палец/кнопка — кисть.
    let painting = response.dragged_by(egui::PointerButton::Primary)
        || response.clicked()
        || response.drag_started();
    if !painting {
        return (None, 0);
    }

    let Some(pos) = pointer else {
        return (None, 0);
    };
    if !rect.contains(pos) {
        return (None, 0);
    }

    let tile = state.view.tile_at(CoreVec2::new(pos.x, pos.y));
    let painted = if state.brush_tick(tile) {
        state.apply_tool_at(tile)
    } else {
        0
    };
    (Some(tile), painted)
}

fn paint_texture(ui: &egui::Ui, rect: Rect, state: &AppState) {
    let Some(texture) = state.textures.id() else {
        return;
    };
    let world = state.sim.size();
    let world_min = state.view.tile_to_screen(CoreVec2::ZERO);
    let world_max = state
        .view
        .tile_to_screen(CoreVec2::new(world.0 as f32, world.1 as f32));

    let world_rect = Rect::from_min_max(
        egui::pos2(world_min.x, world_min.y),
        egui::pos2(world_max.x, world_max.y),
    );
    let draw_rect = world_rect.intersect(rect);
    if draw_rect.width() <= 0.5 || draw_rect.height() <= 0.5 {
        return;
    }

    let uv = Rect::from_min_max(
        egui::pos2(
            (draw_rect.left() - world_rect.left()) / world_rect.width(),
            (draw_rect.top() - world_rect.top()) / world_rect.height(),
        ),
        egui::pos2(
            (draw_rect.right() - world_rect.left()) / world_rect.width(),
            (draw_rect.bottom() - world_rect.top()) / world_rect.height(),
        ),
    );

    ui.painter()
        .with_clip_rect(rect)
        .image(texture, draw_rect, uv, Color32::WHITE);
}

fn paint_brush_cursor(
    ui: &egui::Ui,
    rect: Rect,
    state: &AppState,
    hovered: Option<TilePos>,
) {
    let is_terrain = state.tool.tool.is_terrain();
    let is_spawn = state.tool.tool.species().is_some();
    if !is_terrain && !is_spawn {
        return;
    }

    let Some(pos) = ui.input(|input| input.pointer.hover_pos()) else {
        return;
    };
    if !rect.contains(pos) {
        return;
    }

    let tile = hovered.unwrap_or_else(|| state.view.tile_at(CoreVec2::new(pos.x, pos.y)));
    let center = state.view.tile_center_screen(tile);
    let center = egui::pos2(center.x, center.y);
    let painter = ui.painter().with_clip_rect(rect);

    if is_terrain {
        let radius = (state.tool.clamped_radius() as f32 + 0.6) * state.view.scale;
        painter.circle_stroke(center, radius, Stroke::new(3.5, Color32::from_black_alpha(90)));
        painter.circle_stroke(
            center,
            radius,
            Stroke::new(1.5, theme::with_alpha(theme::ACCENT, 190)),
        );
    }

    if is_spawn {
        painter.circle_stroke(center, 9.0, Stroke::new(2.0, Color32::from_black_alpha(80)));
        painter.circle_filled(center, 3.5, theme::with_alpha(theme::ACCENT, 230));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldgod_core::brush::Tool;
    use worldgod_core::WorldSize;

    #[test]
    fn map_renders_without_panic() {
        let mut state = AppState::new(WorldSize::SMALL, 4);
        state.tool.tool = Tool::Raise;

        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(480.0, 640.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let response = show(ui, &mut state);
                assert!(response.hovered.is_none());
                assert_eq!(response.painted, 0);
            });
        });
        assert!(state.view_initialized);
    }
}
