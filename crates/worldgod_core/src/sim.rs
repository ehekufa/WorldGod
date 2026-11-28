//! Симуляция: владелец мира, существ, построек и стран.
//!
//! Один вызов [`Simulation::step`] — один тик. Приложение решает, сколько
//! тиков выполнить за кадр, исходя из выбранной скорости.

use crate::ai;
use crate::brush::{self, Tool, ToolConfig};
use crate::building::{Building, BuildingId, BuildingKind};
use crate::combat;
use crate::entity::{Entity, EntityId, EntityView, Intent};
use crate::events::{EventKind, EventLog};
use crate::grid::{for_each_in_radius, TilePos};
use crate::names;
use crate::nation::{Diplomacy, Nation, NationId, Relation, NATION_PALETTE};
use crate::pathfind::{greedy_step, PathFinder};
use crate::rng::Rng;
use crate::species::Species;
use crate::world::World;

/// Частота симуляции: 10 тиков в секунду реального времени.
pub const SIM_HZ: u32 = 10;

/// Размер мира.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldSize {
    /// Ширина в тайлах.
    pub width: u32,
    /// Высота в тайлах.
    pub height: u32,
}

impl WorldSize {
    /// Малый мир — быстрый и уютный (192×144).
    pub const SMALL: Self = Self {
        width: 192,
        height: 144,
    };
    /// Средний мир — рекомендуемый (256×192).
    pub const MEDIUM: Self = Self {
        width: 256,
        height: 192,
    };
    /// Большой мир — для долгих партий (352×264).
    pub const LARGE: Self = Self {
        width: 352,
        height: 264,
    };

    /// Все пресеты.
    pub const ALL: [WorldSize; 3] = [Self::SMALL, Self::MEDIUM, Self::LARGE];

    /// Подпись для интерфейса.
    pub fn label_ru(self) -> &'static str {
        match self {
            Self::SMALL => "Малый",
            Self::MEDIUM => "Средний",
            Self::LARGE => "Большой",
            _ => "Свой",
        }
    }

    /// Количество тайлов.
    pub fn tiles(self) -> usize {
        self.width as usize * self.height as usize
    }
}

impl Default for WorldSize {
    fn default() -> Self {
        Self::MEDIUM
    }
}

/// Скорость течения времени.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speed {
    /// Пауза.
    Paused,
    /// Обычная скорость.
    Normal,
    /// Ускорение ×2.
    Fast,
    /// Ускорение ×4.
    Turbo,
}

impl Speed {
    /// Все варианты по кругу.
    pub const ALL: [Speed; 4] = [Self::Paused, Self::Normal, Self::Fast, Self::Turbo];

    /// Множитель реального времени.
    pub fn multiplier(self) -> f32 {
        match self {
            Self::Paused => 0.0,
            Self::Normal => 1.0,
            Self::Fast => 2.0,
            Self::Turbo => 4.0,
        }
    }

    /// Подпись для интерфейса.
    pub fn label_ru(self) -> &'static str {
        match self {
            Self::Paused => "Пауза",
            Self::Normal => "Обычно",
            Self::Fast => "×2",
            Self::Turbo => "×4",
        }
    }

    /// Следующая скорость по кругу.
    pub fn next(self) -> Self {
        match self {
            Self::Paused => Self::Normal,
            Self::Normal => Self::Fast,
            Self::Fast => Self::Turbo,
            Self::Turbo => Self::Paused,
        }
    }

    /// Симуляция стоит?
    pub fn is_paused(self) -> bool {
        self == Self::Paused
    }
}

impl Default for Speed {
    fn default() -> Self {
        Self::Normal
    }
}

/// Текущее состояние мира для интерфейса.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    /// Люди (мирные).
    pub humans: u32,
    /// Воины.
    pub soldiers: u32,
    /// Олени.
    pub deer: u32,
    /// Волки.
    pub wolves: u32,
    /// Зомби.
    pub zombies: u32,
    /// Всего населения стран.
    pub population: u32,
    /// Монстров всего.
    pub monsters: u32,
    /// Готовые дома.
    pub houses: u32,
    /// Костры.
    pub campfires: u32,
    /// Руины.
    pub ruins: u32,
    /// Порты.
    pub ports: u32,
    /// Лодки.
    pub boats: u32,
    /// Идущие войны.
    pub wars: u32,
    /// Действующие союзы.
    pub alliances: u32,
    /// Стран.
    pub nations: u32,
    /// Доля суши 0.0..=1.0.
    pub land_fraction: f32,
    /// Тайлов суши.
    pub land_tiles: u32,
    /// Номер тика.
    pub tick: u64,
}

impl Stats {
    /// Сколько всего существ.
    pub fn creatures(&self) -> u32 {
        self.humans + self.soldiers + self.deer + self.wolves + self.zombies
    }
}

/// Накопленные за партию итоги.
#[derive(Clone, Copy, Debug, Default)]
pub struct Totals {
    /// Сколько родилось жителей.
    pub births: u64,
    /// Сколько существ погибло.
    pub deaths: u64,
    /// Сколько убийств совершено.
    pub kills: u64,
    /// Сколько стран основано.
    pub nations_founded: u64,
    /// Сколько домов построено.
    pub houses_built: u64,
}

/// Мир целиком: карта, существа, постройки, страны и история.
#[derive(Clone, Debug)]
pub struct Simulation {
    /// Карта тайлов.
    pub world: World,
    /// Все существа.
    pub entities: Vec<Entity>,
    /// Все постройки.
    pub buildings: Vec<Building>,
    /// Все страны.
    pub nations: Vec<Nation>,
    /// Лента событий.
    pub events: EventLog,
    /// Генератор случайных чисел симуляции.
    pub rng: Rng,
    /// Номер текущего тика.
    pub tick: u64,
    /// Скорость времени (её меняет игрок).
    pub speed: Speed,
    /// Свежая статистика (пересчитывается каждый тик).
    pub stats: Stats,
    /// Выбранное существо.
    pub selected: Option<EntityId>,
    /// Итоги партии.
    pub totals: Totals,
    /// Отношения между странами: мир, союз или война.
    pub diplomacy: Diplomacy,

    /// Владелец каждого тайла (0 — ничей, иначе id страны).
    territory: Vec<u8>,
    territory_dirty: bool,
    /// Занятые постройками тайлы.
    blocked: Vec<bool>,
    blocked_dirty: bool,
    pathfinder: PathFinder,
    next_entity_id: EntityId,
    next_building_id: BuildingId,
    next_nation_id: NationId,
    seed: u64,
}

impl Simulation {
    /// Бюджет раскрытий A* на один запрос пути.
    const PATH_BUDGET: u32 = 1_500;

    /// Сколько тиков между попытками рождения жителей.
    const BIRTH_INTERVAL: u64 = 60;

    /// Создаёт мир-океан без единого клочка суши.
    pub fn new(size: WorldSize, seed: u64) -> Self {
        let world = World::ocean(size.width, size.height, seed);
        let cells = world.len();

        let mut simulation = Self {
            world,
            entities: Vec::new(),
            buildings: Vec::new(),
            nations: Vec::new(),
            events: EventLog::default(),
            rng: Rng::new(seed),
            tick: 0,
            speed: Speed::Normal,
            stats: Stats::default(),
            selected: None,
            totals: Totals::default(),
            diplomacy: Diplomacy::new(),
            territory: vec![0; cells],
            territory_dirty: true,
            blocked: vec![false; cells],
            blocked_dirty: true,
            pathfinder: PathFinder::new(size.width, size.height),
            next_entity_id: 1,
            next_building_id: 1,
            next_nation_id: 1,
            seed,
        };

        simulation.refresh_stats();
        simulation.events.push(
            0,
            EventKind::Nature,
            "Океан пуст. Поднимите сушу кистью «Суша».",
        );
        simulation
    }

    /// Зерно мира.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Размер мира в тайлах.
    pub fn size(&self) -> (u32, u32) {
        self.world.size()
    }

    /// Начинает партию заново.
    pub fn reset(&mut self, size: WorldSize, seed: u64) {
        *self = Self::new(size, seed);
    }

    // ---------------------------------------------------------------- доступ

    /// Существо по идентификатору.
    pub fn entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == id)
    }

    /// Изменяемое существо по идентификатору.
    pub fn entity_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.iter_mut().find(|e| e.id == id)
    }

    /// Индекс существа в массиве.
    pub fn entity_index(&self, id: EntityId) -> Option<usize> {
        self.entities.iter().position(|e| e.id == id)
    }

    /// Постройка по идентификатору.
    pub fn building(&self, id: BuildingId) -> Option<&Building> {
        self.buildings.iter().find(|b| b.id == id)
    }

    /// Индекс постройки в массиве.
    pub fn building_index(&self, id: BuildingId) -> Option<usize> {
        self.buildings.iter().position(|b| b.id == id)
    }

    /// Постройка на тайле.
    pub fn building_at(&self, pos: TilePos) -> Option<&Building> {
        self.buildings.iter().find(|b| b.pos == pos)
    }

    /// Страна по идентификатору.
    pub fn nation(&self, id: NationId) -> Option<&Nation> {
        self.nations.iter().find(|n| n.id == id)
    }

    /// Изменяемая страна.
    pub fn nation_mut(&mut self, id: NationId) -> Option<&mut Nation> {
        self.nations.iter_mut().find(|n| n.id == id)
    }

    /// Цвет страны (или нейтральный серый).
    pub fn nation_color(&self, id: Option<NationId>) -> [u8; 3] {
        id.and_then(|id| self.nation(id))
            .map_or([190, 190, 184], |nation| nation.color)
    }

    /// Название страны или «без страны».
    pub fn nation_name(&self, id: Option<NationId>) -> &str {
        id.and_then(|id| self.nation(id))
            .map_or("Без страны", |nation| nation.name.as_str())
    }

    // ------------------------------------------------------------ дипломатия

    /// Отношения двух стран.
    pub fn relation(&self, a: NationId, b: NationId) -> Relation {
        self.diplomacy.get(a, b)
    }

    /// Между странами война?
    pub fn at_war(&self, a: NationId, b: NationId) -> bool {
        self.diplomacy.at_war(a, b)
    }

    /// Страны в союзе?
    pub fn allied(&self, a: NationId, b: NationId) -> bool {
        self.diplomacy.allied(a, b)
    }

    /// Воюют ли две страны (у существа без страны врагов нет).
    pub fn nations_at_war(&self, a: Option<NationId>, b: Option<NationId>) -> bool {
        match (a, b) {
            (Some(a), Some(b)) => a != b && self.diplomacy.at_war(a, b),
            _ => false,
        }
    }

    /// Страны в союзе?
    pub fn nations_allied(&self, a: Option<NationId>, b: Option<NationId>) -> bool {
        match (a, b) {
            (Some(a), Some(b)) => a != b && self.diplomacy.allied(a, b),
            _ => false,
        }
    }

    /// Меняет отношения двух стран и пишет об этом в ленту событий.
    ///
    /// Возвращает новый статус или `None`, если такой страны нет.
    pub fn set_relation(
        &mut self,
        a: NationId,
        b: NationId,
        relation: Relation,
    ) -> Option<Relation> {
        if a == b || self.nation(a).is_none() || self.nation(b).is_none() {
            return None;
        }
        self.diplomacy.ensure(self.nations.len());
        if self.diplomacy.get(a, b) == relation {
            return Some(relation);
        }
        self.diplomacy.set(a, b, relation);

        let name_a = self.nation_name(Some(a)).to_string();
        let name_b = self.nation_name(Some(b)).to_string();
        let text = match relation {
            Relation::War => format!("{name_a} объявила войну {name_b}"),
            Relation::Alliance => format!("{name_a} и {name_b} заключили союз против монстров"),
            Relation::Peace => format!("{name_a} и {name_b} заключили мир"),
        };
        self.events.push(self.tick, EventKind::Nation, text);
        self.refresh_stats();
        Some(relation)
    }

    /// Переключает статус пары по кругу: мир → война → союз → мир.
    pub fn cycle_relation(&mut self, a: NationId, b: NationId) -> Option<Relation> {
        let next = self.relation(a, b).next();
        self.set_relation(a, b, next)
    }

    /// Страна, которой принадлежит тайл (для кисти «Война / Мир»).
    ///
    /// Сначала смотрим карту владений, а если она ещё не пересчитана —
    /// ближайшую постройку.
    pub fn nation_at(&self, pos: TilePos) -> Option<NationId> {
        if let Some(owner) = self.territory_owner(pos) {
            return Some(owner);
        }
        self.buildings
            .iter()
            .filter(|building| {
                building.nation.is_some()
                    && building.kind.is_permanent()
                    && building.pos.chebyshev(pos) <= 4
            })
            .min_by_key(|building| building.pos.chebyshev(pos))
            .and_then(|building| building.nation)
    }

    // ------------------------------------------------------------ вражда

    /// Враждебны ли два существа с учётом дипломатии.
    ///
    /// Монстры враждуют с людьми всегда, люди двух стран — только на войне,
    /// а лодку может разломать любой чужой житель воюющей страны.
    pub fn hostile_views(&self, attacker: &EntityView, target: &EntityView) -> bool {
        if attacker.id == target.id {
            return false;
        }
        let a = attacker.species;
        let b = target.species;

        if a.is_boat() || b.is_boat() {
            let (boat, other) = if a.is_boat() {
                (attacker, target)
            } else {
                (target, attacker)
            };
            if !other.species.is_humanoid() {
                return false;
            }
            return self.nations_at_war(boat.nation, other.nation);
        }

        if a.is_monster() || b.is_monster() {
            return a.hostile_to(b);
        }

        if a.is_humanoid() && b.is_humanoid() {
            return self.nations_at_war(attacker.nation, target.nation);
        }

        false
    }

    /// То же самое, но для самих существ.
    pub fn hostile(&self, attacker: &Entity, target: &Entity) -> bool {
        self.hostile_views(&attacker.view(), &target.view())
    }

    /// Ближайший враг с учётом дипломатии.
    pub fn nearest_hostile_view(&self, me: &EntityView, radius: i32) -> Option<EntityId> {
        let mut best: Option<(i32, EntityId)> = None;
        for other in &self.entities {
            if !other.alive || other.id == me.id {
                continue;
            }
            if !self.hostile_views(me, &other.view()) {
                continue;
            }
            let distance = me.pos.chebyshev(other.pos);
            if distance > radius {
                continue;
            }
            if best.map_or(true, |(best_distance, _)| distance < best_distance) {
                best = Some((distance, other.id));
            }
        }
        best.map(|(_, id)| id)
    }

    /// Ближайший враг для конкретного существа.
    pub fn nearest_hostile(&self, me: &Entity, radius: i32) -> Option<EntityId> {
        self.nearest_hostile_view(&me.view(), radius)
    }

    /// Монстр, который угрожает жителям страны-союзника рядом с нами.
    ///
    /// Так работают союзы: воины одной страны приходят на помощь другой.
    pub fn nearest_threat_to_allies(&self, me: &EntityView, radius: i32) -> Option<EntityId> {
        let Some(my_nation) = me.nation else {
            return None;
        };
        let mut best: Option<(i32, EntityId)> = None;
        for ally in &self.entities {
            if !ally.alive || !ally.species.is_humanoid() {
                continue;
            }
            let Some(ally_nation) = ally.nation else {
                continue;
            };
            if ally_nation == my_nation || !self.diplomacy.allied(my_nation, ally_nation) {
                continue;
            }
            if me.pos.chebyshev(ally.pos) > radius {
                continue;
            }
            for monster in &self.entities {
                if !monster.alive || !monster.species.is_monster() {
                    continue;
                }
                if ally.pos.chebyshev(monster.pos) > 6 {
                    continue;
                }
                let distance = me.pos.chebyshev(monster.pos);
                if best.map_or(true, |(best_distance, _)| distance < best_distance) {
                    best = Some((distance, monster.id));
                }
            }
        }
        best.map(|(_, id)| id)
    }

    /// Ближайшая постройка страны, с которой мы воюем.
    pub fn nearest_enemy_building(
        &self,
        pos: TilePos,
        radius: i32,
        me: Option<NationId>,
    ) -> Option<BuildingId> {
        let me = me?;
        let mut best: Option<(i32, BuildingId)> = None;
        for building in &self.buildings {
            let Some(owner) = building.nation else {
                continue;
            };
            if owner == me || !self.diplomacy.at_war(me, owner) {
                continue;
            }
            if !building.is_complete() || !building.kind.is_permanent() {
                continue;
            }
            let distance = pos.chebyshev(building.pos);
            if distance > radius {
                continue;
            }
            if best.map_or(true, |(best_distance, _)| distance < best_distance) {
                best = Some((distance, building.id));
            }
        }
        best.map(|(_, id)| id)
    }

    /// Есть ли у страны хоть один противник.
    pub fn nation_at_war_with_anyone(&self, nation: NationId) -> bool {
        self.diplomacy
            .pairs(Relation::War)
            .iter()
            .any(|(a, b)| *a == nation || *b == nation)
    }

    /// Сколько воинов рядом: своих и союзных.
    pub fn count_allied_soldiers_near(
        &self,
        pos: TilePos,
        radius: i32,
        nation: Option<NationId>,
    ) -> u32 {
        self.entities
            .iter()
            .filter(|entity| {
                if !entity.alive || entity.species != Species::Soldier {
                    return false;
                }
                if entity.nation == nation {
                    return pos.chebyshev(entity.pos) <= radius;
                }
                self.nations_allied(nation, entity.nation) && pos.chebyshev(entity.pos) <= radius
            })
            .count() as u32
    }

    // ------------------------------------------------------------ вода и суша

    /// Может ли вид стоять на этом тайле.
    pub fn is_passable_for(&self, species: Species, pos: TilePos) -> bool {
        if species.is_water_unit() {
            return self.world.is_water(pos) && !self.is_blocked_tile(pos);
        }
        self.is_walkable(pos)
    }

    /// Путь с учётом стихии вида: лодки плывут по воде, остальные ходят по суше.
    pub fn find_path_for(
        &mut self,
        species: Species,
        from: TilePos,
        to: TilePos,
        budget: u32,
    ) -> Option<Vec<TilePos>> {
        let world = &self.world;
        let blocked = &self.blocked;
        let width = world.width() as usize;
        let water = species.is_water_unit();
        let walkable = |p: TilePos| -> bool {
            let passable = if water {
                world.is_water(p)
            } else {
                world.is_walkable(p)
            };
            if !passable {
                return false;
            }
            let index = (p.y as usize) * width + (p.x as usize);
            !blocked.get(index).copied().unwrap_or(false)
        };
        self.pathfinder.find(from, to, walkable, budget)
    }

    /// Ближайшая вода — для лодок.
    pub fn nearest_water(&self, from: TilePos, radius: i32) -> Option<TilePos> {
        self.world
            .find_terrain_near(from, radius, |terrain| terrain.is_water())
    }

    /// Ставит лодку страны на воду рядом с точкой.
    pub fn spawn_boat(&mut self, near: TilePos, nation: NationId) -> Option<EntityId> {
        let spot = self.nearest_water(near, 10)?;
        let id = self.push_entity(Species::Boat, spot);
        if let Some(entity) = self.entity_mut(id) {
            entity.nation = Some(nation);
        }
        self.refresh_stats();
        Some(id)
    }

    /// Место под порт: ровная земля у самой воды рядом со столицей.
    pub fn find_port_site(&self, nation: NationId) -> Option<TilePos> {
        let capital = self.nation(nation).map_or(TilePos::ZERO, |nation| nation.capital);
        let mut best: Option<(i32, TilePos)> = None;
        for dy in -14..=14 {
            for dx in -14..=14 {
                let tile = capital.offset(dx, dy);
                if !self.world.terrain(tile).is_buildable() {
                    continue;
                }
                if self.building_at(tile).is_some() {
                    continue;
                }
                if !tile.neighbors4().any(|near| self.world.is_water(near)) {
                    continue;
                }
                let distance = capital.chebyshev(tile);
                if best.map_or(true, |(best_distance, _)| distance < best_distance) {
                    best = Some((distance, tile));
                }
            }
        }
        best.map(|(_, tile)| tile)
    }

    /// Выбранное существо.
    pub fn selected_entity(&self) -> Option<&Entity> {
        self.selected.and_then(|id| self.entity(id))
    }

    /// Выбирает существо (или снимает выбор).
    pub fn select(&mut self, id: Option<EntityId>) {
        self.selected = id;
    }

    /// Владелец тайла.
    pub fn territory_owner(&self, pos: TilePos) -> Option<NationId> {
        self.world.index(pos).and_then(|index| {
            let owner = *self.territory.get(index)?;
            if owner == 0 {
                None
            } else {
                Some(owner as NationId)
            }
        })
    }

    /// Карта владений (0 — ничья земля).
    pub fn territory(&self) -> &[u8] {
        &self.territory
    }

    /// Тайл занят постройкой, по которой нельзя ходить.
    pub fn is_blocked_tile(&self, pos: TilePos) -> bool {
        self.world
            .index(pos)
            .and_then(|index| self.blocked.get(index).copied())
            .unwrap_or(false)
    }

    /// Можно ли встать на тайл существу.
    pub fn is_walkable(&self, pos: TilePos) -> bool {
        self.world.is_walkable(pos) && !self.is_blocked_tile(pos)
    }

    // ---------------------------------------------------------------- путь

    /// Ищет путь с учётом рельефа и построек.
    pub fn find_path(&mut self, from: TilePos, to: TilePos, budget: u32) -> Option<Vec<TilePos>> {
        let world = &self.world;
        let blocked = &self.blocked;
        let width = world.width() as usize;
        let walkable = |p: TilePos| -> bool {
            if !world.is_walkable(p) {
                return false;
            }
            let index = (p.y as usize) * width + (p.x as usize);
            !blocked.get(index).copied().unwrap_or(false)
        };
        self.pathfinder.find(from, to, walkable, budget)
    }

    /// Страна по тайлу: ставит костёр и сразу регистрирует страну в мире.
    ///
    /// Нужно и кисти «Страна», и тестам: без сущности-основателя
    /// [`Simulation::found_nation`] не вызовешь.
    pub fn found_nation_at(&mut self, pos: TilePos) -> NationId {
        let id = self.next_nation_id;
        self.next_nation_id += 1;

        let taken: Vec<String> = self.nations.iter().map(|nation| nation.name.clone()).collect();
        let name = names::nation_name(&mut self.rng, &taken);
        let color = NATION_PALETTE[(id as usize - 1) % NATION_PALETTE.len()];
        self.nations
            .push(Nation::new(id, name.clone(), color, pos, self.tick));

        let building_id = self.next_building_id;
        self.next_building_id += 1;
        self.buildings.push(Building::finished(
            building_id,
            BuildingKind::Campfire,
            pos,
            Some(id),
            self.tick,
        ));

        self.totals.nations_founded += 1;
        self.blocked_dirty = true;
        self.territory_dirty = true;
        self.diplomacy.ensure(self.nations.len());
        self.refresh_stats();
        self.events.push(
            self.tick,
            EventKind::Nation,
            format!("{name}: основана новая страна"),
        );
        id
    }

    /// Ставит готовую постройку страны (для сцен, кистей и тестов).
    pub fn finish_building(
        &mut self,
        kind: BuildingKind,
        pos: TilePos,
        nation: NationId,
    ) -> BuildingId {
        let id = self.next_building_id;
        self.next_building_id += 1;
        self.buildings.push(Building::finished(
            id,
            kind,
            pos,
            Some(nation),
            self.tick,
        ));
        self.blocked_dirty = true;
        self.territory_dirty = true;
        id
    }

    /// Ставит готовый дом страны.
    pub fn finish_house_for(&mut self, nation: NationId, pos: TilePos) -> BuildingId {
        self.finish_building(BuildingKind::House, pos, nation)
    }

    /// Ближайший враг для существа по его идентификатору.
    pub fn nearest_hostile_to_id(&self, id: EntityId, radius: i32) -> Option<EntityId> {
        let entity = self.entity(id)?;
        self.nearest_hostile(entity, radius)
    }

    /// Случайная вода рядом с центром — для лодок.
    pub fn random_water_near(&mut self, center: TilePos, radius: i32) -> Option<TilePos> {
        for _ in 0..24 {
            let candidate = self.rng.tile_in_square(center, radius);
            if self.world.is_water(candidate) {
                return Some(candidate);
            }
        }
        self.nearest_water(center, radius.max(4))
    }

    /// Случайная проходимая точка рядом с центром.
    pub fn random_walkable_near(&mut self, center: TilePos, radius: i32) -> Option<TilePos> {
        for _ in 0..24 {
            let candidate = self.rng.tile_in_square(center, radius);
            if self.is_walkable(candidate) {
                return Some(candidate);
            }
        }
        self.world.nearest_walkable(center, radius.max(4))
    }

    // ---------------------------------------------------------------- поиск

    /// Ближайшее существо, удовлетворяющее условию.
    pub fn nearest_of(
        &self,
        pos: TilePos,
        radius: i32,
        pred: impl Fn(&Entity) -> bool,
    ) -> Option<EntityId> {
        let mut best: Option<(i32, EntityId)> = None;
        for entity in &self.entities {
            if !entity.alive || !pred(entity) {
                continue;
            }
            let distance = pos.chebyshev(entity.pos);
            if distance > radius {
                continue;
            }
            if best.map_or(true, |(best_distance, _)| distance < best_distance) {
                best = Some((distance, entity.id));
            }
        }
        best.map(|(_, id)| id)
    }

    /// Ближайший враг для вида `me`.
    pub fn nearest_enemy(&self, pos: TilePos, radius: i32, me: Species) -> Option<EntityId> {
        self.nearest_of(pos, radius, |entity| me.hostile_to(entity.species))
    }

    /// Сколько союзных воинов рядом.
    pub fn count_soldiers_near(&self, pos: TilePos, radius: i32, nation: Option<NationId>) -> u32 {
        self.entities
            .iter()
            .filter(|entity| {
                entity.alive
                    && entity.species == Species::Soldier
                    && entity.nation == nation
                    && pos.chebyshev(entity.pos) <= radius
            })
            .count() as u32
    }

    /// Ближайший лес — источник дерева и еды.
    pub fn nearest_forest(&self, pos: TilePos, radius: i32) -> Option<TilePos> {
        self.world
            .find_terrain_near(pos, radius, |terrain| terrain.is_forest())
    }

    /// Ближайший костёр страны.
    pub fn nearest_nation_center(
        &self,
        pos: TilePos,
        radius: i32,
    ) -> Option<(NationId, TilePos)> {
        let mut best: Option<(i32, NationId, TilePos)> = None;
        for building in &self.buildings {
            if building.kind != BuildingKind::Campfire {
                continue;
            }
            let Some(nation) = building.nation else {
                continue;
            };
            let distance = pos.chebyshev(building.pos);
            if distance > radius {
                continue;
            }
            if best.map_or(true, |(best_distance, _, _)| distance < best_distance) {
                best = Some((distance, nation, building.pos));
            }
        }
        best.map(|(_, nation, pos)| (nation, pos))
    }

    /// Незавершённая стройка страны.
    pub fn nation_site(&self, nation: NationId) -> Option<BuildingId> {
        self.buildings
            .iter()
            .find(|building| building.nation == Some(nation) && building.is_site())
            .map(|building| building.id)
    }

    // ---------------------------------------------------------------- спавн

    /// Ставит существо (ищет ближайшую проходимую клетку).
    pub fn spawn(&mut self, species: Species, pos: TilePos) -> Option<EntityId> {
        let spot = if self.world.is_walkable(pos) {
            pos
        } else {
            self.world.nearest_walkable(pos, 6)?
        };

        Some(self.push_entity(species, spot))
    }

    /// Кладёт существо на конкретную клетку без поиска проходимого места.
    fn push_entity(&mut self, species: Species, pos: TilePos) -> EntityId {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        let variant = (self.rng.next_u32() % 4) as u8;
        self.entities.push(Entity::new(id, species, pos, variant));
        // Статистика всегда соответствует миру: интерфейс читает её как есть.
        self.refresh_stats();
        id
    }

    /// Ставит существо сразу в страну.
    pub fn spawn_in_nation(
        &mut self,
        species: Species,
        pos: TilePos,
        nation: NationId,
    ) -> Option<EntityId> {
        let id = self.spawn(species, pos)?;
        if let Some(entity) = self.entity_mut(id) {
            entity.nation = Some(nation);
        }
        if let Some(nation) = self.nation_mut(nation) {
            nation.add_citizen(species == Species::Soldier);
        }
        Some(id)
    }

    /// Ставит стаю/группу существ вокруг точки.
    pub fn spawn_pack(&mut self, species: Species, center: TilePos, count: u32) -> Vec<EntityId> {
        let mut ids = Vec::new();
        for _ in 0..count {
            let spot = self.rng.tile_in_square(center, 2);
            if let Some(id) = self.spawn(species, spot) {
                ids.push(id);
            }
        }
        ids
    }

    // ---------------------------------------------------------------- кисть

    /// Применяет инструмент игрока. Возвращает число изменённых тайлов
    /// (для кистей рельефа) или число появившихся существ.
    pub fn apply_tool(&mut self, config: ToolConfig, center: TilePos) -> u32 {
        if config.tool.is_terrain() {
            let changed = brush::paint(&mut self.world, config, center, &mut self.rng);
            if changed > 0 {
                self.after_terrain_change();
            }
            return changed;
        }

        match config.tool {
            Tool::Inspect => {
                self.selected = self.nearest_of(center, 2, |_| true);
                0
            }
            tool => {
                let Some(species) = tool.species() else {
                    return 0;
                };
                let mut spawned = 0;
                for _ in 0..tool.spawn_count() {
                    let spot = self.rng.tile_in_square(center, 2);
                    if self.spawn(species, spot).is_some() {
                        spawned += 1;
                    }
                }
                if spawned > 0 {
                    self.events.push(
                        self.tick,
                        EventKind::Nature,
                        format!("{}: {} шт.", species.name_ru(), spawned),
                    );
                    self.refresh_stats();
                }
                spawned
            }
        }
    }

    /// Всё, что нужно пересчитать после правки рельефа.
    fn after_terrain_change(&mut self) {
        self.blocked_dirty = true;
        self.territory_dirty = true;
        self.rescue_stranded_entities();
        self.remove_drowned_buildings();
        self.refresh_stats();
    }

    /// Существа, оказавшиеся в воде или в горах, выбираются на сушу.
    fn rescue_stranded_entities(&mut self) {
        for index in 0..self.entities.len() {
            if !self.entities[index].alive {
                continue;
            }
            let pos = self.entities[index].pos;
            let species = self.entities[index].species;
            if self.is_passable_for(species, pos) {
                continue;
            }
            let spot = if species.is_water_unit() {
                self.nearest_water(pos, 12)
            } else {
                self.world.nearest_walkable(pos, 8)
            };
            if let Some(spot) = spot {
                let entity = &mut self.entities[index];
                entity.pos = spot;
                entity.prev_pos = spot;
                entity.path.clear();
            } else {
                self.kill(index, None);
                self.events.push(
                    self.tick,
                    EventKind::Death,
                    format!("{} не пережил(а) потоп", species.name_ru()),
                );
            }
        }
    }

    /// Постройки, ушедшие под воду, исчезают.
    fn remove_drowned_buildings(&mut self) {
        let world = &self.world;
        let before = self.buildings.len();
        self.buildings.retain(|building| world.is_land(building.pos));
        if self.buildings.len() != before {
            self.blocked_dirty = true;
            self.territory_dirty = true;
        }
    }

    // ---------------------------------------------------------------- тик

    /// Один тик симуляции.
    pub fn step(&mut self) {
        if self.speed.is_paused() {
            return;
        }

        self.tick += 1;
        if self.blocked_dirty {
            self.rebuild_blocked();
        }

        ai::think_all(self);
        self.move_pass();
        combat::resolve(self);
        self.economy_pass();
        self.finish_tick();
    }

    /// Выполняет несколько тиков подряд (удобно для тестов и перемотки).
    pub fn run(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.step();
        }
    }

    fn finish_tick(&mut self) {
        self.entities.retain(|entity| entity.alive);
        if let Some(id) = self.selected {
            if self.entity(id).is_none() {
                self.selected = None;
            }
        }

        if self.tick % Self::BIRTH_INTERVAL == 0 {
            self.try_births();
        }

        if self.territory_dirty {
            self.rebuild_territory();
        }

        self.port_pass();
        self.refresh_nations();
        self.refresh_stats();
    }

    fn move_pass(&mut self) {
        for index in 0..self.entities.len() {
            if !self.entities[index].alive {
                continue;
            }
            let timer = self.entities[index].move_timer;
            if timer > 0 {
                self.entities[index].move_timer = timer - 1;
                continue;
            }
            self.move_entity(index);
        }
    }

    fn move_entity(&mut self, index: usize) {
        let (pos, intent, move_interval, attack_range, species) = {
            let entity = &self.entities[index];
            let stats = entity.stats();
            (
                entity.pos,
                entity.intent,
                stats.move_interval,
                stats.attack_range,
                entity.species,
            )
        };

        // Куда идём.
        let goal = match intent {
            Intent::Idle => None,
            Intent::Wander { goal } => Some(goal),
            Intent::Chop { tile } | Intent::Forage { tile } => Some(tile),
            Intent::Work { building } => self
                .buildings
                .iter()
                .find(|target| target.id == building)
                .map(|target| target.pos),
            Intent::Attack { target } => self
                .entities
                .iter()
                .find(|entity| entity.id == target && entity.alive)
                .map(|entity| entity.pos),
            Intent::AttackBuilding { building } => self
                .buildings
                .iter()
                .find(|target| target.id == building && !target.is_destroyed())
                .map(|target| target.pos),
            Intent::Flee { .. } => None,
        };

        if let Intent::Flee { from } = intent {
            if let Some(step) = self.flee_step(pos, from) {
                self.step_to(index, pos, step, move_interval);
            }
            return;
        }

        let Some(goal) = goal else {
            return;
        };

        let in_place = match intent {
            Intent::Work { .. } | Intent::Chop { .. } | Intent::Forage { .. } => {
                pos.chebyshev(goal) <= 1
            }
            Intent::Attack { .. } => distance_between(pos, goal) <= attack_range,
            Intent::AttackBuilding { .. } => pos.chebyshev(goal) <= 1,
            _ => pos == goal,
        };

        if in_place {
            if matches!(intent, Intent::Wander { .. }) {
                self.entities[index].assign(Intent::Idle);
            }
            self.entities[index].path.clear();
            return;
        }

        // Путь: берём закэшированный или считаем новый.
        let mut path = std::mem::take(&mut self.entities[index].path);
        while path.first().is_some_and(|first| *first == pos) {
            path.remove(0);
        }
        if path.is_empty() {
            path = self
                .find_path_for(species, pos, goal, Self::PATH_BUDGET)
                .unwrap_or_default();
        }

        let mut next = path.first().copied();
        if next.is_some_and(|candidate| !self.is_passable_for(species, candidate)) {
            path.clear();
            next = None;
        }
        let next = next.or_else(|| {
            greedy_step(pos, goal, |candidate| {
                self.is_passable_for(species, candidate)
            })
        });
        let next = next.filter(|candidate| self.is_passable_for(species, *candidate));

        if let Some(step) = next {
            if path.first().is_some_and(|first| *first == step) {
                path.remove(0);
            }
            self.entities[index].path = path;
            self.step_to(index, pos, step, move_interval);
        } else {
            // Застряли: сбрасываем путь и попробуем позже.
            self.entities[index].path.clear();
            self.entities[index].move_timer = move_interval;
        }
    }

    fn step_to(&mut self, index: usize, from: TilePos, to: TilePos, move_interval: u16) {
        let entity = &mut self.entities[index];
        entity.prev_pos = from;
        entity.pos = to;
        entity.facing_left = to.x < from.x;
        entity.move_timer = move_interval;
    }

    fn flee_step(&self, pos: TilePos, from: TilePos) -> Option<TilePos> {
        let mut best: Option<(i32, TilePos)> = None;
        for neighbor in pos.neighbors8() {
            if !self.is_walkable(neighbor) {
                continue;
            }
            let distance = neighbor.chebyshev(from);
            if best.map_or(true, |(best_distance, _)| distance > best_distance) {
                best = Some((distance, neighbor));
            }
        }
        best.map(|(_, pos)| pos)
    }

    fn economy_pass(&mut self) {
        for index in 0..self.entities.len() {
            if !self.entities[index].alive {
                continue;
            }
            self.age_entity(index);
            if !self.entities[index].alive {
                continue;
            }
            self.act_entity(index);
        }
    }

    fn age_entity(&mut self, index: usize) {
        let species = self.entities[index].species;
        let stats = species.stats();

        self.entities[index].age = self.entities[index].age.saturating_add(1);

        if stats.hunger_rate > 0
            && self.tick % stats.hunger_rate as u64 == 0
            && self.entities[index].hunger < Entity::MAX_HUNGER
        {
            self.entities[index].hunger += 1;
        }

        let starving = self.entities[index].hunger >= Entity::MAX_HUNGER;
        let old = self.entities[index].age > stats.max_age;

        if starving || old {
            self.entities[index].hp -= 1;
        }

        if !self.entities[index].is_alive() {
            let reason = if starving {
                "от голода"
            } else if old {
                "от старости"
            } else {
                "от ран"
            };
            self.kill(index, None);
            self.events.push(
                self.tick,
                EventKind::Death,
                format!("{} умер {reason}", species.name_ru()),
            );
        }
    }

    fn act_entity(&mut self, index: usize) {
        let intent = self.entities[index].intent;
        let pos = self.entities[index].pos;

        match intent {
            Intent::Wander { goal } => {
                if pos == goal {
                    self.entities[index].assign(Intent::Idle);
                }
            }
            Intent::Chop { tile } => {
                if pos.chebyshev(tile) <= 1 {
                    self.chop(index, tile);
                }
            }
            Intent::Forage { tile } => {
                if pos.chebyshev(tile) <= 1 {
                    self.forage(index);
                }
            }
            Intent::Work { building } => {
                if self
                    .building(building)
                    .is_some_and(|target| pos.chebyshev(target.pos) <= 1)
                {
                    self.work_on(index, building);
                }
            }
            _ => {}
        }
    }

    fn chop(&mut self, index: usize, tile: TilePos) {
        if self.world.terrain(tile).is_forest() {
            // Дерево срублено: лес становится лугом.
            self.world.set_moisture(tile, 40);
        }
        let wood = (self.entities[index].wood + 1).min(9);
        self.entities[index].wood = wood;
        self.entities[index].assign(Intent::Idle);
    }

    fn forage(&mut self, index: usize) {
        let entity = &mut self.entities[index];
        entity.hunger = 0;
        entity.hp = (entity.hp + 4).min(entity.max_hp());
        entity.assign(Intent::Idle);
    }

    fn work_on(&mut self, index: usize, building_id: BuildingId) {
        let Some(building_index) = self.building_index(building_id) else {
            self.entities[index].assign(Intent::Idle);
            return;
        };
        if self.buildings[building_index].is_complete() {
            self.entities[index].assign(Intent::Idle);
            return;
        }

        let completed = self.buildings[building_index].add_work(1, self.tick);

        if completed {
            let kind = self.buildings[building_index].kind;
            let nation = self.buildings[building_index].nation;
            self.blocked_dirty = true;
            self.territory_dirty = true;

            match kind {
                BuildingKind::House => {
                    self.totals.houses_built += 1;
                    if let Some(nation_id) = nation {
                        if let Some(nation) = self.nation_mut(nation_id) {
                            nation.houses += 1;
                        }
                        let name = self
                            .nation(nation_id)
                            .map(|nation| nation.name.clone())
                            .unwrap_or_default();
                        self.events.push(
                            self.tick,
                            EventKind::Build,
                            format!("{name}: построен новый дом"),
                        );
                    }
                }
                BuildingKind::Campfire => {
                    self.territory_dirty = true;
                }
                BuildingKind::Port => {
                    self.territory_dirty = true;
                    let site = self
                        .buildings
                        .iter()
                        .find(|building| building.id == building_id)
                        .map(|building| building.pos)
                        .unwrap_or(self.entities[index].pos);
                    if let Some(nation_id) = nation {
                        let name = self.nation_name(Some(nation_id)).to_string();
                        self.spawn_boat(site, nation_id);
                        self.events.push(
                            self.tick,
                            EventKind::Build,
                            format!("{name}: порт готов, лодка спущена на воду"),
                        );
                    }
                }
                BuildingKind::Ruin => {}
            }
            self.entities[index].assign(Intent::Idle);
        }
    }

    /// Основывает страну в точке, где стоит существо.
    pub fn found_nation(&mut self, index: usize) {
        if !self.entities[index].alive || !self.entities[index].species.can_found_nation() {
            return;
        }
        let pos = self.entities[index].pos;
        if !self.world.is_land(pos) {
            return;
        }

        let nation_id = self.next_nation_id;
        self.next_nation_id += 1;

        let taken: Vec<String> = self.nations.iter().map(|nation| nation.name.clone()).collect();
        let name = names::nation_name(&mut self.rng, &taken);
        let color = NATION_PALETTE[(nation_id as usize - 1) % NATION_PALETTE.len()];
        let nation = Nation::new(nation_id, name.clone(), color, pos, self.tick);
        self.nations.push(nation);

        let building_id = self.next_building_id;
        self.next_building_id += 1;
        self.buildings.push(Building::finished(
            building_id,
            BuildingKind::Campfire,
            pos,
            Some(nation_id),
            self.tick,
        ));

        let entity = &mut self.entities[index];
        entity.nation = Some(nation_id);
        entity.wood = entity.wood.saturating_sub(2);
        entity.assign(Intent::Idle);

        if let Some(nation) = self.nation_mut(nation_id) {
            nation.add_citizen(false);
        }

        self.totals.nations_founded += 1;
        self.blocked_dirty = true;
        self.territory_dirty = true;
        // Новая страна входит в мир со всеми и сразу попадает в таблицу.
        self.diplomacy.ensure(self.nations.len());
        self.refresh_stats();
        self.events.push(
            self.tick,
            EventKind::Nation,
            format!("{name}: основана новая страна"),
        );
    }

    /// Закладывает фундамент дома. Возвращает id постройки.
    pub fn start_house_site(&mut self, nation: NationId, near: TilePos) -> Option<BuildingId> {
        let site = self.find_house_site(nation, near)?;
        let id = self.next_building_id;
        self.next_building_id += 1;
        self.buildings
            .push(Building::new(id, BuildingKind::House, site, Some(nation)));
        self.blocked_dirty = true;
        self.territory_dirty = true;
        Some(id)
    }

    /// Ищет подходящее место под дом.
    pub fn find_house_site(&mut self, nation: NationId, near: TilePos) -> Option<TilePos> {
        let capital = self.nation(nation).map_or(near, |nation| nation.capital);
        let house_positions: Vec<TilePos> = self
            .buildings
            .iter()
            .filter(|building| building.nation == Some(nation))
            .map(|building| building.pos)
            .collect();

        for _ in 0..48 {
            let candidate = self.rng.tile_in_square(capital, 11);
            if !self.world.terrain(candidate).is_buildable() {
                continue;
            }
            if self.building_at(candidate).is_some() {
                continue;
            }
            if house_positions
                .iter()
                .any(|pos| pos.chebyshev(candidate) < 2)
            {
                continue;
            }
            if self.nearest_enemy(candidate, 4, Species::Human).is_some() {
                continue;
            }
            return Some(candidate);
        }
        None
    }

    fn try_births(&mut self) {
        let nations = self.nations.clone();
        for nation in nations {
            if nation.houses == 0 || nation.population >= nation.capacity() {
                continue;
            }
            if !self.rng.chance(0.7) {
                continue;
            }

            let houses: Vec<TilePos> = self
                .buildings
                .iter()
                .filter(|building| {
                    building.nation == Some(nation.id)
                        && building.kind == BuildingKind::House
                        && building.is_complete()
                })
                .map(|building| building.pos)
                .collect();

            let Some(&house) = self.rng.choose(&houses) else {
                continue;
            };
            let species = if nation.population >= 4 && self.rng.chance(0.2) {
                Species::Soldier
            } else {
                Species::Human
            };
            let Some(spot) = self.world.nearest_walkable(house, 3) else {
                continue;
            };
            if self.spawn_in_nation(species, spot, nation.id).is_some() {
                self.totals.births += 1;
                if species == Species::Soldier {
                    self.events.push(
                        self.tick,
                        EventKind::Birth,
                        format!("{}: вырастили воина", nation.name),
                    );
                }
            }
        }
    }

    /// Война требует флота: страны, у которых есть дома и враг, закладывают
    /// порт. Готовый порт сразу спускает на воду одну лодку (см. `work_on`).
    fn port_pass(&mut self) {
        if self.nations.is_empty() || self.tick % 40 != 0 {
            return;
        }
        let nations: Vec<NationId> = self.nations.iter().map(|nation| nation.id).collect();
        for nation in nations {
            let houses = self.nation(nation).map_or(0, |nation| nation.houses);
            if houses < 2 || !self.nation_at_war_with_anyone(nation) {
                continue;
            }
            let has_port = self
                .buildings
                .iter()
                .any(|building| building.nation == Some(nation) && building.kind == BuildingKind::Port);
            if has_port {
                continue;
            }
            if !self.rng.chance(0.25) {
                continue;
            }
            let Some(site) = self.find_port_site(nation) else {
                continue;
            };
            if self.building_at(site).is_some() {
                continue;
            }
            let id = self.next_building_id;
            self.next_building_id += 1;
            self.buildings
                .push(Building::new(id, BuildingKind::Port, site, Some(nation)));
            self.blocked_dirty = true;
            self.territory_dirty = true;
            let name = self.nation_name(Some(nation)).to_string();
            self.events.push(
                self.tick,
                EventKind::Build,
                format!("{name}: заложен порт для переправы к врагу"),
            );
        }
    }

    /// Наносит урон постройке. Возвращает `true`, если она только что рухнула.
    pub fn damage_building(&mut self, index: usize, amount: i32) -> bool {
        if index >= self.buildings.len() {
            return false;
        }
        if !self.buildings[index].damage(amount) {
            return false;
        }

        let kind = self.buildings[index].kind;
        let nation = self.buildings[index].nation;
        self.buildings[index].collapse();
        self.blocked_dirty = true;
        self.territory_dirty = true;

        let owner = self.nation_name(nation).to_string();
        let what = kind.name_ru().to_lowercase();
        let text = if nation.is_some() {
            format!("{owner}: {what} разрушен в бою")
        } else {
            format!("{what} разрушен в бою")
        };
        self.events.push(self.tick, EventKind::Combat, text);
        true
    }

    fn rebuild_blocked(&mut self) {
        let cells = self.world.len();
        if self.blocked.len() != cells {
            self.blocked = vec![false; cells];
        }
        self.blocked.iter_mut().for_each(|blocked| *blocked = false);
        for building in &self.buildings {
            if building.kind.blocks_movement() && building.is_complete() {
                if let Some(index) = self.world.index(building.pos) {
                    self.blocked[index] = true;
                }
            }
        }
        self.blocked_dirty = false;
    }

    fn rebuild_territory(&mut self) {
        let cells = self.world.len();
        if self.territory.len() != cells {
            self.territory = vec![0; cells];
        }
        self.territory.iter_mut().for_each(|owner| *owner = 0);

        let claims: Vec<(TilePos, i32, u8)> = self
            .buildings
            .iter()
            .filter(|building| building.kind.is_permanent() && building.is_complete())
            .filter_map(|building| {
                let nation = building.nation?;
                if nation > u8::MAX as NationId {
                    return None;
                }
                Some((
                    building.pos,
                    building.kind.territory_radius(),
                    nation as u8,
                ))
            })
            .collect();

        let world = &self.world;
        let territory = &mut self.territory;
        for (pos, radius, owner) in claims {
            for_each_in_radius(pos, radius, |tile, distance| {
                if distance > 1.0 || !world.is_land(tile) {
                    return;
                }
                if let Some(index) = world.index(tile) {
                    if territory[index] == 0 {
                        territory[index] = owner;
                    }
                }
            });
        }

        self.territory_dirty = false;
    }

    fn refresh_nations(&mut self) {
        let entities = &self.entities;
        let buildings = &self.buildings;
        for nation in &mut self.nations {
            let mut population = 0;
            let mut soldiers = 0;
            let mut houses = 0;

            for entity in entities.iter() {
                if entity.alive && entity.nation == Some(nation.id) && entity.species.is_humanoid() {
                    population += 1;
                    if entity.species == Species::Soldier {
                        soldiers += 1;
                    }
                }
            }
            for building in buildings.iter() {
                if building.nation == Some(nation.id)
                    && building.kind == BuildingKind::House
                    && building.is_complete()
                {
                    houses += 1;
                }
            }

            nation.population = population;
            nation.soldiers = soldiers;
            nation.houses = houses;
            nation.peak_population = nation.peak_population.max(population);
        }
    }

    fn refresh_stats(&mut self) {
        let mut stats = Stats::default();
        for entity in &self.entities {
            if !entity.alive {
                continue;
            }
            match entity.species {
                Species::Human => stats.humans += 1,
                Species::Soldier => stats.soldiers += 1,
                Species::Deer => stats.deer += 1,
                Species::Wolf => stats.wolves += 1,
                Species::Zombie => stats.zombies += 1,
                Species::Boat => stats.boats += 1,
            }
        }
        for building in &self.buildings {
            match building.kind {
                BuildingKind::House if building.is_complete() => stats.houses += 1,
                BuildingKind::Campfire => stats.campfires += 1,
                BuildingKind::Ruin => stats.ruins += 1,
                BuildingKind::Port if building.is_complete() => stats.ports += 1,
                _ => {}
            }
        }

        stats.population = stats.humans + stats.soldiers;
        stats.monsters = stats.wolves + stats.zombies;
        stats.nations = self.nations.len() as u32;
        stats.wars = self.diplomacy.war_count();
        stats.alliances = self.diplomacy.alliance_count();
        stats.land_fraction = self.world.land_fraction();
        stats.land_tiles = self.world.land_tiles();
        stats.tick = self.tick;
        self.stats = stats;
    }

    /// Убивает существо: помечает мёртвым, обновляет счётчики и, если убийца
    /// был зомби, иногда поднимает нового зомби.
    pub fn kill(&mut self, index: usize, killer: Option<Species>) {
        if index >= self.entities.len() || !self.entities[index].alive {
            return;
        }

        let species = self.entities[index].species;
        let pos = self.entities[index].pos;
        let nation = self.entities[index].nation;

        self.entities[index].alive = false;
        self.entities[index].hp = 0;
        self.totals.deaths += 1;

        if let Some(nation_id) = nation {
            if let Some(nation) = self.nation_mut(nation_id) {
                // Лодка — имущество, а не житель: население не меняется.
                if species.is_humanoid() {
                    nation.remove_citizen(species == Species::Soldier);
                    nation.losses += 1;
                }
            }
        }

        if killer == Some(Species::Zombie) && species.is_humanoid() && self.rng.chance(0.6) {
            if self.spawn(Species::Zombie, pos).is_some() {
                self.events.push(
                    self.tick,
                    EventKind::Combat,
                    "Зомби поднял нового ходячего мертвеца".to_string(),
                );
            }
        }
    }

    /// Записывает смерть в ленту событий.
    pub fn log_death(&mut self, victim: Species, killer: Species) {
        let text = if killer.is_monster() {
            format!(
                "{} погиб: {} оказался сильнее",
                victim.name_ru(),
                killer.name_ru()
            )
        } else {
            format!("{} погиб в схватке", victim.name_ru())
        };
        self.events.push(self.tick, EventKind::Death, text);
    }

    /// Переключает скорость по кругу.
    pub fn cycle_speed(&mut self) {
        self.speed = self.speed.next();
    }

    /// Ставит паузу или продолжает.
    pub fn toggle_pause(&mut self) {
        self.speed = if self.speed.is_paused() {
            Speed::Normal
        } else {
            Speed::Paused
        };
    }

    /// Сколько тиков нужно выполнить за `dt` секунд реального времени.
    pub fn ticks_for(&self, dt: f32) -> u32 {
        let ticks = dt * SIM_HZ as f32 * self.speed.multiplier();
        ticks.clamp(0.0, 12.0) as u32
    }

    /// Время мира в тиках, прошедшее с основания первой страны.
    pub fn age_ticks(&self) -> u64 {
        self.tick
    }
}

fn distance_between(a: TilePos, b: TilePos) -> f32 {
    a.distance(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::pos;

    /// Остров из двух мазков суши и леса — база для сценарных тестов.
    fn island(seed: u64) -> Simulation {
        let mut sim = Simulation::new(WorldSize::SMALL, seed);
        sim.apply_tool(ToolConfig::new(Tool::Raise, 12), pos(60, 60));
        sim.apply_tool(ToolConfig::new(Tool::Raise, 10), pos(70, 66));
        sim.apply_tool(ToolConfig::new(Tool::Forest, 6), pos(64, 62));
        sim
    }

    #[test]
    fn new_world_is_empty_ocean() {
        let sim = Simulation::new(WorldSize::SMALL, 1);
        assert_eq!(sim.world.land_fraction(), 0.0);
        assert_eq!(sim.stats.land_tiles, 0);
        assert!(sim.entities.is_empty());
        assert!(sim.nations.is_empty());
        assert!(sim.buildings.is_empty());
        assert_eq!(sim.tick, 0);
        assert_eq!(sim.speed, Speed::Normal);
        assert!(sim.events.last().is_some(), "стартовое сообщение есть");
    }

    #[test]
    fn painting_land_updates_statistics() {
        let sim = island(7);
        assert!(sim.stats.land_tiles > 500, "суши: {}", sim.stats.land_tiles);
        assert!(sim.stats.land_fraction > 0.0);
        assert!(sim.stats.land_fraction <= 1.0);
    }

    #[test]
    fn spawn_is_impossible_in_open_water() {
        let mut sim = Simulation::new(WorldSize::SMALL, 3);
        assert!(sim.spawn(Species::Human, pos(10, 10)).is_none());
        assert!(sim.entities.is_empty());
    }

    #[test]
    fn spawning_on_land_is_counted() {
        let mut sim = island(11);
        let id = sim.spawn(Species::Human, pos(60, 60)).expect("человек");
        assert!(sim.entity(id).is_some());
        assert_eq!(sim.stats.humans, 1);
        assert_eq!(sim.stats.population, 1);
    }

    #[test]
    fn founding_a_nation_builds_a_capital() {
        let mut sim = island(5);
        let id = sim.spawn(Species::Human, pos(60, 60)).expect("человек");
        let index = sim.entity_index(id).expect("индекс");
        sim.found_nation(index);

        assert_eq!(sim.nations.len(), 1);
        assert_eq!(sim.totals.nations_founded, 1);
        let nation = sim.nations[0].clone();
        assert_eq!(nation.capital, pos(60, 60));
        assert_eq!(sim.entity(id).and_then(|entity| entity.nation), Some(nation.id));
        assert!(
            sim.buildings
                .iter()
                .any(|building| building.kind == BuildingKind::Campfire),
            "столица-костёр должна появиться"
        );
        assert_eq!(sim.stats.nations, 1);
    }

    #[test]
    fn capital_claims_territory_after_a_tick() {
        let mut sim = island(9);
        let id = sim.spawn(Species::Human, pos(60, 60)).expect("человек");
        let index = sim.entity_index(id).expect("индекс");
        sim.found_nation(index);
        let nation = sim.nations[0].id;

        sim.step();
        assert_eq!(sim.territory_owner(pos(60, 60)), Some(nation));
        assert_eq!(sim.territory_owner(pos(62, 60)), Some(nation));
        assert_eq!(sim.territory_owner(pos(5, 5)), None);
    }

    #[test]
    fn finished_house_blocks_its_tile() {
        let mut sim = island(13);
        let house = pos(62, 60);
        sim.buildings
            .push(Building::finished(99, BuildingKind::House, house, None, 0));

        // Любая правка рельефа помечает карту занятости грязной.
        sim.apply_tool(ToolConfig::new(Tool::Raise, 1), pos(60, 60));
        sim.step();

        assert!(sim.is_blocked_tile(house));
        assert!(!sim.is_walkable(house));
    }

    #[test]
    fn sinking_land_removes_buildings() {
        let mut sim = island(17);
        sim.buildings
            .push(Building::finished(50, BuildingKind::Campfire, pos(61, 61), None, 0));
        assert_eq!(sim.buildings.len(), 1);

        sim.apply_tool(ToolConfig::new(Tool::Lower, 14), pos(61, 61));
        assert!(sim.buildings.is_empty(), "утопленная постройка исчезает");
    }

    #[test]
    fn brush_can_spawn_monsters() {
        let mut sim = island(29);
        let spawned = sim.apply_tool(ToolConfig::new(Tool::Wolf, 1), pos(70, 66));
        assert!(spawned >= 1);
        assert!(sim.stats.monsters >= 1);
        assert_eq!(sim.stats.zombies, 0);
    }

    #[test]
    fn pause_stops_the_clock() {
        let mut sim = island(21);
        sim.speed = Speed::Paused;
        sim.run(50);
        assert_eq!(sim.tick, 0, "на паузе тики не идут");

        sim.toggle_pause();
        assert_eq!(sim.speed, Speed::Normal);
        sim.run(5);
        assert_eq!(sim.tick, 5);
    }

    #[test]
    fn speed_cycles_through_all_steps() {
        let mut sim = Simulation::new(WorldSize::SMALL, 2);
        assert_eq!(sim.speed, Speed::Normal);
        sim.cycle_speed();
        assert_eq!(sim.speed, Speed::Fast);
        sim.cycle_speed();
        assert_eq!(sim.speed, Speed::Turbo);
        sim.cycle_speed();
        assert_eq!(sim.speed, Speed::Paused);
        sim.cycle_speed();
        assert_eq!(sim.speed, Speed::Normal);
    }

    #[test]
    fn selection_refers_to_a_living_entity() {
        let mut sim = island(23);
        let id = sim.spawn(Species::Human, pos(60, 60)).expect("человек");
        sim.select(Some(id));
        assert_eq!(sim.selected_entity().map(|entity| entity.id), Some(id));

        sim.select(None);
        assert!(sim.selected_entity().is_none());
    }

    #[test]
    fn walking_targets_avoid_water() {
        let mut sim = island(31);
        let id = sim.spawn(Species::Human, pos(60, 60)).expect("человек");
        let index = sim.entity_index(id).expect("индекс");
        assert!(sim.is_walkable(sim.entities[index].pos));

        for _ in 0..40 {
            sim.step();
            let position = sim.entity(id).map(|entity| entity.pos);
            if let Some(position) = position {
                assert!(sim.world.terrain(position).is_walkable());
            }
        }
    }
}
