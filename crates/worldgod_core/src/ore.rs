//! Руды и способности, которые они дают жителям.
//!
//! Залежь — это слой тайла (`Tile::ore` + `Tile::ore_amount`), такой же
//! погодный/ресурсный слой, как снег: рельеф от руды не меняется, но жители
//! страны, которая изучила «Добычу», выкапывают руду, несут очки в
//! исследования и иногда получают **способность** — бессмертие, режим бога,
//! регенерацию, огненный удар или стойкость.
//!
//! Камень и уголь — обычные ресурсы (только наука), а алмаз, рубин и изумруд —
//! драгоценные: они и дают сверхспособности.

use crate::grid::{for_each_in_radius, TilePos};
use crate::rng::Rng;
use crate::world::World;

/// Сколько руды кладёт кисть в центре мазка.
pub const ORE_PER_TILE: u8 = 6;

/// Порода руды.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OreKind {
    /// Камень — самый обычный ресурс.
    Stone = 1,
    /// Уголь — топливо для мастерских.
    Coal = 2,
    /// Алмаз — даёт бессмертие.
    Diamond = 3,
    /// Рубин — даёт режим бога.
    Ruby = 4,
    /// Изумруд — даёт регенерацию (и стойкость).
    Emerald = 5,
}

impl OreKind {
    /// Все породы в порядке панели.
    pub const ALL: [OreKind; 5] = [
        Self::Stone,
        Self::Coal,
        Self::Diamond,
        Self::Ruby,
        Self::Emerald,
    ];

    /// Код для хранения в тайле (0 — руды нет).
    pub fn code(self) -> u8 {
        self as u8
    }

    /// Восстановление породы из кода тайла.
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.iter().copied().find(|kind| kind.code() == code)
    }

    /// Название для интерфейса.
    pub fn name_ru(self) -> &'static str {
        match self {
            Self::Stone => "Камень",
            Self::Coal => "Уголь",
            Self::Diamond => "Алмаз",
            Self::Ruby => "Рубин",
            Self::Emerald => "Изумруд",
        }
    }

    /// Название в родительном падеже («житель добыл алмаз»).
    pub fn name_accusative_ru(self) -> &'static str {
        match self {
            Self::Stone => "камень",
            Self::Coal => "уголь",
            Self::Diamond => "алмаз",
            Self::Ruby => "рубин",
            Self::Emerald => "изумруд",
        }
    }

    /// Подсказка под кнопкой кисти.
    pub fn hint_ru(self) -> &'static str {
        match self {
            Self::Stone => "Камень: обычный ресурс для науки",
            Self::Coal => "Уголь: топливо, ускоряет разработки",
            Self::Diamond => "Алмаз: даёт жителю бессмертие",
            Self::Ruby => "Рубин: даёт режим бога",
            Self::Emerald => "Изумруд: регенерация и стойкость",
        }
    }

    /// Цвет кристалла в рендере и на иконке.
    pub fn color_rgb(self) -> [u8; 3] {
        match self {
            Self::Stone => [156, 158, 164],
            Self::Coal => [56, 58, 64],
            Self::Diamond => [186, 236, 250],
            Self::Ruby => [226, 66, 84],
            Self::Emerald => [74, 206, 138],
        }
    }

    /// Способность, которую даёт порода.
    pub fn ability(self) -> Ability {
        match self {
            Self::Stone => Ability::Toughness,
            Self::Coal => Ability::Fire,
            Self::Diamond => Ability::Immortality,
            Self::Ruby => Ability::GodMode,
            Self::Emerald => Ability::Regeneration,
        }
    }

    /// Драгоценная ли порода (даёт сверхспособность, а не только науку).
    pub fn is_gem(self) -> bool {
        matches!(self, Self::Diamond | Self::Ruby | Self::Emerald)
    }

    /// Сколько очков исследований приносит единица руды.
    pub fn research_points(self) -> u32 {
        match self {
            Self::Stone => 2,
            Self::Coal => 3,
            Self::Diamond => 9,
            Self::Ruby => 8,
            Self::Emerald => 8,
        }
    }

    /// Шанс, что житель, добывший руду, получит способность.
    pub fn ability_chance(self) -> f32 {
        match self {
            Self::Stone | Self::Coal => 0.0,
            Self::Diamond => 0.75,
            Self::Ruby => 0.75,
            Self::Emerald => 0.6,
        }
    }

    /// Описание для строки осмотра.
    pub fn inspect_ru(self) -> String {
        format!(
            "{} — способность «{}»",
            self.name_ru(),
            self.ability().name_ru()
        )
    }
}

/// Способность жителя, полученная из руды.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ability {
    /// Стойкость: урон по жителю уменьшается вдвое.
    Toughness = 0,
    /// Огненный удар: бьёт в полтора раза сильнее.
    Fire = 1,
    /// Бессмертие: здоровье не опускается ниже единицы.
    Immortality = 2,
    /// Режим бога: не получает урона, бьёт втрое сильнее.
    GodMode = 3,
    /// Регенерация: залечивает раны каждый тик.
    Regeneration = 4,
}

impl Ability {
    /// Все способности.
    pub const ALL: [Ability; 5] = [
        Self::Toughness,
        Self::Fire,
        Self::Immortality,
        Self::GodMode,
        Self::Regeneration,
    ];

    /// Бит в маске способностей существа.
    pub fn bit(self) -> u8 {
        1 << (self as u8)
    }

    /// Название для интерфейса.
    pub fn name_ru(self) -> &'static str {
        match self {
            Self::Toughness => "Стойкость",
            Self::Fire => "Огненный удар",
            Self::Immortality => "Бессмертие",
            Self::GodMode => "Режим бога",
            Self::Regeneration => "Регенерация",
        }
    }

    /// Что способность даёт.
    pub fn hint_ru(self) -> &'static str {
        match self {
            Self::Toughness => "получает вдвое меньше урона",
            Self::Fire => "бьёт в полтора раза сильнее",
            Self::Immortality => "не может умереть",
            Self::GodMode => "неуязвим и бьёт втрое сильнее",
            Self::Regeneration => "залечивает раны сам",
        }
    }
}

/// Текстовая расшифровка маски способностей: «Бессмертие, Режим бога».
pub fn abilities_ru(mask: u8) -> String {
    let names: Vec<&str> = Ability::ALL
        .iter()
        .filter(|ability| mask & ability.bit() != 0)
        .map(|ability| ability.name_ru())
        .collect();
    if names.is_empty() {
        "нет".to_string()
    } else {
        names.join(", ")
    }
}

/// Кладёт залежь руды на сушу в круге.
///
/// Вода и горные пики руду не принимают: добывать её живому существу негде.
/// Возвращает число тайлов, где руда появилась.
pub fn paint(
    world: &mut World,
    kind: OreKind,
    center: TilePos,
    radius: i32,
    rng: &mut Rng,
) -> u32 {
    let radius = radius.clamp(0, 64);
    let mut targets: Vec<(TilePos, u8)> = Vec::new();

    for_each_in_radius(center, radius, |p, distance| {
        let strength = ((1.0 - distance) * (0.7 + rng.next_f32() * 0.6)).clamp(0.0, 1.0);
        if strength < 0.25 {
            return;
        }
        let Some(tile) = world.tile(p) else {
            return;
        };
        let terrain = tile.terrain();
        if !terrain.is_land() || !terrain.is_walkable() {
            return;
        }
        let amount = (strength * ORE_PER_TILE as f32).round().max(1.0) as u8;
        targets.push((p, amount));
    });

    let mut changed = 0;
    for (p, amount) in targets {
        if world.set_ore(p, kind, amount) {
            changed += 1;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::SEA_LEVEL;

    #[test]
    fn codes_roundtrip_and_zero_is_empty() {
        assert_eq!(OreKind::from_code(0), None);
        for kind in OreKind::ALL {
            assert_eq!(OreKind::from_code(kind.code()), Some(kind));
            assert!(!kind.name_ru().is_empty());
            assert!(!kind.hint_ru().is_empty());
            assert_ne!(kind.color_rgb(), [0, 0, 0]);
        }
    }

    #[test]
    fn gems_give_abilities_plain_ores_only_science() {
        assert_eq!(OreKind::Diamond.ability(), Ability::Immortality);
        assert_eq!(OreKind::Ruby.ability(), Ability::GodMode);
        assert_eq!(OreKind::Emerald.ability(), Ability::Regeneration);
        assert!(OreKind::Diamond.is_gem());
        assert!(!OreKind::Coal.is_gem());
        assert_eq!(OreKind::Stone.ability_chance(), 0.0);
        assert!(OreKind::Diamond.ability_chance() > 0.5);
        assert!(OreKind::Diamond.research_points() > OreKind::Stone.research_points());
        assert!(OreKind::Diamond.inspect_ru().contains("Бессмертие"));
    }

    #[test]
    fn ability_bits_are_unique_and_labeled() {
        let mut mask = 0;
        for ability in Ability::ALL {
            assert_eq!(mask & ability.bit(), 0, "бит {} занят", ability.name_ru());
            mask |= ability.bit();
            assert!(!ability.hint_ru().is_empty());
        }
        let text = abilities_ru(Ability::Immortality.bit() | Ability::GodMode.bit());
        assert!(text.contains("Бессмертие") && text.contains("Режим бога"));
        assert_eq!(abilities_ru(0), "нет");
    }

    fn island() -> World {
        let mut world = World::ocean(40, 40, 11);
        for y in 16..24 {
            for x in 16..24 {
                let p = TilePos::new(x, y);
                world.set_elevation(p, SEA_LEVEL + 120);
                world.set_moisture(p, 40);
            }
        }
        world
    }

    #[test]
    fn brush_places_ore_on_land_only() {
        let mut world = island();
        let mut rng = Rng::new(3);
        let changed = paint(&mut world, OreKind::Diamond, TilePos::new(20, 20), 3, &mut rng);
        assert!(changed > 0);
        assert_eq!(world.ore(TilePos::new(20, 20)), Some(OreKind::Diamond));
        assert!(world.ore_amount(TilePos::new(20, 20)) > 0);

        // В океан руда не кладётся: там добывать некому.
        let water = TilePos::new(2, 2);
        assert_eq!(paint(&mut world, OreKind::Coal, water, 2, &mut rng), 0);
        assert_eq!(world.ore(water), None);
        assert!(world.ore_tiles() > 0);
    }

    #[test]
    fn flooded_deposit_disappears() {
        let mut world = island();
        let mut rng = Rng::new(7);
        let p = TilePos::new(20, 20);
        paint(&mut world, OreKind::Emerald, p, 1, &mut rng);
        assert_eq!(world.ore(p), Some(OreKind::Emerald));

        world.set_elevation(p, SEA_LEVEL - 20);
        assert!(world.is_water(p));
        assert_eq!(world.ore(p), None, "затопленная залежь исчезает");
        assert_eq!(world.ore_amount(p), 0);
    }

    #[test]
    fn ore_is_mined_out_gradually() {
        let mut world = island();
        let mut rng = Rng::new(5);
        paint(&mut world, OreKind::Ruby, TilePos::new(20, 20), 1, &mut rng);
        let p = TilePos::new(20, 20);
        assert_eq!(world.ore(p), Some(OreKind::Ruby));

        let mut mined = 0;
        while let Some(kind) = world.take_ore(p, 1) {
            assert_eq!(kind, OreKind::Ruby);
            mined += 1;
            assert!(mined < 20, "залежь должна кончиться");
        }
        assert!(mined > 0);
        assert_eq!(world.ore(p), None);
    }
}
