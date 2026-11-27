//! Страны: имя, цвет, столица и счётчики.

use crate::grid::TilePos;

/// Идентификатор страны.
pub type NationId = u32;

/// Палитра цветов стран (различимы на карте и на тёмном фоне интерфейса).
pub const NATION_PALETTE: [[u8; 3]; 10] = [
    [214, 96, 77],
    [232, 168, 74],
    [126, 176, 96],
    [86, 168, 152],
    [96, 132, 208],
    [156, 116, 200],
    [214, 120, 168],
    [196, 186, 110],
    [120, 190, 210],
    [200, 140, 90],
];

/// Страна, основанная человечеством.
#[derive(Clone, Debug)]
pub struct Nation {
    /// Идентификатор.
    pub id: NationId,
    /// Название, например «Велдор».
    pub name: String,
    /// Цвет флага и территорий.
    pub color: [u8; 3],
    /// Столица — место первого костра.
    pub capital: TilePos,
    /// Тик основания.
    pub founded_tick: u64,
    /// Количество домов.
    pub houses: u32,
    /// Текущее население.
    pub population: u32,
    /// Воины в населении.
    pub soldiers: u32,
    /// Убитые монстры.
    pub kills: u32,
    /// Потери жителей.
    pub losses: u32,
    /// Сколько жителей было за всю историю.
    pub peak_population: u32,
}

impl Nation {
    /// Новая страна.
    pub fn new(id: NationId, name: String, color: [u8; 3], capital: TilePos, tick: u64) -> Self {
        Self {
            id,
            name,
            color,
            capital,
            founded_tick: tick,
            houses: 0,
            population: 0,
            soldiers: 0,
            kills: 0,
            losses: 0,
            peak_population: 0,
        }
    }

    /// Сколько жителей страна может прокормить при текущем числе домов.
    pub fn capacity(&self) -> u32 {
        2 + self.houses * 3
    }

    /// Доля заселённости (0.0..=1.0) для индикатора в интерфейсе.
    pub fn housing_usage(&self) -> f32 {
        let capacity = self.capacity();
        if capacity == 0 {
            0.0
        } else {
            (self.population as f32 / capacity as f32).clamp(0.0, 1.0)
        }
    }

    /// Учитывает нового жителя.
    pub fn add_citizen(&mut self, is_soldier: bool) {
        self.population += 1;
        if is_soldier {
            self.soldiers += 1;
        }
        self.peak_population = self.peak_population.max(self.population);
    }

    /// Учитывает потерю жителя.
    pub fn remove_citizen(&mut self, is_soldier: bool) {
        self.population = self.population.saturating_sub(1);
        if is_soldier {
            self.soldiers = self.soldiers.saturating_sub(1);
        }
    }

    /// Цвет страны, осветлённый на `factor` (для текста и иконок).
    pub fn text_color(&self) -> [u8; 3] {
        crate::math::brightness(self.color, 1.25)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::pos;

    #[test]
    fn capacity_grows_with_houses() {
        let mut nation = Nation::new(1, "Велдор".into(), NATION_PALETTE[0], pos(10, 10), 0);
        assert_eq!(nation.capacity(), 2);
        nation.houses = 3;
        assert_eq!(nation.capacity(), 11);
    }

    #[test]
    fn population_counters() {
        let mut nation = Nation::new(1, "Велдор".into(), NATION_PALETTE[0], pos(10, 10), 0);
        nation.add_citizen(false);
        nation.add_citizen(true);
        assert_eq!(nation.population, 2);
        assert_eq!(nation.soldiers, 1);
        assert_eq!(nation.peak_population, 2);
        nation.remove_citizen(true);
        assert_eq!(nation.population, 1);
        assert_eq!(nation.soldiers, 0);
        nation.remove_citizen(false);
        nation.remove_citizen(false);
        assert_eq!(nation.population, 0);
        assert_eq!(nation.housing_usage(), 0.0);
    }
}
