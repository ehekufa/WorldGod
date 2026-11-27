//! Сборка кадра: рельеф, постройки, существа, подсветка выбора.

use worldgod_core::building::Building;
use worldgod_core::entity::Entity;
use worldgod_core::sim::Simulation;

use crate::framebuffer::Frame;
use crate::palette;
use crate::sprite;
use crate::terrain_pass;

/// Настройки кадра.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderOptions {
    /// Сколько пикселей кадра приходится на один тайл.
    pub px_per_tile: u32,
    /// Время в секундах — используется для волн, огня и пены.
    pub time: f32,
    /// Доля пути между двумя тиками (0..1) для плавного движения.
    pub alpha: f32,
    /// Рисовать ли существ.
    pub show_entities: bool,
}

impl RenderOptions {
    /// Значения по умолчанию: 3 пикселя на тайл, без интерполяции.
    pub const fn new(px_per_tile: u32) -> Self {
        Self {
            px_per_tile,
            time: 0.0,
            alpha: 1.0,
            show_entities: true,
        }
    }
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self::new(3)
    }
}

/// Размер кадра для мира и масштаба.
pub fn scene_size(sim: &Simulation, px_per_tile: u32) -> (u32, u32) {
    let (width, height) = sim.size();
    let px = px_per_tile.max(1);
    (width * px, height * px)
}

/// Рисует полный кадр мира.
pub fn render_scene(sim: &Simulation, options: RenderOptions) -> Frame {
    let px = options.px_per_tile.max(1);
    let (width, height) = scene_size(sim, px);
    let mut frame = Frame::filled(width, height, palette::BACKGROUND);

    terrain_pass::draw(
        &mut frame,
        &sim.world,
        sim.territory(),
        &sim.nations,
        px,
        options.time,
    );

    for building in &sim.buildings {
        draw_building(&mut frame, sim, building, px, options.time);
    }

    if options.show_entities {
        draw_entities(&mut frame, sim, px, options);
    }

    frame
}

fn draw_building(frame: &mut Frame, sim: &Simulation, building: &Building, px: u32, time: f32) {
    let nation_color = sim.nation_color(building.nation);
    let center_x = building.pos.x * px as i32 + px as i32 / 2;
    let bottom_y = (building.pos.y + 1) * px as i32;

    sprite::draw_building(
        frame,
        building.kind,
        center_x,
        bottom_y,
        px as i32,
        nation_color,
        time,
        building.completion(),
        building.variant,
    );
}

fn draw_entities(frame: &mut Frame, sim: &Simulation, px: u32, options: RenderOptions) {
    let px_i = px as i32;
    let mut entities: Vec<&Entity> = sim.entities.iter().filter(|entity| entity.alive).collect();
    // Нижние существа рисуются поверх — так сцена читается объёмнее.
    entities.sort_by_key(|entity| (entity.pos.y, entity.id));

    for entity in entities {
        let (center_x, bottom_y) = screen_position(entity, px_i, options.alpha);
        let cloth = sim.nation_color(entity.nation);

        sprite::draw_entity(
            frame,
            entity.species,
            center_x,
            bottom_y,
            px_i,
            cloth,
            entity.variant,
            entity.intent.is_active(),
        );

        if sim.selected == Some(entity.id) {
            draw_selection(frame, center_x, bottom_y, px_i);
        }
    }
}

/// Экранная позиция существа с интерполяцией между тиками.
fn screen_position(entity: &Entity, px: i32, alpha: f32) -> (i32, i32) {
    let alpha = alpha.clamp(0.0, 1.0);
    let x = entity.prev_pos.x as f32 + (entity.pos.x - entity.prev_pos.x) as f32 * alpha;
    let y = entity.prev_pos.y as f32 + (entity.pos.y - entity.prev_pos.y) as f32 * alpha;
    let center_x = ((x + 0.5) * px as f32).round() as i32;
    let bottom_y = ((y + 1.0) * px as f32).round() as i32;
    (center_x, bottom_y)
}

fn draw_selection(frame: &mut Frame, center_x: i32, bottom_y: i32, px: i32) {
    let scale = (px / 3).max(1);
    let height = scale * 5;
    let width = scale * 4;
    let x = center_x - width / 2;
    let y = bottom_y - height;

    frame.outline_rect(
        x,
        y,
        width as u32,
        height as u32,
        palette::SELECTION,
    );
    frame.blend(center_x, y - 1, palette::SELECTION, 0.7);
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldgod_core::brush::{Tool, ToolConfig};
    use worldgod_core::grid::TilePos;
    use worldgod_core::species::Species;
    use worldgod_core::WorldSize;

    fn sample_world() -> Simulation {
        let mut sim = Simulation::new(WorldSize::SMALL, 11);
        sim.apply_tool(ToolConfig::new(Tool::Raise, 8), TilePos::new(60, 60));
        sim.apply_tool(ToolConfig::new(Tool::Forest, 5), TilePos::new(62, 62));
        sim.spawn(Species::Human, TilePos::new(60, 60));
        sim.spawn(Species::Wolf, TilePos::new(63, 60));
        sim.run(30);
        sim
    }

    #[test]
    fn scene_has_expected_size() {
        let sim = sample_world();
        let frame = render_scene(&sim, RenderOptions::new(3));
        assert_eq!(
            frame.size(),
            (sim.size().0 * 3, sim.size().1 * 3)
        );
    }

    #[test]
    fn scene_changes_after_terrain_edit() {
        let mut sim = Simulation::new(WorldSize::SMALL, 2);
        let before = render_scene(&sim, RenderOptions::new(2));
        sim.apply_tool(ToolConfig::new(Tool::Raise, 6), TilePos::new(50, 40));
        let after = render_scene(&sim, RenderOptions::new(2));
        assert_ne!(before.pixels(), after.pixels());
    }

    #[test]
    fn hiding_entities_changes_pixels() {
        let sim = sample_world();
        let with = render_scene(&sim, RenderOptions::new(3));
        let without = render_scene(
            &sim,
            RenderOptions {
                px_per_tile: 3,
                time: 0.0,
                alpha: 1.0,
                show_entities: false,
            },
        );
        assert_ne!(with.pixels(), without.pixels());
    }

    #[test]
    fn selected_entity_is_highlighted() {
        let mut sim = sample_world();
        let id = sim.entities.first().map(|entity| entity.id);
        sim.select(id);
        let plain = render_scene(&sim, RenderOptions::new(3));
        sim.select(None);
        let selected_free = render_scene(&sim, RenderOptions::new(3));
        assert_ne!(plain.pixels(), selected_free.pixels());
    }

    #[test]
    fn png_is_produced() {
        let sim = sample_world();
        let frame = render_scene(&sim, RenderOptions::new(2));
        let png = frame.to_png();
        assert!(png.len() > 1000);
    }
}
