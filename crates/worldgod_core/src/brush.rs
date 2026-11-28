//! Кисти игрока: рельеф и заселение мира.
//!
//! Мазок — это круг с мягким краем: в центре сила максимальна, к границе
//! затухает, плюс немного шума, чтобы берега получались живыми, а не
//! «циркулем».

use crate::grid::{for_each_in_radius, TilePos};
use crate::rng::Rng;
use crate::species::Species;
use crate::terrain::{MAX_ELEVATION, SEA_LEVEL};
use crate::weather::Weather;
use crate::world::{Tile, World};

/// Инструмент в руке бога.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tool {
    /// Поднять сушу из воды.
    Raise,
    /// Опустить землю обратно в океан.
    Lower,
    /// Вырастить лес.
    Forest,
    /// Насыпать песчаный пляж.
    Sand,
    /// Поднять горы.
    Mountain,
    /// Поставить человека — он построит дом и основать страну.
    Human,
    /// Поставить воина-защитника.
    Soldier,
    /// Поставить оленя.
    Deer,
    /// Выпустить волка-охотника.
    Wolf,
    /// Выпустить зомби.
    Zombie,
    /// Рассмотреть существо или тайл.
    Inspect,
    /// Дипломатия: тапнуть по двум странам — объявить войну или союз.
    Diplomacy,
    /// Стихия: ураган.
    Hurricane,
    /// Стихия: вспышка солнца (жара и сушь).
    SolarFlare,
    /// Стихия: дождь.
    Rain,
    /// Стихия: снегопад.
    Snowfall,
}

/// Категория инструментов — вкладка нижней панели.
///
/// Категорий несколько, чтобы кисти не сваливались в одну длинную ленту:
/// рельеф меняет карту, жизнь заселяет её, стихии бьют по миру, а в «Прочем»
/// лежат осмотр и дипломатия.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToolCategory {
    /// Суша, вода, лес, пляж, горы.
    Terrain,
    /// Люди, воины, олени, волки, зомби.
    Life,
    /// Ураган, вспышка солнца, дождь, снегопад.
    Weather,
    /// Осмотр существ и кисть «Война / Мир».
    Other,
}

impl ToolCategory {
    /// Все категории в порядке вкладок.
    pub const ALL: [ToolCategory; 4] = [
        Self::Terrain,
        Self::Life,
        Self::Weather,
        Self::Other,
    ];

    /// Подпись вкладки.
    pub fn label_ru(self) -> &'static str {
        match self {
            Self::Terrain => "Рельеф",
            Self::Life => "Жизнь",
            Self::Weather => "Стихии",
            Self::Other => "Прочее",
        }
    }

    /// Какие инструменты входят в категорию.
    pub fn tools(self) -> &'static [Tool] {
        match self {
            Self::Terrain => &Tool::TERRAIN,
            Self::Life => &Tool::SPAWN,
            Self::Weather => &Tool::WEATHER,
            Self::Other => &Tool::OTHER,
        }
    }

    /// Категория, в которой лежит инструмент.
    pub fn of(tool: Tool) -> Self {
        tool.category()
    }
}

impl Tool {
    /// Все инструменты по категориям (в порядке вкладок).
    pub const ALL: [Tool; 16] = [
        Self::Raise,
        Self::Lower,
        Self::Forest,
        Self::Sand,
        Self::Mountain,
        Self::Human,
        Self::Soldier,
        Self::Deer,
        Self::Wolf,
        Self::Zombie,
        Self::Hurricane,
        Self::SolarFlare,
        Self::Rain,
        Self::Snowfall,
        Self::Inspect,
        Self::Diplomacy,
    ];

    /// Кисти рельефа.
    pub const TERRAIN: [Tool; 5] = [
        Self::Raise,
        Self::Lower,
        Self::Forest,
        Self::Sand,
        Self::Mountain,
    ];

    /// Кисти заселения.
    pub const SPAWN: [Tool; 5] = [
        Self::Human,
        Self::Soldier,
        Self::Deer,
        Self::Wolf,
        Self::Zombie,
    ];

    /// Стихии.
    pub const WEATHER: [Tool; 4] = [
        Self::Hurricane,
        Self::SolarFlare,
        Self::Rain,
        Self::Snowfall,
    ];

    /// Осмотр и дипломатия.
    pub const OTHER: [Tool; 2] = [Self::Inspect, Self::Diplomacy];

    /// Категория инструмента.
    pub fn category(self) -> ToolCategory {
        match self {
            Self::Raise | Self::Lower | Self::Forest | Self::Sand | Self::Mountain => {
                ToolCategory::Terrain
            }
            Self::Human | Self::Soldier | Self::Deer | Self::Wolf | Self::Zombie => {
                ToolCategory::Life
            }
            Self::Hurricane | Self::SolarFlare | Self::Rain | Self::Snowfall => ToolCategory::Weather,
            Self::Inspect | Self::Diplomacy => ToolCategory::Other,
        }
    }

    /// Подпись на кнопке.
    pub fn label_ru(self) -> &'static str {
        match self {
            Self::Raise => "Суша",
            Self::Lower => "Вода",
            Self::Forest => "Лес",
            Self::Sand => "Пляж",
            Self::Mountain => "Горы",
            Self::Human => "Люди",
            Self::Soldier => "Воины",
            Self::Deer => "Олени",
            Self::Wolf => "Волки",
            Self::Zombie => "Зомби",
            Self::Inspect => "Осмотр",
            Self::Diplomacy => "Война / Мир",
            Self::Hurricane => "Ураган",
            Self::SolarFlare => "Вспышка",
            Self::Rain => "Дождь",
            Self::Snowfall => "Снег",
        }
    }

    /// Короткая подсказка под кнопкой.
    pub fn hint_ru(self) -> &'static str {
        match self {
            Self::Raise => "Поднять сушу из океана",
            Self::Lower => "Утопить землю",
            Self::Forest => "Вырастить лес: дерево и еда",
            Self::Sand => "Насыпать пляж",
            Self::Mountain => "Поднять горы (непроходимы)",
            Self::Human => "Человек основывает страну",
            Self::Soldier => "Воин защищает страну",
            Self::Deer => "Олень — добыча волков",
            Self::Wolf => "Волк охотится на людей",
            Self::Zombie => "Зомби заражает убитых",
            Self::Inspect => "Тапните по существу",
            Self::Diplomacy => "Тапните по двум странам: война → союз → мир",
            Self::Hurricane => Weather::Hurricane.hint_ru(),
            Self::SolarFlare => Weather::SolarFlare.hint_ru(),
            Self::Rain => Weather::Rain.hint_ru(),
            Self::Snowfall => Weather::Snowfall.hint_ru(),
        }
    }

    /// Цвет иконки и подсветки.
    pub fn accent_rgb(self) -> [u8; 3] {
        match self {
            Self::Raise => [190, 170, 110],
            Self::Lower => [86, 160, 210],
            Self::Forest => [96, 160, 92],
            Self::Sand => [224, 206, 150],
            Self::Mountain => [168, 170, 176],
            Self::Human => [236, 208, 168],
            Self::Soldier => [150, 180, 220],
            Self::Deer => [186, 140, 92],
            Self::Wolf => [150, 156, 168],
            Self::Zombie => [146, 190, 120],
            Self::Inspect => [200, 200, 200],
            Self::Diplomacy => [224, 120, 96],
            Self::Hurricane => Weather::Hurricane.accent_rgb(),
            Self::SolarFlare => Weather::SolarFlare.accent_rgb(),
            Self::Rain => Weather::Rain.accent_rgb(),
            Self::Snowfall => Weather::Snowfall.accent_rgb(),
        }
    }

    /// Инструмент дипломатии (работает не с рельефом, а со странами).
    pub fn is_diplomacy(self) -> bool {
        self == Self::Diplomacy
    }

    /// Это кисть рельефа?
    pub fn is_terrain(self) -> bool {
        matches!(
            self,
            Self::Raise | Self::Lower | Self::Forest | Self::Sand | Self::Mountain
        )
    }

    /// Это стихия?
    pub fn is_weather(self) -> bool {
        self.weather().is_some()
    }

    /// Какая стихия применяется этим инструментом.
    pub fn weather(self) -> Option<Weather> {
        match self {
            Self::Hurricane => Some(Weather::Hurricane),
            Self::SolarFlare => Some(Weather::SolarFlare),
            Self::Rain => Some(Weather::Rain),
            Self::Snowfall => Some(Weather::Snowfall),
            _ => None,
        }
    }

    /// Кисть работает кругом с радиусом (рельеф и стихии).
    pub fn is_paint(self) -> bool {
        self.is_terrain() || self.is_weather()
    }

    /// Существо, которое ставит инструмент.
    pub fn species(self) -> Option<Species> {
        match self {
            Self::Human => Some(Species::Human),
            Self::Soldier => Some(Species::Soldier),
            Self::Deer => Some(Species::Deer),
            Self::Wolf => Some(Species::Wolf),
            Self::Zombie => Some(Species::Zombie),
            _ => None,
        }
    }

    /// Сколько существ появляется за одно нажатие.
    pub fn spawn_count(self) -> u32 {
        match self {
            Self::Wolf | Self::Zombie => 3,
            _ => 1,
        }
    }
}

/// Текущая кисть и её размер.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolConfig {
    /// Инструмент.
    pub tool: Tool,
    /// Радиус в тайлах.
    pub radius: i32,
}

impl ToolConfig {
    /// Минимальный радиус.
    pub const MIN_RADIUS: i32 = 1;
    /// Максимальный радиус.
    pub const MAX_RADIUS: i32 = 14;

    /// Конфигурация с заданным инструментом и радиусом.
    pub const fn new(tool: Tool, radius: i32) -> Self {
        Self { tool, radius }
    }

    /// Радиус, зажатый в допустимых пределах.
    pub fn clamped_radius(self) -> i32 {
        self.radius.clamp(Self::MIN_RADIUS, Self::MAX_RADIUS)
    }

    /// Копия с другим радиусом.
    pub fn with_radius(mut self, radius: i32) -> Self {
        self.radius = radius.clamp(Self::MIN_RADIUS, Self::MAX_RADIUS);
        self
    }

    /// Подпись текущего режима для строки состояния.
    pub fn status_ru(self) -> String {
        if self.tool.is_paint() {
            format!("{} · радиус {}", self.tool.label_ru(), self.clamped_radius())
        } else {
            self.tool.label_ru().to_string()
        }
    }
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            tool: Tool::Raise,
            radius: 4,
        }
    }
}

/// Применяет кисть рельефа. Возвращает число изменённых тайлов.
pub fn paint(world: &mut World, config: ToolConfig, center: TilePos, rng: &mut Rng) -> u32 {
    if !config.tool.is_terrain() {
        return 0;
    }

    let radius = config.clamped_radius();
    let tool = config.tool;
    let mut changed = 0u32;

    world.for_each_in_radius_mut(center, radius, |_p, distance, tile| {
        let falloff = 1.0 - distance;
        // Немного шума по краю: сила мазка «дышит».
        let jitter = 0.7 + rng.next_f32() * 0.5;
        let strength = (falloff * jitter).clamp(0.0, 1.0);
        if strength <= 0.04 {
            return;
        }
        let touched = match tool {
            Tool::Raise => raise_tile(tile, strength),
            Tool::Lower => lower_tile(tile, strength),
            Tool::Forest => forest_tile(tile, strength),
            Tool::Sand => sand_tile(tile, strength),
            Tool::Mountain => mountain_tile(tile, strength),
            _ => false,
        };
        if touched {
            changed += 1;
        }
    });

    // Береговой шельф: вода рядом с новой сушей становится мелководьем,
    // иначе берег выглядит обрывом в бездну.
    if matches!(tool, Tool::Raise | Tool::Forest | Tool::Sand | Tool::Mountain) {
        let mut shelf = Vec::new();
        for_each_in_radius(center, radius + 2, |p, _| {
            if world.is_water(p) && world.land_neighbors(p) > 0 {
                shelf.push(p);
            }
        });
        for tile in shelf {
            let target = world.elevation(tile).max(SEA_LEVEL - 35);
            world.set_elevation(tile, target);
        }
    }

    changed
}

/// Поднимает тайл: из воды появляется суша, из суши — холмы и горы.
fn raise_tile(tile: &mut Tile, strength: f32) -> bool {
    let before = *tile;
    let elevation = tile.elevation as i32;
    let sea = SEA_LEVEL as i32;

    let target = if elevation < sea {
        // Первый мазок создаёт берег сразу, без «полузатопленных» состояний.
        sea + 8 + (strength * 58.0) as i32
    } else {
        elevation + 3 + (strength * 28.0) as i32
    };

    tile.elevation = target.clamp(0, MAX_ELEVATION as i32) as u16;
    if tile.elevation > SEA_LEVEL {
        // Новая суша — луг, а не сразу лес.
        tile.moisture = tile.moisture.min(120);
    }
    *tile != before
}

/// Опускает тайл: земля уходит под воду, а вода становится глубже.
fn lower_tile(tile: &mut Tile, strength: f32) -> bool {
    let before = *tile;
    let elevation = tile.elevation as i32;
    let sea = SEA_LEVEL as i32;

    let target = if elevation >= sea {
        sea - 4 - (strength * 62.0) as i32
    } else {
        elevation - 18 - (strength * 80.0) as i32
    };

    tile.elevation = target.clamp(0, MAX_ELEVATION as i32) as u16;
    tile.moisture = 0;
    *tile != before
}

/// Лес: поднимает сушу, если нужно, и включает влажность.
fn forest_tile(tile: &mut Tile, strength: f32) -> bool {
    let before = *tile;
    if tile.elevation < SEA_LEVEL + 60 {
        let target = SEA_LEVEL as i32 + 90 + (strength * 70.0) as i32;
        tile.elevation = (target as u16).min(MAX_ELEVATION);
    }
    let moisture = (150.0 + strength * 105.0) as u8;
    tile.moisture = tile.moisture.max(moisture);
    *tile != before
}

/// Пляж: выравнивает тайл до уровня песка.
fn sand_tile(tile: &mut Tile, strength: f32) -> bool {
    let before = *tile;
    let target = SEA_LEVEL as i32 + 6 + (strength * 26.0) as i32;
    tile.elevation = (target as u16).min(MAX_ELEVATION);
    tile.moisture = 5;
    *tile != before
}

/// Горы: в центре кисти — горы, по краю — холмы.
fn mountain_tile(tile: &mut Tile, strength: f32) -> bool {
    let before = *tile;
    let target = 640.0 + strength * 205.0;
    tile.elevation = (target as u16).min(MAX_ELEVATION);
    tile.moisture = 20;
    *tile != before
}

/// Координаты всех тайлов под кистью — полезно для подсветки курсора в UI.
pub fn brush_tiles(center: TilePos, radius: i32) -> Vec<TilePos> {
    let mut tiles = Vec::new();
    for_each_in_radius(center, radius.clamp(0, 64), |p, _| tiles.push(p));
    tiles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::Terrain;
    use crate::world::World;

    fn ocean() -> World {
        World::ocean(48, 48, 12345)
    }

    #[test]
    fn raise_creates_land_immediately() {
        let mut world = ocean();
        let mut rng = Rng::new(1);
        let changed = paint(
            &mut world,
            ToolConfig::new(Tool::Raise, 3),
            TilePos::new(24, 24),
            &mut rng,
        );
        assert!(changed > 0);
        assert!(world.is_walkable(TilePos::new(24, 24)));
        assert!(world.land_fraction() > 0.0);
    }

    #[test]
    fn lower_sinks_land() {
        let mut world = ocean();
        let mut rng = Rng::new(2);
        paint(
            &mut world,
            ToolConfig::new(Tool::Raise, 4),
            TilePos::new(10, 10),
            &mut rng,
        );
        assert!(world.is_land(TilePos::new(10, 10)));
        paint(
            &mut world,
            ToolConfig::new(Tool::Lower, 4),
            TilePos::new(10, 10),
            &mut rng,
        );
        assert!(world.is_water(TilePos::new(10, 10)));
    }

    #[test]
    fn forest_brush_makes_forest() {
        let mut world = ocean();
        let mut rng = Rng::new(3);
        paint(
            &mut world,
            ToolConfig::new(Tool::Forest, 2),
            TilePos::new(20, 20),
            &mut rng,
        );
        assert_eq!(world.terrain(TilePos::new(20, 20)), Terrain::Forest);
    }

    #[test]
    fn mountain_brush_is_impassable() {
        let mut world = ocean();
        let mut rng = Rng::new(4);
        paint(
            &mut world,
            ToolConfig::new(Tool::Mountain, 1),
            TilePos::new(30, 30),
            &mut rng,
        );
        let terrain = world.terrain(TilePos::new(30, 30));
        assert!(
            matches!(terrain, Terrain::Mountain | Terrain::Snow | Terrain::Hills),
            "неожиданный рельеф: {terrain:?}"
        );
    }

    #[test]
    fn sand_brush_is_beach() {
        let mut world = ocean();
        let mut rng = Rng::new(5);
        paint(
            &mut world,
            ToolConfig::new(Tool::Sand, 1),
            TilePos::new(5, 5),
            &mut rng,
        );
        assert_eq!(world.terrain(TilePos::new(5, 5)), Terrain::Beach);
    }

    #[test]
    fn brush_stays_inside_map() {
        let mut world = ocean();
        let mut rng = Rng::new(6);
        // Мазок в углу не должен паниковать и не должен «заворачиваться».
        paint(
            &mut world,
            ToolConfig::new(Tool::Raise, 14),
            TilePos::new(0, 0),
            &mut rng,
        );
        assert!(world.is_land(TilePos::new(0, 0)));
        assert!(world.is_land(TilePos::new(3, 2)));
        // За границей карты ничего не появилось (проверяем через отсутствие паники).
        assert!(!world.in_bounds(TilePos::new(-1, -1)));
    }

    #[test]
    fn spawn_tools_have_species() {
        assert_eq!(Tool::Human.species(), Some(Species::Human));
        assert_eq!(Tool::Wolf.species(), Some(Species::Wolf));
        assert_eq!(Tool::Raise.species(), None);
        assert_eq!(Tool::Wolf.spawn_count(), 3);
        assert_eq!(Tool::Human.spawn_count(), 1);
        assert!(Tool::Forest.is_terrain());
        assert!(!Tool::Human.is_terrain());
    }

    #[test]
    fn weather_tools_map_to_weather() {
        assert_eq!(Tool::Hurricane.weather(), Some(Weather::Hurricane));
        assert_eq!(Tool::Snowfall.weather(), Some(Weather::Snowfall));
        assert_eq!(Tool::Raise.weather(), None);
        assert!(Tool::Rain.is_weather());
        assert!(Tool::Rain.is_paint());
        assert!(!Tool::Rain.is_terrain());
        assert!(!Tool::Human.is_paint());
        for tool in Tool::WEATHER {
            assert_eq!(tool.category(), ToolCategory::Weather);
            assert!(!tool.label_ru().is_empty());
            assert!(!tool.hint_ru().is_empty());
        }
    }

    #[test]
    fn categories_cover_every_tool_once() {
        let mut seen = Vec::new();
        for category in ToolCategory::ALL {
            assert!(!category.label_ru().is_empty());
            assert!(!category.tools().is_empty(), "{:?} пуста", category);
            for tool in category.tools() {
                assert!(!seen.contains(tool), "{tool:?} попал в две категории");
                seen.push(*tool);
                assert_eq!(tool.category(), category);
                assert_eq!(ToolCategory::of(*tool), category);
            }
        }
        assert_eq!(seen.len(), Tool::ALL.len(), "не все инструменты разложены");
        for tool in Tool::ALL {
            assert!(seen.contains(&tool), "{tool:?} не попал ни в одну категорию");
        }
    }

    #[test]
    fn radius_is_clamped() {
        let config = ToolConfig::new(Tool::Raise, 999);
        assert_eq!(config.clamped_radius(), ToolConfig::MAX_RADIUS);
        let config = ToolConfig::new(Tool::Raise, -5);
        assert_eq!(config.clamped_radius(), ToolConfig::MIN_RADIUS);
        assert_eq!(config.with_radius(7).radius, 7);
    }

    #[test]
    fn brush_tiles_cover_circle() {
        let tiles = brush_tiles(TilePos::new(5, 5), 2);
        assert!(tiles.contains(&TilePos::new(5, 5)));
        assert!(tiles.len() >= 9);
        assert!(tiles.iter().all(|p| p.chebyshev(TilePos::new(5, 5)) <= 2));
    }
}
