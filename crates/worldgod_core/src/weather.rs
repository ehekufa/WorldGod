//! Стихии: ураган, вспышка солнца, дождь и снегопад.
//!
//! Стихия — это большая кисть, которая меняет не рельеф напрямую, а
//! «погодные» свойства тайлов: влажность, снежный покров и высоту у воды.
//! Тип рельефа по-прежнему выводится из высоты и влажности
//! ([`crate::terrain::terrain_at`]), поэтому ураган действительно срывает лес
//! (влажность падает — лес становится лугом), а снегопад оставляет тайл
//! прежним, но накрывает его снегом.
//!
//! Урон постройкам и существам наносит [`crate::Simulation::cast_weather`]:
//! модуль знает только про тайлы и не зависит от симуляции.

use crate::grid::TilePos;
use crate::rng::Rng;
use crate::terrain::{Terrain, MAX_ELEVATION};
use crate::world::World;

/// Стихия, которую игрок обрушивает на мир.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Weather {
    /// Ураган: срывает лес, сдувает снег, ломает постройки.
    Hurricane,
    /// Вспышка солнца: жара, сушь, море отступает, всё живое горит.
    SolarFlare,
    /// Дождь: влага возвращается, лес растёт, снег тает.
    Rain,
    /// Снегопад: земля укрывается снегом, ходить становится медленнее.
    Snowfall,
}

impl Weather {
    /// Все стихии в порядке панели.
    pub const ALL: [Weather; 4] = [
        Self::Hurricane,
        Self::SolarFlare,
        Self::Rain,
        Self::Snowfall,
    ];

    /// Подпись на кнопке.
    pub fn label_ru(self) -> &'static str {
        match self {
            Self::Hurricane => "Ураган",
            Self::SolarFlare => "Вспышка",
            Self::Rain => "Дождь",
            Self::Snowfall => "Снегопад",
        }
    }

    /// Полное название для ленты событий.
    pub fn name_ru(self) -> &'static str {
        match self {
            Self::Hurricane => "Ураган",
            Self::SolarFlare => "Вспышка солнца",
            Self::Rain => "Дождь",
            Self::Snowfall => "Снегопад",
        }
    }

    /// Подсказка под кнопкой.
    pub fn hint_ru(self) -> &'static str {
        match self {
            Self::Hurricane => "Срывает лес, сдувает снег и ломает постройки",
            Self::SolarFlare => "Жара: сушит землю, испаряет море, жжёт всё живое",
            Self::Rain => "Влага: лес растёт, снег тает, пожарам конец",
            Self::Snowfall => "Укрывает землю снегом, ходить по нему медленно",
        }
    }

    /// Цвет иконки и подсветки кисти.
    pub fn accent_rgb(self) -> [u8; 3] {
        match self {
            Self::Hurricane => [146, 198, 226],
            Self::SolarFlare => [255, 188, 92],
            Self::Rain => [112, 162, 220],
            Self::Snowfall => [224, 238, 252],
        }
    }

    /// Стихия бьёт по постройкам и существам?
    pub fn is_extreme(self) -> bool {
        matches!(self, Self::Hurricane | Self::SolarFlare)
    }

    /// Урон постройкам за одно применение.
    pub fn building_damage(self) -> i32 {
        match self {
            Self::Hurricane => 24,
            Self::SolarFlare => 32,
            Self::Rain | Self::Snowfall => 0,
        }
    }

    /// Урон существам за одно применение.
    pub fn entity_damage(self) -> i32 {
        match self {
            Self::Hurricane => 12,
            Self::SolarFlare => 22,
            Self::Rain | Self::Snowfall => 0,
        }
    }
}

/// Что именно натворила стихия.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WeatherReport {
    /// Сколько тайлов изменилось.
    pub tiles: u32,
    /// Сколько лесных тайлов потеряло листву.
    pub forest_lost: u32,
    /// Сколько тайлов присыпало снегом.
    pub snow_added: u32,
    /// Сколько тайлов освободилось от снега.
    pub snow_melted: u32,
    /// Сколько водных тайлов испарилось (море отступило).
    pub water_evaporated: u32,
    /// Сколько построек получило урон.
    pub buildings_hit: u32,
    /// Сколько построек разрушено.
    pub buildings_ruined: u32,
    /// Сколько существ пострадало.
    pub entities_hurt: u32,
    /// Сколько существ погибло.
    pub entities_killed: u32,
}

impl WeatherReport {
    /// Короткая строка для ленты событий: «Ураган: 214 тайлов, 3 постройки».
    pub fn summary_ru(&self, kind: Weather) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.tiles > 0 {
            parts.push(format!("{} тайл{}", self.tiles, plural_suffix(self.tiles)));
        }
        if self.forest_lost > 0 {
            parts.push(format!("лес сорван: {}", self.forest_lost));
        }
        if self.water_evaporated > 0 {
            parts.push(format!("море отступило: {}", self.water_evaporated));
        }
        if self.snow_added > 0 {
            parts.push(format!("под снегом: {}", self.snow_added));
        }
        if self.snow_melted > 0 {
            parts.push(format!("снег сошёл: {}", self.snow_melted));
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
            return format!("{}: прошла мимо", kind.name_ru());
        }
        format!("{}: {}", kind.name_ru(), parts.join(" · "))
    }
}

/// Русское окончание для слова «тайл» по числу: 1 тайл, 2 тайла, 5 тайлов.
fn plural_suffix(n: u32) -> &'static str {
    let tail = n % 100;
    if (11..=14).contains(&tail) {
        return "ов";
    }
    match n % 10 {
        1 => "",
        2..=4 => "а",
        _ => "ов",
    }
}

/// Обрушивает стихию на круг тайлов. Возвращает отчёт о тайловых изменениях.
///
/// Урон постройкам и существам считается в симуляции: этот модуль
/// принципиально не знает ни о ком, кроме тайлов.
pub fn paint(
    world: &mut World,
    kind: Weather,
    center: TilePos,
    radius: i32,
    rng: &mut Rng,
) -> WeatherReport {
    let mut report = WeatherReport::default();
    let radius = radius.clamp(0, 64);

    world.for_each_in_radius_mut(center, radius, |_p, distance, tile| {
        // Как и у кистей рельефа: в центре сильнее, по краю слабее.
        let strength = ((1.0 - distance) * (0.75 + rng.next_f32() * 0.5)).clamp(0.0, 1.0);
        if strength <= 0.05 {
            return;
        }

        let was_water = tile.terrain().is_water();
        let snow_before = tile.snow;

        match kind {
            Weather::Hurricane => {
                // Ветер срывает кроны: влажность падает, лес редеет до луга.
                if matches!(tile.terrain(), Terrain::Forest) && strength > 0.25 {
                    tile.moisture = tile.moisture.saturating_sub((120.0 * strength) as u8);
                    report.forest_lost += 1;
                } else if !was_water {
                    tile.moisture = tile.moisture.saturating_sub((25.0 * strength) as u8);
                }
                // Снег сдувает в море.
                if snow_before > 0 {
                    tile.snow = (snow_before as f32 * (1.0 - strength * 0.8)) as u8;
                }
            }
            Weather::SolarFlare => {
                if was_water {
                    // Жара испаряет воду: мелководье уходит, берег наступает.
                    let rise = (strength * 16.0) as u16;
                    let target = tile.elevation.saturating_add(rise).min(MAX_ELEVATION);
                    if target > tile.elevation {
                        tile.elevation = target;
                        report.water_evaporated += 1;
                    }
                } else {
                    // Сушь: трава жухнет, лес превращается в выжженный луг.
                    if matches!(tile.terrain(), Terrain::Forest) {
                        report.forest_lost += 1;
                    }
                    tile.moisture = tile.moisture.saturating_sub((170.0 * strength) as u8);
                }
                // Снег мгновенно тает.
                if snow_before > 0 {
                    tile.snow = (snow_before as f32 * (1.0 - strength)) as u8;
                }
            }
            Weather::Rain => {
                if !was_water {
                    // Насыщает влагой: слабый дождь поит траву, сильный рождает лес.
                    let target = (120.0 + strength * 250.0) as u8;
                    tile.moisture = tile.moisture.max(target);
                    // Дождь подтапливает низины, но сушу не топит.
                    if tile.snow > 0 {
                        tile.snow = tile.snow.saturating_sub((70.0 * strength) as u8);
                    }
                }
            }
            Weather::Snowfall => {
                if !was_water {
                    // Снег ложится слоем: центр кисти — сугроб, край — позёмка.
                    let target = (90.0 + strength * 165.0) as u8;
                    tile.snow = tile.snow.max(target);
                }
            }
        }

        if tile.snow > snow_before {
            report.snow_added += 1;
        } else if tile.snow < snow_before {
            report.snow_melted += 1;
        }
        report.tiles += 1;
    });

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::SEA_LEVEL;
    use crate::world::World;

    fn island() -> (World, TilePos) {
        let mut world = World::ocean(48, 48, 7);
        let center = TilePos::new(24, 24);
        for p in [
            center,
            TilePos::new(23, 24),
            TilePos::new(25, 24),
            TilePos::new(24, 23),
            TilePos::new(24, 25),
        ] {
            world.set_elevation(p, SEA_LEVEL + 120);
            world.set_moisture(p, 220);
        }
        (world, center)
    }

    #[test]
    fn names_and_colors_are_set() {
        for kind in Weather::ALL {
            assert!(!kind.label_ru().is_empty());
            assert!(!kind.name_ru().is_empty());
            assert!(!kind.hint_ru().is_empty());
            assert_ne!(kind.accent_rgb(), [0, 0, 0]);
        }
        assert!(Weather::Hurricane.is_extreme());
        assert!(!Weather::Rain.is_extreme());
        assert_eq!(Weather::Rain.building_damage(), 0);
        assert!(Weather::SolarFlare.entity_damage() > 0);
    }

    #[test]
    fn snowfall_covers_land_but_keeps_terrain() {
        let (mut world, center) = island();
        let terrain_before = world.terrain(center);
        let mut rng = Rng::new(1);
        let report = paint(&mut world, Weather::Snowfall, center, 3, &mut rng);

        assert!(report.snow_added > 0);
        assert!(world.snow(center) > 100, "в центре должен быть сугроб");
        // Рельеф не подменился: снег — это погодный слой.
        assert_eq!(world.terrain(center), terrain_before);
        assert!(world.snow_tiles() > 0);
    }

    #[test]
    fn hurricane_strips_the_forest() {
        let (mut world, center) = island();
        assert_eq!(world.terrain(center), Terrain::Forest);
        let mut rng = Rng::new(2);
        let report = paint(&mut world, Weather::Hurricane, center, 3, &mut rng);

        assert!(report.forest_lost > 0);
        assert_eq!(world.terrain(center), Terrain::Grass, "лес должен стать лугом");
    }

    #[test]
    fn solar_flare_dries_land_and_evaporates_the_sea() {
        let (mut world, center) = island();
        let shore = TilePos::new(24, 22);
        world.set_elevation(shore, SEA_LEVEL - 20);
        let mut rng = Rng::new(3);
        let report = paint(&mut world, Weather::SolarFlare, center, 4, &mut rng);

        assert!(
            world.moisture(center) < 60,
            "сушь выжигает влагу: {}",
            world.moisture(center)
        );
        assert!(report.water_evaporated > 0, "море должно отступить");
        assert!(world.elevation(shore) > SEA_LEVEL - 20);
    }

    #[test]
    fn rain_brings_the_forest_back_and_melts_snow() {
        let (mut world, center) = island();
        world.set_moisture(center, 20);
        world.set_snow(center, 200);
        let mut rng = Rng::new(4);
        let report = paint(&mut world, Weather::Rain, center, 2, &mut rng);

        assert!(report.snow_melted > 0);
        assert!(world.snow(center) < 200);
        assert!(world.moisture(center) > 165, "после дождя вырастает лес");
        assert_eq!(world.terrain(center), Terrain::Forest);
    }

    #[test]
    fn report_summary_is_russian() {
        let report = WeatherReport {
            tiles: 21,
            forest_lost: 5,
            snow_added: 7,
            ..WeatherReport::default()
        };
        let text = report.summary_ru(Weather::Snowfall);
        assert!(text.starts_with("Снегопад:"));
        assert!(text.contains("21 тайл"), "неверное согласование: {text}");
        assert!(text.contains("под снегом: 7"));

        let empty = WeatherReport::default().summary_ru(Weather::Rain);
        assert_eq!(empty, "Дождь: прошла мимо");
        assert_eq!(plural_suffix(2), "а");
        assert_eq!(plural_suffix(5), "ов");
        assert_eq!(plural_suffix(11), "ов");
    }

    #[test]
    fn snow_layer_can_be_melted() {
        let (mut world, center) = island();
        let mut rng = Rng::new(5);
        paint(&mut world, Weather::Snowfall, center, 2, &mut rng);
        assert!(world.snow(center) > 0);
        world.melt_snow(255);
        assert_eq!(world.snow(center), 0);
        assert_eq!(world.snow_tiles(), 0);
    }
}
