//! Отрисовка рельефа: тайлы, рельефное затенение, вода, берег, границы стран.

use worldgod_core::grid::TilePos;
use worldgod_core::nation::Nation;
use worldgod_core::terrain::{Terrain, SEA_LEVEL};
use worldgod_core::world::{Tile, World};

use crate::framebuffer::Frame;
use crate::palette::{self, lighten, mix};

/// Рисует всю карту.
///
/// `territory` — карта владений из [`worldgod_core::Simulation::territory`]
/// (0 — ничья земля), `nations` — список стран для цветов.
pub fn draw(
    frame: &mut Frame,
    world: &World,
    territory: &[u8],
    nations: &[Nation],
    px_per_tile: u32,
    time: f32,
) {
    let px = px_per_tile.max(1) as i32;
    let width = world.width() as i32;
    let height = world.height() as i32;
    let has_territory = territory.len() == world.len();

    for y in 0..height {
        for x in 0..width {
            let pos = TilePos::new(x, y);
            let Some(tile) = world.tile(pos) else {
                continue;
            };
            let terrain = tile.terrain();
            let sx = x * px;
            let sy = y * px;

            frame.fill_rect(
                sx,
                sy,
                px as u32,
                px as u32,
                tile_color(world, pos, tile, terrain, time),
            );

            // Под толстым слоем снега мелкие детали (трава, песок, кроны)
            // не видны: снег рисуется как ровное белое покрывало.
            if tile.snow < 90 {
                match terrain {
                    Terrain::Forest => forest_detail(frame, sx, sy, px, tile.variant),
                    Terrain::Hills | Terrain::Mountain | Terrain::Snow => {
                        rock_detail(frame, sx, sy, px, terrain, tile)
                    }
                    Terrain::Beach => sand_detail(frame, sx, sy, px, tile.variant),
                    Terrain::Grass => grass_detail(frame, sx, sy, px, tile.variant),
                    _ => {}
                }
            }

            if terrain.is_water() && world.land_neighbors(pos) > 0 {
                coast_foam(frame, sx, sy, px, world, pos, time);
            }

            if has_territory {
                let index = (y as usize) * (width as usize) + (x as usize);
                let owner = territory[index];
                if owner != 0 && terrain.is_land() {
                    if let Some(color) = nation_color(nations, owner) {
                        frame.blend_rect(sx, sy, px as u32, px as u32, color, 0.16);
                        draw_border(frame, world, territory, nations, pos, sx, sy, px);
                    }
                }
            }
        }
    }
}

/// Цвет тайла с учётом рельефа и времени (вода «дышит»).
fn tile_color(world: &World, pos: TilePos, tile: &Tile, terrain: Terrain, time: f32) -> [u8; 3] {
    let base = palette::terrain_color(terrain);

    if terrain.is_water() {
        return water_color(pos, tile, terrain, time);
    }

    // Свет с северо-запада: тайл, который выше соседей, светлее.
    let elevation = tile.elevation as f32;
    let left = world.elevation(pos.offset(-1, 0)) as f32;
    let up = world.elevation(pos.offset(0, -1)) as f32;
    let slope = ((elevation - left) + (elevation - up)) * 0.5;
    let light = (1.0 + slope / 900.0).clamp(0.72, 1.22);
    let variation = match tile.variant {
        0 => 1.0,
        1 => 0.96,
        2 => 1.035,
        _ => 0.985,
    };

    let color = lighten(base, light * variation);

    // Снежный покров — погодный слой поверх рельефа: чем его больше,
    // тем сильнее тайл уходит в белый.
    let snow = tile.snow_fraction();
    if snow > 0.02 {
        mix(color, palette::SNOW, (snow * 0.88).min(1.0))
    } else {
        color
    }
}

fn water_color(pos: TilePos, tile: &Tile, terrain: Terrain, time: f32) -> [u8; 3] {
    let depth = (tile.elevation as f32 / SEA_LEVEL as f32).clamp(0.0, 1.0);
    let base = match terrain {
        Terrain::DeepWater => mix(palette::DEEP_WATER, palette::OCEAN, depth * 0.7),
        Terrain::Ocean => mix(palette::DEEP_WATER, palette::OCEAN, 0.6 + depth * 0.4),
        _ => mix(palette::OCEAN, palette::SHALLOW_WATER, 0.55 + depth * 0.45),
    };

    // Медленные волны: чем мельче, тем заметнее блики.
    let phase = pos.x as f32 * 0.55 + pos.y as f32 * 1.05 + time * 1.3;
    let wave = (phase.sin() * 0.5 + 0.5).powi(6);
    let shimmer = match terrain {
        Terrain::DeepWater => 0.08,
        Terrain::Ocean => 0.14,
        _ => 0.3,
    };

    mix(base, palette::FOAM, wave * shimmer)
}

fn forest_detail(frame: &mut Frame, sx: i32, sy: i32, px: i32, variant: u8) {
    let points: [(i32, i32); 2] = match variant {
        0 => [(px / 3, px / 3), (px * 2 / 3, px * 2 / 3)],
        1 => [(px / 2, px / 4), (px / 3, px * 3 / 4)],
        2 => [(px / 4, px / 2), (px * 3 / 4, px / 2)],
        _ => [(px / 2, px / 2), (px / 4, px * 3 / 4)],
    };
    for (dx, dy) in points {
        frame.put(sx + dx, sy + dy + 1, palette::TREE);
        frame.put(sx + dx, sy + dy, palette::TREE_LIGHT);
    }
}

fn rock_detail(frame: &mut Frame, sx: i32, sy: i32, px: i32, terrain: Terrain, tile: &Tile) {
    if terrain == Terrain::Mountain && px >= 2 {
        // Снежная шапка у высоких вершин.
        if tile.elevation > 860 {
            frame.put(sx + px / 2, sy + px / 3, palette::SNOW);
        } else {
            frame.put(sx + px / 3, sy + px / 3, palette::MOUNTAIN_LIGHT);
        }
    } else if px >= 2 {
        frame.put(sx + px / 2, sy + px / 2, palette::MOUNTAIN_LIGHT);
    }
}

fn sand_detail(frame: &mut Frame, sx: i32, sy: i32, px: i32, variant: u8) {
    if px < 2 {
        return;
    }
    let (dx, dy) = match variant {
        0 => (px / 3, px * 2 / 3),
        1 => (px * 2 / 3, px / 3),
        2 => (px / 2, px / 2),
        _ => (px / 4, px / 2),
    };
    frame.put(sx + dx, sy + dy, palette::BEACH_LIGHT);
}

fn grass_detail(frame: &mut Frame, sx: i32, sy: i32, px: i32, variant: u8) {
    if px < 2 {
        return;
    }
    let (dx, dy) = match variant {
        0 => (px / 3, px / 3),
        1 => (px * 2 / 3, px / 2),
        2 => (px / 2, px * 2 / 3),
        _ => (px / 4, px * 3 / 4),
    };
    frame.put(sx + dx, sy + dy, palette::GRASS_DARK);
}

fn coast_foam(frame: &mut Frame, sx: i32, sy: i32, px: i32, world: &World, pos: TilePos, time: f32) {
    let pulse = (time * 1.1 + pos.x as f32 * 0.35 + pos.y as f32 * 0.2).sin() * 0.5 + 0.5;
    let alpha = 0.35 + pulse * 0.35;
    let px_u = px as u32;

    if world.is_land(pos.offset(1, 0)) {
        frame.blend_rect(sx + px - 1, sy, 1, px_u, palette::FOAM, alpha);
    }
    if world.is_land(pos.offset(-1, 0)) {
        frame.blend_rect(sx, sy, 1, px_u, palette::FOAM, alpha);
    }
    if world.is_land(pos.offset(0, 1)) {
        frame.blend_rect(sx, sy + px - 1, px_u, 1, palette::FOAM, alpha);
    }
    if world.is_land(pos.offset(0, -1)) {
        frame.blend_rect(sx, sy, px_u, 1, palette::FOAM, alpha);
    }
}

fn draw_border(
    frame: &mut Frame,
    world: &World,
    territory: &[u8],
    nations: &[Nation],
    pos: TilePos,
    sx: i32,
    sy: i32,
    px: i32,
) {
    let Some(index) = world.index(pos) else {
        return;
    };
    let owner = territory[index];
    if owner == 0 {
        return;
    }
    let Some(color) = nation_color(nations, owner).map(|color| palette::lighten(color, 1.3)) else {
        return;
    };

    let width = world.width() as usize;
    let left_owner = if pos.x > 0 { territory[index - 1] } else { 0 };
    let top_owner = if pos.y > 0 {
        territory[index - width]
    } else {
        0
    };

    if left_owner != owner {
        frame.vline(sx, sy, sy + px - 1, color);
    }
    if top_owner != owner {
        frame.hline(sx, sx + px - 1, sy, color);
    }
}

fn nation_color(nations: &[Nation], owner: u8) -> Option<[u8; 3]> {
    let id = owner as u32;
    nations
        .iter()
        .find(|nation| nation.id == id)
        .map(|nation| nation.color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldgod_core::world::World;

    #[test]
    fn ocean_is_blue_and_land_is_not_blue() {
        let mut world = World::ocean(16, 16, 3);
        let mut frame = Frame::new(16 * 3, 16 * 3);
        draw(&mut frame, &world, &[], &[], 3, 0.0);
        let water = frame.get(5, 5).expect("пиксель");
        assert!(water[2] > water[0], "океан должен быть синим: {water:?}");

        world.set_terrain(TilePos::new(8, 8), Terrain::Grass);
        draw(&mut frame, &world, &[], &[], 3, 0.0);
        let land = frame.get(8 * 3 + 1, 8 * 3 + 1).expect("пиксель");
        assert!(land[1] > land[2], "трава должна быть зеленее синего: {land:?}");
    }

    #[test]
    fn snow_makes_the_tile_lighter() {
        let mut world = World::ocean(32, 32, 4);
        let spot = TilePos::new(16, 16);
        world.set_elevation(spot, SEA_LEVEL + 120);
        world.set_moisture(spot, 60);

        let frame_clear = render_tile(&world);
        world.set_snow(spot, 255);
        let frame_snowy = render_tile(&world);

        let clear = pixel_color(&frame_clear);
        let snowy = pixel_color(&frame_snowy);
        assert!(
            brightness(snowy) > brightness(clear),
            "снег должен осветлять тайл: {clear:?} → {snowy:?}"
        );
    }

    fn render_tile(world: &World) -> Frame {
        let mut frame = Frame::new(world.width() * 2, world.height() * 2);
        draw(&mut frame, world, &[], &[], 2, 0.0);
        frame
    }

    fn pixel_color(frame: &Frame) -> [u8; 3] {
        // Внутренний пиксель тайла (16,16) при 2 пикселях на тайл.
        frame.get(16 * 2 + 1, 16 * 2 + 1).expect("пиксель")
    }

    fn brightness(color: [u8; 3]) -> u32 {
        color.iter().map(|channel| *channel as u32).sum()
    }

    #[test]
    fn zero_pixels_per_tile_is_safe() {
        let world = World::ocean(8, 8, 1);
        let mut frame = Frame::new(8, 8);
        draw(&mut frame, &world, &[], &[], 0, 0.0);
    }

    #[test]
    fn territory_tints_tiles() {
        use worldgod_core::grid::pos;
        use worldgod_core::nation::{Nation, NATION_PALETTE};

        let mut world = World::ocean(16, 16, 4);
        world.set_terrain(pos(4, 4), Terrain::Grass);
        let nation = Nation::new(
            1,
            "Велдор".into(),
            NATION_PALETTE[4],
            pos(4, 4),
            0,
        );
        let mut territory = vec![0u8; world.len()];
        territory[4 * 16 + 4] = 1;

        let mut plain = Frame::new(16 * 3, 16 * 3);
        draw(&mut plain, &world, &[], &[], 3, 0.0);
        let mut owned = Frame::new(16 * 3, 16 * 3);
        draw(&mut owned, &world, &territory, &[nation], 3, 0.0);

        let plain_px = plain.get(4 * 3 + 1, 4 * 3 + 1).unwrap();
        let owned_px = owned.get(4 * 3 + 1, 4 * 3 + 1).unwrap();
        assert_ne!(plain_px, owned_px);
    }
}
