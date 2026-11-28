//! Постройки: костры-столицы, дома, порты и руины.

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
    /// Порт: от него страна спускает на воду лодки.
    Port = 3,
    /// Лаборатория: собственный дом страны, где копятся разработки.
    Laboratory = 4,
}

impl BuildingKind {
    /// Все виды.
    pub const ALL: [BuildingKind; 5] = [
        Self::Campfire,
        Self::House,
        Self::Ruin,
        Self::Port,
        Self::Laboratory,
    ];

    /// Название для интерфейса.
    pub fn name_ru(self) -> &'static str {
        match self {
            Self::Campfire => "Костёр",
            Self::House => "Дом",
            Self::Ruin => "Руины",
            Self::Port => "Порт",
            Self::Laboratory => "Лаборатория",
        }
    }

    /// Занимает ли клетку целиком (по ней нельзя ходить).
    pub fn blocks_movement(self) -> bool {
        matches!(self, Self::House | Self::Laboratory)
    }

    /// Сколько рабочих единиц нужно на постройку.
    pub fn work_required(self) -> u16 {
        match self {
            Self::Campfire => 1,
            Self::House => 40,
            Self::Ruin => 1,
            Self::Port => 60,
            Self::Laboratory => 80,
        }
    }

    /// Сколько здоровья у постройки.
    pub fn max_hp(self) -> i32 {
        match self {
            Self::Campfire => 160,
            Self::House => 90,
            Self::Ruin => 0,
            Self::Port => 130,
            Self::Laboratory => 140,
        }
    }

    /// Можно ли сломать постройку в бою.
    pub fn can_be_destroyed(self) -> bool {
        matches!(
            self,
            Self::House | Self::Port | Self::Campfire | Self::Laboratory
        )
    }

    /// Ставится ли порт только у воды (на берегу).
    pub fn needs_shore(self) -> bool {
        self == Self::Port
    }

    /// Даёт ли жильё.
    pub fn housing(self) -> u32 {
        match self {
            Self::House => 1,
            _ => 0,
        }
    }

    /// Даёт ли постройка очки исследований за тик науки.
    pub fn research_per_tick(self) -> u32 {
        match self {
            Self::Laboratory => 3,
            Self::Campfire | Self::House => 1,
            _ => 0,
        }
    }

    /// Нужен ли для неё фундамент из готовых построек.
    pub fn is_permanent(self) -> bool {
        self != Self::Ruin
    }

    /// Радиус территории, которую держит постройка.
    pub fn territory_radius(self) -> i32 {
        match self {
            Self::Campfire => 9,
            Self::House | Self::Port => 4,
            Self::Laboratory => 5,
            Self::Ruin => 0,
        }
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
    /// Текущее здоровье: враги могут разломать дом.
    pub hp: i32,
    /// Максимальное здоровье.
    pub max_hp: i32,
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
            hp: kind.max_hp(),
            max_hp: kind.max_hp(),
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

    /// Доля здоровья 0.0..=1.0 (для полоски и руин).
    pub fn health_fraction(&self) -> f32 {
        if self.max_hp <= 0 {
            return 0.0;
        }
        (self.hp as f32 / self.max_hp as f32).clamp(0.0, 1.0)
    }

    /// Разрушена?
    pub fn is_destroyed(&self) -> bool {
        self.hp <= 0 || self.kind == BuildingKind::Ruin
    }

    /// Наносит урон. Возвращает `true`, если постройка только что разрушилась.
    pub fn damage(&mut self, amount: i32) -> bool {
        if !self.kind.can_be_destroyed() || self.hp <= 0 {
            return false;
        }
        self.hp -= amount.max(1);
        self.hp <= 0
    }

    /// Превращает постройку в руины: дом больше не держит территорию.
    pub fn collapse(&mut self) {
        self.kind = BuildingKind::Ruin;
        self.hp = 0;
        self.max_hp = 0;
        self.progress = self.work_required;
        self.built_tick = None;
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
    fn house_can_be_destroyed() {
        let mut house = Building::finished(7, BuildingKind::House, pos(4, 4), Some(2), 10);
        let max_hp = house.max_hp;
        assert!(max_hp > 0);
        assert!(!house.is_destroyed());
        assert!(!house.damage(max_hp / 2));
        assert!((house.health_fraction() - 0.5).abs() < 0.6);
        assert!(house.damage(max_hp));
        assert!(house.is_destroyed());

        house.collapse();
        assert_eq!(house.kind, BuildingKind::Ruin);
        assert_eq!(house.hp, 0);
        assert!(!BuildingKind::Ruin.can_be_destroyed());
        assert!(!BuildingKind::Ruin.is_permanent());
        assert_eq!(BuildingKind::Port.name_ru(), "Порт");
        assert_eq!(BuildingKind::Laboratory.name_ru(), "Лаборатория");
        assert!(BuildingKind::Laboratory.can_be_destroyed());
        assert!(BuildingKind::Laboratory.research_per_tick() > 0);
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
