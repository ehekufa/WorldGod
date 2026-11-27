//! Палитра мира: цвета рельефа, воды, построек и существ.
//!
//! Все цвета подобраны так, чтобы карта читалась даже на небольшом экране
//! телефона: холодная вода, тёплая суша, яркие монстры.

use worldgod_core::math::{brightness, mix_rgb};
use worldgod_core::Species;

/// Фон за пределами карты.
pub const BACKGROUND: [u8; 3] = [8, 16, 26];

/// Глубокий океан.
pub const DEEP_WATER: [u8; 3] = [12, 38, 64];
/// Открытый океан.
pub const OCEAN: [u8; 3] = [20, 66, 102];
/// Мелководье.
pub const SHALLOW_WATER: [u8; 3] = [38, 112, 140];
/// Пена у берега и блики на воде.
pub const FOAM: [u8; 3] = [154, 218, 224];

/// Песок.
pub const BEACH: [u8; 3] = [216, 198, 140];
/// Светлый песок.
pub const BEACH_LIGHT: [u8; 3] = [234, 218, 166];

/// Луг.
pub const GRASS: [u8; 3] = [92, 148, 78];
/// Тёмная трава.
pub const GRASS_DARK: [u8; 3] = [74, 124, 64];

/// Лес.
pub const FOREST: [u8; 3] = [54, 106, 64];
/// Тени в лесу.
pub const FOREST_DARK: [u8; 3] = [32, 74, 46];
/// Кроны деревьев.
pub const TREE: [u8; 3] = [70, 130, 72];
/// Светлые верхушки деревьев.
pub const TREE_LIGHT: [u8; 3] = [96, 158, 88];

/// Холмы.
pub const HILLS: [u8; 3] = [118, 122, 88];
/// Горы.
pub const MOUNTAIN: [u8; 3] = [126, 128, 134];
/// Светлые склоны гор.
pub const MOUNTAIN_LIGHT: [u8; 3] = [172, 174, 180];
/// Снег.
pub const SNOW: [u8; 3] = [234, 240, 246];

/// Кожа.
pub const SKIN: [u8; 3] = [238, 204, 162];
/// Тёмная кожа (загар).
pub const SKIN_DARK: [u8; 3] = [206, 166, 124];
/// Дерево.
pub const WOOD: [u8; 3] = [134, 98, 62];
/// Тёмное дерево.
pub const WOOD_DARK: [u8; 3] = [98, 70, 44];
/// Крыша.
pub const ROOF: [u8; 3] = [156, 78, 60];
/// Тёмная крыша.
pub const ROOF_DARK: [u8; 3] = [120, 58, 46];
/// Камень.
pub const STONE: [u8; 3] = [124, 120, 114];
/// Огонь.
pub const FLAME: [u8; 3] = [255, 178, 66];
/// Угли.
pub const EMBER: [u8; 3] = [228, 104, 46];
/// Тень под объектом.
pub const SHADOW: [u8; 3] = [10, 12, 16];

/// Шерсть волка.
pub const WOLF: [u8; 3] = [122, 128, 140];
/// Тёмная шерсть волка.
pub const WOLF_DARK: [u8; 3] = [80, 84, 94];
/// Плоть зомби.
pub const ZOMBIE: [u8; 3] = [126, 172, 98];
/// Тёмная плоть зомби.
pub const ZOMBIE_DARK: [u8; 3] = [82, 118, 66];
/// Шкура оленя.
pub const DEER: [u8; 3] = [168, 120, 74];
/// Светлая шкура оленя.
pub const DEER_LIGHT: [u8; 3] = [208, 174, 122];
/// Одежда человека без страны.
pub const NEUTRAL_CLOTH: [u8; 3] = [204, 196, 178];
/// Сталь доспеха.
pub const STEEL: [u8; 3] = [186, 198, 214];
/// Тёмная сталь.
pub const STEEL_DARK: [u8; 3] = [130, 144, 162];

/// Подсветка выбранного существа.
pub const SELECTION: [u8; 3] = [255, 255, 255];
/// Кровь.
pub const BLOOD: [u8; 3] = [148, 46, 42];

/// Базовый цвет рельефа.
pub fn terrain_color(terrain: worldgod_core::Terrain) -> [u8; 3] {
    use worldgod_core::Terrain;
    match terrain {
        Terrain::DeepWater => DEEP_WATER,
        Terrain::Ocean => OCEAN,
        Terrain::ShallowWater => SHALLOW_WATER,
        Terrain::Beach => BEACH,
        Terrain::Grass => GRASS,
        Terrain::Forest => FOREST,
        Terrain::Hills => HILLS,
        Terrain::Mountain => MOUNTAIN,
        Terrain::Snow => SNOW,
    }
}

/// Цвет одежды существа: жители носят цвет своей страны.
pub fn cloth_color(species: Species, nation_color: Option<[u8; 3]>) -> [u8; 3] {
    let base = nation_color.unwrap_or(NEUTRAL_CLOTH);
    match species {
        Species::Soldier => mix_rgb(base, STEEL, 0.55),
        Species::Human => base,
        _ => base,
    }
}

/// Осветляет цвет на `factor` (1.0 — без изменений).
pub fn lighten(color: [u8; 3], factor: f32) -> [u8; 3] {
    brightness(color, factor)
}

/// Смешивает цвета.
pub fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    mix_rgb(a, b, t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldgod_core::Terrain;

    #[test]
    fn every_terrain_has_color() {
        for terrain in Terrain::ALL {
            let color = terrain_color(terrain);
            assert!(color.iter().any(|channel| *channel > 0));
        }
    }

    #[test]
    fn soldier_cloth_is_steel_tinged() {
        let cloth = cloth_color(Species::Soldier, Some([200, 60, 60]));
        let plain = cloth_color(Species::Human, Some([200, 60, 60]));
        assert_ne!(cloth, plain);
    }

    #[test]
    fn color_helpers() {
        assert_eq!(lighten([100, 100, 100], 1.0), [100, 100, 100]);
        assert_eq!(mix([0, 0, 0], [100, 100, 100], 0.5), [50, 50, 50]);
    }
}
