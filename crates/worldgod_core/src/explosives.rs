//! Взрывчатка: динамит и ядерная бомба.
//!
//! Как и стихии, взрыв сначала меняет тайлы (воронка, выжженная земля,
//! осыпавшаяся руда), а урон по постройкам и существам считает
//! [`crate::Simulation::detonate`]. Модуль знает только про [`crate::World`],
//! поэтому воронку можно проверить тестом без симуляции.

use crate::grid::{for_each_in_radius, TilePos};
use crate::rng::Rng;
use crate::terrain::MAX_ELEVATION;
use crate::world::World;

/// Вид взрывчатки.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Explosive {
    /// Динамит: локальный взрыв, ломает дом и калечит всех рядом.
    Dynamite = 0,
    /// Ядерка: выжигает всё в большом радиусе и оставляет котлован с водой.
    Nuke = 1,
}

impl Explosive {
    /// Все виды в порядке панели.
    pub const ALL: [Explosive; 2] = [Self::Dynamite, Self::Nuke];

    /// Подпись на кнопке.
    pub fn label_ru(self) -> &'static str {
        match self {
            Self::Dynamite => "Динамит",
            Self::Nuke => "Ядерка",
        }
    }

    /// Полное название для ленты событий.
    pub fn name_ru(self) -> &'static str {
        match self {
            Self::Dynamite => "Взрыв динамита",
            Self::Nuke => "Ядерный взрыв",
        }
    }

    /// Подсказка под кнопкой.
    pub fn hint_ru(self) -> &'static str {
        match self {
            Self::Dynamite => "Рвёт дом и всё живое в радиусе 4",
            Self::Nuke => "Выжигает всё в радиусе 13 и оставляет кратер",
        }
    }

    /// Цвет иконки и подсветки.
    pub fn accent_rgb(self) -> [u8; 3] {
        match self {
            Self::Dynamite => [236, 142, 72],
            Self::Nuke => [246, 224, 120],
        }
    }

    /// Радиус взрыва в тайлах.
    pub fn radius(self) -> i32 {
        match self {
            Self::Dynamite => 4,
            Self::Nuke => 13,
        }
    }

    /// Урон существам в центре взрыва.
    pub fn entity_damage(self) -> i32 {
        match self {
            Self::Dynamite => 70,
            Self::Nuke => 400,
        }
    }

    /// Урон постройкам в центре взрыва.
    pub fn building_damage(self) -> i32 {
        match self {
            Self::Dynamite => 160,
            Self::Nuke => 900,
        }
    }

    /// Насколько глубоко проседает земля в центре.
    pub fn crater_depth(self) -> u16 {
        match self {
            Self::Dynamite => 55,
            Self::Nuke => 200,
        }
    }

    /// Маска разработки, без которой страна такое не применяет.
    pub fn required_tech(self) -> crate::tech::Tech {
        match self {
            Self::Dynamite => crate::tech::Tech::Chemistry,
            Self::Nuke => crate::tech::Tech::Atomic,
        }
    }
}

/// Что натворил взрыв.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExplosionReport {
    /// Сколько тайлов изменилось.
    pub tiles: u32,
    /// Сколько тайлов превратилось в воронку.
    pub craters: u32,
    /// Сколько руды осыпалось.
    pub ore_lost: u32,
    /// Сколько построек получило урон.
    pub buildings_hit: u32,
    /// Сколько построек разрушено.
    pub buildings_ruined: u32,
    /// Сколько существ задело.
    pub entities_hurt: u32,
    /// Сколько существ погибло.
    pub entities_killed: u32,
}

impl ExplosionReport {
    /// Строка для ленты событий.
    pub fn summary_ru(&self, kind: Explosive) -> String {
        let mut parts = Vec::new();
        if self.craters > 0 {
            parts.push(format!("воронка: {} тайлов", self.craters));
        }
        if self.buildings_hit > 0 {
            parts.push(format!(
                "построек задето: {}{}",
                self.buildings_hit,
                if self.buildings_ruined > 0 {
                    format!(", разрушено: {}", self.buildings_ruined)
                } else {
                    String::new()
                }
            ));
        }
        if self.entities_hurt > 0 {
            parts.push(format!(
                "существ задето: {}{}",
                self.entities_hurt,
                if self.entities_killed > 0 {
                    format!(", погибло: {}", self.entities_killed)
                } else {
                    String::new()
                }
            ));
        }
        if parts.is_empty() {
            return format!("{}: лишь грохот", kind.name_ru());
        }
        format!("{}: {}", kind.name_ru(), parts.join(" · "))
    }
}

/// Выбивает воронку: земля проседает, руда осыпается, снег испаряется.
///
/// Вода в воронке появляется сама: рельеф опускается ниже уровня моря,
/// а тип тайла выводится из высоты.
pub fn crater(
    world: &mut World,
    center: TilePos,
    radius: i32,
    depth: u16,
    rng: &mut Rng,
) -> (u32, u32) {
    let radius = radius.clamp(0, 64);
    let mut targets: Vec<(TilePos, u16, bool)> = Vec::new();

    for_each_in_radius(center, radius, |p, distance| {
        let strength = ((1.0 - distance) * (0.8 + rng.next_f32() * 0.4)).clamp(0.0, 1.0);
        if strength < 0.18 {
            return;
        }
        let Some(tile) = world.tile(p) else {
            return;
        };
        let drop = (depth as f32 * strength).round() as u16;
        if drop == 0 && tile.ore == 0 {
            return;
        }
        targets.push((p, drop, tile.ore != 0));
    });

    let mut craters = 0;
    let mut ore_lost = 0;
    for (p, drop, had_ore) in targets {
        let elevation = world.elevation(p);
        if drop > 0 {
            let target = elevation.saturating_sub(drop).min(MAX_ELEVATION);
            if world.set_elevation(p, target) {
                craters += 1;
            }
        }
        world.set_moisture(p, 0);
        world.set_snow(p, 0);
        if had_ore {
            world.clear_ore(p);
            ore_lost += 1;
        }
    }
    (craters, ore_lost)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::SEA_LEVEL;

    fn island() -> World {
        let mut world = World::ocean(48, 48, 21);
        for y in 12..36 {
            for x in 12..36 {
                let p = TilePos::new(x, y);
                world.set_elevation(p, SEA_LEVEL + 160);
                world.set_moisture(p, 60);
            }
        }
        world
    }

    #[test]
    fn kinds_differ_in_power() {
        assert!(Explosive::Nuke.radius() > Explosive::Dynamite.radius());
        assert!(Explosive::Nuke.entity_damage() > Explosive::Dynamite.entity_damage());
        assert!(Explosive::Nuke.building_damage() > Explosive::Dynamite.building_damage());
        for kind in Explosive::ALL {
            assert!(!kind.label_ru().is_empty());
            assert!(!kind.hint_ru().is_empty());
            assert_ne!(kind.accent_rgb(), [0, 0, 0]);
        }
        assert_eq!(Explosive::Nuke.required_tech().name_ru(), "Атом");
    }

    #[test]
    fn crater_lowers_land_and_clears_ore() {
        let mut world = island();
        let center = TilePos::new(24, 24);
        world.set_ore(center, crate::ore::OreKind::Diamond, 5);
        let before = world.elevation(center);
        let mut rng = Rng::new(7);

        let (craters, ore_lost) = crater(&mut world, center, 5, 90, &mut rng);
        assert!(craters > 0);
        assert_eq!(ore_lost, 1);
        assert!(world.elevation(center) < before);
        assert_eq!(world.ore(center), None);
        assert_eq!(world.moisture(center), 0);
    }

    #[test]
    fn nuclear_crater_reaches_water() {
        let mut world = island();
        let center = TilePos::new(24, 24);
        let mut rng = Rng::new(9);
        crater(&mut world, center, Explosive::Nuke.radius(), 400, &mut rng);
        assert!(
            world.is_water(center),
            "глубокий кратер должен заполниться водой"
        );
    }

    #[test]
    fn report_text_is_readable() {
        let report = ExplosionReport {
            craters: 12,
            buildings_hit: 2,
            buildings_ruined: 1,
            entities_hurt: 3,
            entities_killed: 1,
            ..ExplosionReport::default()
        };
        let text = report.summary_ru(Explosive::Dynamite);
        assert!(text.starts_with("Взрыв динамита:"));
        assert!(text.contains("воронка: 12 тайлов"));
        assert!(text.contains("разрушено: 1"));
        assert_eq!(
            ExplosionReport::default().summary_ru(Explosive::Nuke),
            "Ядерный взрыв: лишь грохот"
        );
    }
}
