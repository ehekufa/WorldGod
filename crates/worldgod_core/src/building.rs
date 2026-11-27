//! Постройки: костры-столицы, дома и руины.

use crate::grid::TilePos;
use crate::nation::NationId;

/// Идентификатор постройки.
pub type BuildingId = u32;

/// Тип постройки.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuildingKind {
    /// Костёр — символ основания страны.
    Campfire = 0,
    /// Дом: даёт жильё и укрытие.
    House = 1,
    /// Руины: дом был разрушен.
    Ruin = 2,
}

impl BuildingKind {
    /// Все виды.
    pub const ALL: [BuildingKind; 3] = [Self::Campfire, Self::House, Self::Ruin];

    /// Название для интерфейса.
    pub fn name_ru(self) -> &'static str {
        match self {
            Self::Campfire => "Костёр",
            Self::House => "Дом",
            Self::Ruin => "Руины",
        }
    }

    /// Занимает ли клетку целиком (по ней нельзя ходить).
    pub fn blocks_movement(self) -> bool {
        matches!(self, Self::House)
    }

    /// Сколько рабочих единиц нужно на постройку.
    pub fn work_required(self) -> u16 {
        match self {
            Self::Campfire => 1,
            Self::House => 40,
            Self::Ruin => 1,
        }
    }

    /// Даёт ли жильё.
    pub fn housing(self) -> u32 {
        match self {
            Self::House => 1,
            _ => 0,
        }
    }

    /// Нужен ли для неё фундамент из готовых построек.
    pub fn is_permanent(self) -> bool {
        self != Self::Ruin
    }
}

/// Конкретная постройка на карте.
#[derive(Clone, Debug)]
pub struct Building {
    /// Идентификатор.
    pub id: BuildingId,
    /// Тип.
    pub kind: BuildingKind,
    /// Клетка, которую занимает постройка.
    pub pos: TilePos,
    /// Владелец-страна.
    pub nation: Option<NationId>,
    /// Выполненная работа.
    pub progress: u16,
    /// Требуемая работа.
    pub work_required: u16,
    /// Декоративный вариант.
    pub variant: u8,
    /// Тик постройки.
    pub built_tick: Option<u64>,
}

impl Building {
    /// Новая постройка (например, готовый костёр или фундамент дома).
    pub fn new(id: BuildingId, kind: BuildingKind, pos: TilePos, nation: Option<NationId>) -> Self {
        Self {
            id,
            kind,
            pos,
            nation,
            progress: 0,
            work_required: kind.work_required(),
            // Вариант задаёт мелкие отличия в спрайте (окно, труба, флаг).
            variant: (id % 4) as u8,
            built_tick: None,
        }
    }

    /// Готовая постройка (ставится сразу завершённой).
    pub fn finished(
        id: BuildingId,
        kind: BuildingKind,
        pos: TilePos,
        nation: Option<NationId>,
        tick: u64,
    ) -> Self {
        let mut building = Self::new(id, kind, pos, nation);
        building.progress = building.work_required;
        building.built_tick = Some(tick);
        building
    }

    /// Стройка завершена?
    pub fn is_complete(&self) -> bool {
        self.progress >= self.work_required
    }

    /// Это ещё строительная площадка?
    pub fn is_site(&self) -> bool {
        !self.is_complete()
    }

    /// Доля готовности 0.0..=1.0.
    pub fn completion(&self) -> f32 {
        if self.work_required == 0 {
            1.0
        } else {
            (self.progress as f32 / self.work_required as f32).clamp(0.0, 1.0)
        }
    }

    /// Добавляет работу. Возвращает `true`, если постройка только что завершилась.
    pub fn add_work(&mut self, amount: u16, tick: u64) -> bool {
        if self.is_complete() {
            return false;
        }
        self.progress = self.progress.saturating_add(amount);
        if self.is_complete() {
            self.built_tick = Some(tick);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::pos;

    #[test]
    fn site_becomes_complete() {
        let mut house = Building::new(1, BuildingKind::House, pos(5, 5), Some(1));
        assert!(house.is_site());
        assert_eq!(house.completion(), 0.0);
        assert!(!house.add_work(10, 1));
        assert!((house.completion() - 0.25).abs() < 1e-6);
        assert!(house.add_work(100, 2));
        assert!(house.is_complete());
        assert_eq!(house.completion(), 1.0);
        assert_eq!(house.built_tick, Some(2));
        // Повторная работа ничего не делает.
        assert!(!house.add_work(5, 3));
    }

    #[test]
    fn campfire_is_instant() {
        let fire = Building::finished(2, BuildingKind::Campfire, pos(1, 1), Some(1), 5);
        assert!(fire.is_complete());
        assert_eq!(fire.built_tick, Some(5));
        assert!(!BuildingKind::Campfire.blocks_movement());
        assert!(BuildingKind::House.blocks_movement());
    }
}
