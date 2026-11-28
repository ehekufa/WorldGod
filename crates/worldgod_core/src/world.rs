//! Сетка мира: тайлы, высота, влажность и быстрые запросы по рельефу.

use crate::grid::{for_each_in_radius, TilePos};
use crate::noise;
use crate::rng::Rng;
use crate::terrain::{
    default_elevation, default_moisture, terrain_at, Terrain, MAX_ELEVATION, SEA_LEVEL,
};

/// Один тайл карты.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tile {
    /// Высота 0..[`MAX_ELEVATION`].
    pub elevation: u16,
    /// Влажность 0..255.
    pub moisture: u8,
    /// Декоративный вариант (0..3), чтобы трава и песок не выглядели одинаково.
    pub variant: u8,
}

impl Tile {
    /// Тип рельефа, вычисленный из высоты и влажности.
    pub fn terrain(&self) -> Terrain {
        terrain_at(self.elevation, self.moisture)
    }

    /// Это вода?
    pub fn is_water(&self) -> bool {
        self.terrain().is_water()
    }

    /// Это суша?
    pub fn is_land(&self) -> bool {
        self.terrain().is_land()
    }
}

/// Карта тайлов фиксированного размера.
///
/// Мир всегда начинается как океан: сушу создаёт игрок своей кистью.
#[derive(Clone, Debug)]
pub struct World {
    width: u32,
    height: u32,
    tiles: Vec<Tile>,
    land_tiles: u32,
    revision: u32,
    seed: u64,
}

impl World {
    /// Создаёт мир, полностью покрытый водой.
    ///
    /// Глубина слегка варьируется шумом, поэтому океан выглядит живым,
    /// но суши на карте нет совсем.
    pub fn ocean(width: u32, height: u32, seed: u64) -> Self {
        let width = width.max(16);
        let height = height.max(16);
        let count = (width as usize) * (height as usize);
        let mut tiles = Vec::with_capacity(count);
        let mut rng = Rng::new(seed ^ 0x5EED_0CEA_1234_5678);

        for y in 0..height {
            for x in 0..width {
                let depth = noise::fbm(x as f32 * 0.05, y as f32 * 0.05, seed, 4);
                // 40..295 — всегда ниже уровня моря: глубокие впадины и отмели.
                let elevation = (40.0 + depth * 255.0) as u16;
                let moisture =
                    (noise::fbm(x as f32 * 0.08 + 11.0, y as f32 * 0.08 - 7.0, seed, 3) * 255.0) as u8;
                tiles.push(Tile {
                    elevation: elevation.min(SEA_LEVEL - 25),
                    moisture,
                    variant: (rng.next_u32() % 4) as u8,
                });
            }
        }

        Self {
            width,
            height,
            tiles,
            land_tiles: 0,
            revision: 1,
            seed,
        }
    }

    /// Ширина карты в тайлах.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Высота карты в тайлах.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Размер карты `(ширина, высота)`.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Зерно, с которым мир был создан.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Счётчик изменений: растёт при любой правке тайла.
    ///
    /// Рендер использует его как «грязный флаг» для кэша.
    pub fn revision(&self) -> u32 {
        self.revision
    }

    /// Количество тайлов.
    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    /// Пустой мир? (всегда `false` для корректного мира)
    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    /// Все тайлы подряд, слева направо и сверху вниз.
    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }

    /// Проверяет, что координата внутри карты.
    pub fn in_bounds(&self, p: TilePos) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < self.width as i32 && p.y < self.height as i32
    }

    /// Индекс тайла в плоском массиве.
    pub fn index(&self, p: TilePos) -> Option<usize> {
        if self.in_bounds(p) {
            Some((p.y as usize) * (self.width as usize) + (p.x as usize))
        } else {
            None
        }
    }

    /// Координата по индексу.
    pub fn pos_of_index(&self, index: usize) -> TilePos {
        let w = self.width as usize;
        TilePos::new((index % w) as i32, (index / w) as i32)
    }

    /// Тайл или `None` за границей карты.
    pub fn tile(&self, p: TilePos) -> Option<&Tile> {
        self.index(p).map(|i| &self.tiles[i])
    }

    /// Тип рельефа; за границей — глубокий океан.
    pub fn terrain(&self, p: TilePos) -> Terrain {
        self.tile(p).map_or(Terrain::DeepWater, |t| t.terrain())
    }

    /// Высота тайла; за границей — 0.
    pub fn elevation(&self, p: TilePos) -> u16 {
        self.tile(p).map_or(0, |t| t.elevation)
    }

    /// Влажность тайла; за границей — 0.
    pub fn moisture(&self, p: TilePos) -> u8 {
        self.tile(p).map_or(0, |t| t.moisture)
    }

    /// Можно ли ходить по тайлу.
    pub fn is_walkable(&self, p: TilePos) -> bool {
        self.terrain(p).is_walkable()
    }

    /// Суша?
    pub fn is_land(&self, p: TilePos) -> bool {
        self.terrain(p).is_land()
    }

    /// Вода?
    pub fn is_water(&self, p: TilePos) -> bool {
        self.terrain(p).is_water()
    }

    /// Доля суши от всей карты.
    pub fn land_fraction(&self) -> f32 {
        if self.tiles.is_empty() {
            0.0
        } else {
            self.land_tiles as f32 / self.tiles.len() as f32
        }
    }

    /// Количество тайлов суши.
    pub fn land_tiles(&self) -> u32 {
        self.land_tiles
    }

    /// Подсчёт тайлов по типам рельефа.
    pub fn terrain_counts(&self) -> TerrainCounts {
        let mut counts = TerrainCounts::default();
        for tile in &self.tiles {
            counts.add(tile.terrain());
        }
        counts
    }

    /// Устанавливает высоту тайла. Возвращает `true`, если что-то изменилось.
    pub fn set_elevation(&mut self, p: TilePos, elevation: u16) -> bool {
        let Some(index) = self.index(p) else {
            return false;
        };
        let moisture = self.tiles[index].moisture;
        self.write_tile(index, elevation, moisture)
    }

    /// Устанавливает влажность тайла. Возвращает `true`, если что-то изменилось.
    pub fn set_moisture(&mut self, p: TilePos, moisture: u8) -> bool {
        let Some(index) = self.index(p) else {
            return false;
        };
        let elevation = self.tiles[index].elevation;
        self.write_tile(index, elevation, moisture)
    }

    /// Поднимает тайл на `amount` (с ограничением).
    pub fn raise(&mut self, p: TilePos, amount: u16) -> bool {
        let elevation = self.elevation(p).saturating_add(amount);
        self.set_elevation(p, elevation)
    }

    /// Опускает тайл на `amount`.
    pub fn lower(&mut self, p: TilePos, amount: u16) -> bool {
        let elevation = self.elevation(p).saturating_sub(amount);
        self.set_elevation(p, elevation)
    }

    /// Превращает тайл в указанный рельеф «в один клик».
    pub fn set_terrain(&mut self, p: TilePos, terrain: Terrain) -> bool {
        let elevation = default_elevation(terrain);
        let moisture = default_moisture(terrain);
        let Some(index) = self.index(p) else {
            return false;
        };
        self.write_tile(index, elevation, moisture)
    }

    /// Применяет функцию к каждому тайлу.
    pub fn for_each_tile_mut(&mut self, mut f: impl FnMut(TilePos, &mut Tile)) {
        let width = self.width as usize;
        let mut land_tiles = 0;
        for (index, tile) in self.tiles.iter_mut().enumerate() {
            let p = TilePos::new((index % width) as i32, (index / width) as i32);
            f(p, tile);
            tile.elevation = tile.elevation.min(MAX_ELEVATION);
            if tile.terrain().is_land() {
                land_tiles += 1;
            }
        }
        if land_tiles != self.land_tiles {
            self.land_tiles = land_tiles;
            self.bump_revision();
        }
    }

    /// Обходит тайлы в круге и передаёт изменяемую ссылку.
    pub fn for_each_in_radius_mut(
        &mut self,
        center: TilePos,
        radius: i32,
        mut f: impl FnMut(TilePos, f32, &mut Tile),
    ) {
        let width = self.width as usize;
        let height = self.height as usize;
        let mut land_tiles = self.land_tiles;
        let mut changed = false;
        let tiles = &mut self.tiles;

        for_each_in_radius(center, radius, |p, dist| {
            if p.x < 0 || p.y < 0 || p.x as usize >= width || p.y as usize >= height {
                return;
            }
            let index = (p.y as usize) * width + (p.x as usize);
            let tile = &mut tiles[index];
            let was_land = tile.terrain().is_land();
            f(p, dist, tile);
            tile.elevation = tile.elevation.min(MAX_ELEVATION);
            let is_land = tile.terrain().is_land();
            if was_land != is_land {
                if is_land {
                    land_tiles += 1;
                } else {
                    land_tiles = land_tiles.saturating_sub(1);
                }
                changed = true;
            }
        });

        self.land_tiles = land_tiles;
        if changed {
            self.bump_revision();
        }
    }

    /// Случайный проходимый тайл (до `tries` попыток).
    pub fn random_walkable(&self, rng: &mut Rng) -> Option<TilePos> {
        self.random_walkable_with(rng, 400)
    }

    /// Случайный проходимый тайл с настраиваемым числом попыток.
    pub fn random_walkable_with(&self, rng: &mut Rng, tries: u32) -> Option<TilePos> {
        for _ in 0..tries {
            let p = TilePos::new(
                rng.range(0, self.width as i32),
                rng.range(0, self.height as i32),
            );
            if self.is_walkable(p) {
                return Some(p);
            }
        }
        // Резервный вариант: линейный поиск, чтобы никогда не подвести.
        for (index, tile) in self.tiles.iter().enumerate() {
            if tile.terrain().is_walkable() {
                return Some(self.pos_of_index(index));
            }
        }
        None
    }

    /// Ближайший тайл, проходимый для ходьбы.
    pub fn nearest_walkable(&self, from: TilePos, max_radius: i32) -> Option<TilePos> {
        self.find_terrain_near(from, max_radius, |t| t.is_walkable())
    }

    /// Ближайший тайл, удовлетворяющий предикату (поиск по расширяющимся кольцам).
    pub fn find_terrain_near(
        &self,
        from: TilePos,
        max_radius: i32,
        pred: impl Fn(Terrain) -> bool,
    ) -> Option<TilePos> {
        if self.in_bounds(from) && pred(self.terrain(from)) {
            return Some(from);
        }
        for radius in 1..=max_radius.max(1) {
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx.abs() != radius && dy.abs() != radius {
                        continue;
                    }
                    let p = from.offset(dx, dy);
                    if self.in_bounds(p) && pred(self.terrain(p)) {
                        return Some(p);
                    }
                }
            }
        }
        None
    }

    /// Считает, сколько соседних тайлов — суша (для береговой линии и ИИ).
    pub fn land_neighbors(&self, p: TilePos) -> u8 {
        let mut count = 0;
        for neighbor in p.neighbors8() {
            if self.is_land(neighbor) {
                count += 1;
            }
        }
        count
    }

    fn write_tile(&mut self, index: usize, elevation: u16, moisture: u8) -> bool {
        let elevation = elevation.min(MAX_ELEVATION);
        let old = self.tiles[index];
        if old.elevation == elevation && old.moisture == moisture {
            return false;
        }

        let was_land = old.terrain().is_land();
        self.tiles[index] = Tile {
            elevation,
            moisture,
            variant: old.variant,
        };
        let is_land = self.tiles[index].terrain().is_land();
        match (was_land, is_land) {
            (false, true) => self.land_tiles += 1,
            (true, false) => self.land_tiles = self.land_tiles.saturating_sub(1),
            _ => {}
        }
        self.bump_revision();
        true
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}

/// Статистика по типам рельефа (используется в интерфейсе и тестах).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerrainCounts {
    counts: [u32; Terrain::COUNT],
}

impl TerrainCounts {
    /// Добавляет один тайл.
    pub fn add(&mut self, terrain: Terrain) {
        self.counts[terrain as usize] += 1;
    }

    /// Сколько тайлов заданного типа.
    pub fn get(&self, terrain: Terrain) -> u32 {
        self.counts[terrain as usize]
    }

    /// Всего тайлов.
    pub fn total(&self) -> u32 {
        self.counts.iter().sum()
    }

    /// Сумма по всем типам суши.
    pub fn land(&self) -> u32 {
        Terrain::ALL
            .iter()
            .filter(|t| t.is_land())
            .map(|t| self.get(*t))
            .sum()
    }

    /// Сумма по всем типам воды.
    pub fn water(&self) -> u32 {
        self.total() - self.land()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::pos;

    #[test]
    fn ocean_world_has_no_land() {
        let world = World::ocean(64, 48, 7);
        assert_eq!(world.land_tiles(), 0);
        assert_eq!(world.land_fraction(), 0.0);
        for p in [pos(0, 0), pos(10, 10), pos(63, 47)] {
            assert!(world.is_water(p));
            assert!(!world.is_walkable(p));
        }
    }

    #[test]
    fn raising_creates_land() {
        let mut world = World::ocean(32, 32, 1);
        assert!(!world.is_land(pos(10, 10)));
        world.set_terrain(pos(10, 10), Terrain::Grass);
        assert!(world.is_land(pos(10, 10)));
        assert!(world.is_walkable(pos(10, 10)));
        assert_eq!(world.land_tiles(), 1);

        world.set_terrain(pos(10, 10), Terrain::DeepWater);
        assert_eq!(world.land_tiles(), 0);
    }

    #[test]
    fn revision_grows_on_change() {
        let mut world = World::ocean(16, 16, 3);
        let start = world.revision();
        world.set_terrain(pos(4, 4), Terrain::Grass);
        assert!(world.revision() > start);
        let after = world.revision();
        // Повторная установка того же рельефа ничего не меняет.
        assert!(!world.set_terrain(pos(4, 4), Terrain::Grass));
        assert_eq!(world.revision(), after);
    }

    #[test]
    fn bounds_are_respected() {
        let world = World::ocean(16, 12, 5);
        assert!(world.in_bounds(pos(0, 0)));
        assert!(world.in_bounds(pos(15, 11)));
        assert!(!world.in_bounds(pos(16, 11)));
        assert!(!world.in_bounds(pos(-1, 0)));
        assert_eq!(world.terrain(pos(100, 100)), Terrain::DeepWater);
        assert!(world.index(pos(100, 100)).is_none());
    }

    #[test]
    fn nearest_walkable_finds_island() {
        let mut world = World::ocean(32, 32, 11);
        world.set_terrain(pos(16, 16), Terrain::Grass);
        assert_eq!(world.nearest_walkable(pos(10, 10), 12), Some(pos(16, 16)));
        assert_eq!(world.nearest_walkable(pos(0, 0), 3), None);
    }

    #[test]
    fn counts_add_up() {
        let mut world = World::ocean(20, 20, 2);
        world.set_terrain(pos(5, 5), Terrain::Grass);
        world.set_terrain(pos(6, 5), Terrain::Forest);
        world.set_terrain(pos(7, 5), Terrain::Mountain);
        let counts = world.terrain_counts();
        assert_eq!(counts.total(), 400);
        assert_eq!(counts.land(), 3);
        assert_eq!(counts.water(), 397);
        assert_eq!(counts.get(Terrain::Forest), 1);
    }

    #[test]
    fn brush_iteration_can_raise_tiles() {
        let mut world = World::ocean(24, 24, 4);
        world.for_each_in_radius_mut(pos(12, 12), 3, |_, _, tile| {
            tile.elevation = tile.elevation.max(SEA_LEVEL + 60);
            tile.moisture = 0;
        });
        assert!(world.is_walkable(pos(12, 12)));
        assert!(world.land_tiles() > 10);
    }
}
