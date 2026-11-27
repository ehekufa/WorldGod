//! Существа: жители, воины, животные и монстры.

use crate::building::BuildingId;
use crate::grid::TilePos;
use crate::nation::NationId;
use crate::species::{Species, Stats};

/// Идентификатор существа. Уникален в пределах одной симуляции.
pub type EntityId = u32;

/// Текущее намерение существа — то, чем занят его «мозг».
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    /// Стоит на месте и ждёт следующей мысли.
    Idle,
    /// Гуляет к случайной точке.
    Wander {
        /// Куда идёт.
        goal: TilePos,
    },
    /// Идёт рубить дерево.
    Chop {
        /// Лесной тайл.
        tile: TilePos,
    },
    /// Идёт собирать еду.
    Forage {
        /// Лесной тайл с ягодами.
        tile: TilePos,
    },
    /// Строит дом или костёр.
    Work {
        /// Что именно строит.
        building: BuildingId,
    },
    /// Атакует врага.
    Attack {
        /// Кого бьёт.
        target: EntityId,
    },
    /// Убегает от угрозы.
    Flee {
        /// Откуда бежит.
        from: TilePos,
    },
}

impl Intent {
    /// Намерение требует действий?
    pub fn is_active(self) -> bool {
        !matches!(self, Self::Idle)
    }

    /// Цель на карте, если она известна без обращения к симуляции.
    pub fn static_goal(self) -> Option<TilePos> {
        match self {
            Self::Wander { goal } => Some(goal),
            Self::Chop { tile } | Self::Forage { tile } => Some(tile),
            Self::Flee { from } => Some(from),
            Self::Idle | Self::Work { .. } | Self::Attack { .. } => None,
        }
    }

    /// Подпись для панели выбранного существа.
    pub fn label_ru(self) -> &'static str {
        match self {
            Self::Idle => "Отдыхает",
            Self::Wander { .. } => "Гуляет",
            Self::Chop { .. } => "Заготавливает дерево",
            Self::Forage { .. } => "Собирает еду",
            Self::Work { .. } => "Строит",
            Self::Attack { .. } => "Сражается",
            Self::Flee { .. } => "Убегает",
        }
    }
}

/// Существо на карте.
#[derive(Clone, Debug)]
pub struct Entity {
    /// Идентификатор.
    pub id: EntityId,
    /// Вид.
    pub species: Species,
    /// Текущая клетка.
    pub pos: TilePos,
    /// Клетка на предыдущем тике — для плавной интерполяции рендера.
    pub prev_pos: TilePos,
    /// Здоровье.
    pub hp: i32,
    /// Страна (у людей и воинов).
    pub nation: Option<NationId>,
    /// Текущее намерение.
    pub intent: Intent,
    /// Очередь шагов пути (может быть пустой — тогда шаг жадный).
    pub path: Vec<TilePos>,
    /// Тиков до следующего шага.
    pub move_timer: u16,
    /// Тиков до следующего удара.
    pub attack_timer: u16,
    /// Тиков до следующего «размышления».
    pub think_timer: u16,
    /// Сколько дерева несёт.
    pub wood: u8,
    /// Голод 0..=[`Entity::MAX_HUNGER`].
    pub hunger: u16,
    /// Возраст в тиках.
    pub age: u32,
    /// Убийства.
    pub kills: u32,
    /// Кто ударил последним (для мести и паники).
    pub last_attacker: Option<EntityId>,
    /// Смотрит влево?
    pub facing_left: bool,
    /// Декоративный вариант спрайта.
    pub variant: u8,
    /// Живо ли существо.
    pub alive: bool,
}

impl Entity {
    /// Порог голода, после которого существо начинает голодать.
    pub const MAX_HUNGER: u16 = 255;

    /// Голод, при котором житель идёт искать еду.
    pub const HUNGRY: u16 = 70;

    /// Создаёт существо.
    pub fn new(id: EntityId, species: Species, pos: TilePos, variant: u8) -> Self {
        let stats = species.stats();
        Self {
            id,
            species,
            pos,
            prev_pos: pos,
            hp: stats.max_hp,
            nation: None,
            intent: Intent::Idle,
            path: Vec::new(),
            move_timer: 0,
            attack_timer: 0,
            think_timer: (id % 9) as u16,
            wood: 0,
            hunger: 0,
            age: 0,
            kills: 0,
            last_attacker: None,
            facing_left: false,
            variant,
            alive: true,
        }
    }

    /// Характеристики вида.
    pub fn stats(&self) -> &'static Stats {
        self.species.stats()
    }

    /// Максимальное здоровье.
    pub fn max_hp(&self) -> i32 {
        self.stats().max_hp
    }

    /// Доля здоровья 0.0..=1.0.
    pub fn health_fraction(&self) -> f32 {
        let max = self.max_hp().max(1);
        (self.hp as f32 / max as f32).clamp(0.0, 1.0)
    }

    /// Доля голода 0.0..=1.0.
    pub fn hunger_fraction(&self) -> f32 {
        (self.hunger as f32 / Self::MAX_HUNGER as f32).clamp(0.0, 1.0)
    }

    /// Живо ли существо (и не помечено ли мёртвым).
    pub fn is_alive(&self) -> bool {
        self.alive && self.hp > 0
    }

    /// Меняет намерение, сбрасывая устаревший путь.
    pub fn assign(&mut self, intent: Intent) {
        if self.intent != intent {
            self.intent = intent;
            self.path.clear();
        }
    }

    /// Короткая копия полей, нужных ИИ (чтобы не держать заимствование).
    pub fn view(&self) -> EntityView {
        EntityView {
            id: self.id,
            species: self.species,
            pos: self.pos,
            hp: self.hp,
            nation: self.nation,
            intent: self.intent,
            wood: self.wood,
            hunger: self.hunger,
        }
    }
}

/// Снимок существа для принятия решений.
#[derive(Clone, Copy, Debug)]
pub struct EntityView {
    /// Идентификатор.
    pub id: EntityId,
    /// Вид.
    pub species: Species,
    /// Клетка.
    pub pos: TilePos,
    /// Здоровье.
    pub hp: i32,
    /// Страна.
    pub nation: Option<NationId>,
    /// Намерение.
    pub intent: Intent,
    /// Дерево в руках.
    pub wood: u8,
    /// Голод.
    pub hunger: u16,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::pos;

    #[test]
    fn new_entity_is_alive_and_full() {
        let entity = Entity::new(7, Species::Human, pos(3, 4), 1);
        assert!(entity.is_alive());
        assert_eq!(entity.hp, entity.max_hp());
        assert_eq!(entity.health_fraction(), 1.0);
        assert_eq!(entity.hunger, 0);
        assert_eq!(entity.pos, entity.prev_pos);
        assert!(entity.think_timer <= 9);
    }

    #[test]
    fn assigning_new_intent_clears_path() {
        let mut entity = Entity::new(1, Species::Human, pos(0, 0), 0);
        entity.path = vec![pos(1, 0), pos(2, 0)];
        entity.assign(Intent::Wander { goal: pos(5, 5) });
        assert!(entity.path.is_empty());
        assert_eq!(
            entity.intent,
            Intent::Wander {
                goal: pos(5, 5)
            }
        );
        // Повторное назначение того же намерения путь не трогает.
        entity.path = vec![pos(1, 0)];
        entity.assign(Intent::Wander { goal: pos(5, 5) });
        assert_eq!(entity.path.len(), 1);
    }

    #[test]
    fn death_is_detected() {
        let mut entity = Entity::new(2, Species::Wolf, pos(0, 0), 0);
        entity.hp = 0;
        assert!(!entity.is_alive());
        entity.alive = false;
        assert!(!entity.is_alive());
    }

    #[test]
    fn view_copies_fields() {
        let entity = Entity::new(5, Species::Soldier, pos(9, 9), 2);
        let view = entity.view();
        assert_eq!(view.id, 5);
        assert_eq!(view.species, Species::Soldier);
        assert_eq!(view.pos, pos(9, 9));
    }
}
