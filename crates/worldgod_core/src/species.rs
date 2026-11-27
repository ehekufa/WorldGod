//! Виды существ и их характеристики.

/// Вид существа.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Species {
    /// Мирный житель: строит дома и основывает страну.
    Human = 0,
    /// Воин: защищает страну от монстров.
    Soldier = 1,
    /// Олень: мирная добыча волков.
    Deer = 2,
    /// Волк: охотится на людей и оленей.
    Wolf = 3,
    /// Зомби: убивает людей и превращает их в зомби.
    Zombie = 4,
}

impl Species {
    /// Все виды.
    pub const ALL: [Species; 5] = [
        Self::Human,
        Self::Soldier,
        Self::Deer,
        Self::Wolf,
        Self::Zombie,
    ];

    /// Характеристики вида.
    pub fn stats(self) -> &'static Stats {
        &STATS[self as usize]
    }

    /// Название для интерфейса.
    pub fn name_ru(self) -> &'static str {
        match self {
            Self::Human => "Человек",
            Self::Soldier => "Воин",
            Self::Deer => "Олень",
            Self::Wolf => "Волк",
            Self::Zombie => "Зомби",
        }
    }

    /// Название в родительном падеже («погиб от клыков волка»).
    pub fn name_genitive_ru(self) -> &'static str {
        match self {
            Self::Human => "человека",
            Self::Soldier => "воина",
            Self::Deer => "оленя",
            Self::Wolf => "волка",
            Self::Zombie => "зомби",
        }
    }

    /// Люди и воины — разумные, живут в странах.
    pub fn is_humanoid(self) -> bool {
        matches!(self, Self::Human | Self::Soldier)
    }

    /// Монстры — враги людей.
    pub fn is_monster(self) -> bool {
        matches!(self, Self::Wolf | Self::Zombie)
    }

    /// Животное, которое просто живёт своей жизнью.
    pub fn is_animal(self) -> bool {
        self == Self::Deer
    }

    /// Может ли вид основать страну.
    pub fn can_found_nation(self) -> bool {
        self == Self::Human
    }

    /// Код для сохранений.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Восстановление из кода.
    pub fn from_u8(value: u8) -> Option<Self> {
        Self::ALL.get(value as usize).copied()
    }

    /// Враждебны ли два вида друг другу.
    pub fn hostile_to(self, other: Species) -> bool {
        if self == other {
            return false;
        }
        if self.is_monster() && other.is_humanoid() {
            return true;
        }
        if self.is_humanoid() && other.is_monster() {
            return true;
        }
        // Волки охотятся на оленей, но зомби оленей не трогают.
        (self == Self::Wolf && other == Self::Deer) || (self == Self::Deer && other == Self::Wolf)
    }
}

/// Числовые характеристики вида.
#[derive(Clone, Copy, Debug)]
pub struct Stats {
    /// Максимальное здоровье.
    pub max_hp: i32,
    /// Урон за удар.
    pub damage: i32,
    /// Дистанция удара в тайлах.
    pub attack_range: f32,
    /// Пауза между ударами, тиков.
    pub attack_cooldown: u16,
    /// Пауза между шагами, тиков.
    pub move_interval: u16,
    /// Радиус обнаружения врага.
    pub aggro_radius: i32,
    /// Радиус паники.
    pub flee_radius: i32,
    /// Продолжительность жизни, тиков.
    pub max_age: u32,
    /// Как часто растёт голод (раз в N тиков).
    pub hunger_rate: u16,
    /// Сколько дерева нужно, чтобы основать страну.
    pub wood_for_nation: u8,
    /// Сколько дерева нужно на дом.
    pub wood_for_house: u8,
}

/// Таблица характеристик, индексированная значением [`Species`].
pub static STATS: [Stats; 5] = [
    // Человек: слабый, но упорный строитель.
    Stats {
        max_hp: 100,
        damage: 4,
        attack_range: 1.5,
        attack_cooldown: 9,
        move_interval: 4,
        aggro_radius: 0,
        flee_radius: 6,
        max_age: 9_000,
        hunger_rate: 12,
        wood_for_nation: 2,
        wood_for_house: 3,
    },
    // Воин: крепкий, бьёт больно, сам ищет врага.
    Stats {
        max_hp: 170,
        damage: 13,
        attack_range: 1.7,
        attack_cooldown: 6,
        move_interval: 3,
        aggro_radius: 9,
        flee_radius: 0,
        max_age: 12_000,
        hunger_rate: 14,
        wood_for_nation: 0,
        wood_for_house: 0,
    },
    // Олень: быстрый и пугливый.
    Stats {
        max_hp: 60,
        damage: 0,
        attack_range: 0.0,
        attack_cooldown: 30,
        move_interval: 3,
        aggro_radius: 0,
        flee_radius: 5,
        max_age: 5_000,
        hunger_rate: 20,
        wood_for_nation: 0,
        wood_for_house: 0,
    },
    // Волк: стайный хищник.
    Stats {
        max_hp: 95,
        damage: 9,
        attack_range: 1.5,
        attack_cooldown: 7,
        move_interval: 3,
        aggro_radius: 8,
        flee_radius: 0,
        max_age: 6_000,
        hunger_rate: 18,
        wood_for_nation: 0,
        wood_for_house: 0,
    },
    // Зомби: медленный, живучий, заражает убитых.
    Stats {
        max_hp: 120,
        damage: 7,
        attack_range: 1.5,
        attack_cooldown: 9,
        move_interval: 5,
        aggro_radius: 10,
        flee_radius: 0,
        max_age: 500_000,
        hunger_rate: 0,
        wood_for_nation: 0,
        wood_for_house: 0,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_table_matches_species() {
        assert_eq!(STATS.len(), Species::ALL.len());
        for species in Species::ALL {
            let stats = species.stats();
            assert!(stats.max_hp > 0);
            assert_eq!(Species::from_u8(species.as_u8()), Some(species));
        }
        assert_eq!(Species::from_u8(200), None);
    }

    #[test]
    fn hostility_is_symmetric_for_monsters() {
        for a in Species::ALL {
            for b in Species::ALL {
                assert_eq!(
                    a.hostile_to(b),
                    b.hostile_to(a),
                    "{a:?} и {b:?} враждуют несимметрично"
                );
            }
        }
        assert!(Species::Wolf.hostile_to(Species::Human));
        assert!(Species::Zombie.hostile_to(Species::Soldier));
        assert!(Species::Wolf.hostile_to(Species::Deer));
        assert!(!Species::Human.hostile_to(Species::Human));
        assert!(!Species::Deer.hostile_to(Species::Human));
        assert!(!Species::Zombie.hostile_to(Species::Deer));
    }

    #[test]
    fn roles() {
        assert!(Species::Human.is_humanoid());
        assert!(Species::Soldier.is_humanoid());
        assert!(Species::Wolf.is_monster());
        assert!(Species::Zombie.is_monster());
        assert!(Species::Deer.is_animal());
        assert!(Species::Human.can_found_nation());
        assert!(!Species::Soldier.can_found_nation());
    }
}
