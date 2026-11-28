//! Разработки: страны постепенно изобретают технологии.
//!
//! Наука копится сама: дома дают немного очков исследований, лаборатория —
//! много, а добытая руда приносит очки сверху. Когда очков хватает на
//! следующую разработку из [`Tech::ALL`], страна её изобретает и получает
//! новые возможности:
//!
//! * **Добыча** — жители начинают выкапывать руду (и получать способности);
//! * **Кладка** — страна строит лабораторию рядом со столицей;
//! * **Химия** — лаборатория собирает динамит для войн;
//! * **Атом** — страна доходит до ядерного взрыва.
//!
//! Разработки хранятся маской в [`crate::Nation::tech`], поэтому «знает ли
//! страна технологию» — это один `&` с битом.

/// Разработка страны.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tech {
    /// Добыча руды и способности жителей.
    Mining = 0,
    /// Кладка: лаборатория и крепкие постройки.
    Masonry = 1,
    /// Химия: динамит.
    Chemistry = 2,
    /// Атом: ядерный взрыв.
    Atomic = 3,
}

impl Tech {
    /// Порядок изучения.
    pub const ALL: [Tech; 4] = [Self::Mining, Self::Masonry, Self::Chemistry, Self::Atomic];

    /// Бит в маске разработок.
    pub fn bit(self) -> u8 {
        1 << (self as u8)
    }

    /// Название для интерфейса и ленты событий.
    pub fn name_ru(self) -> &'static str {
        match self {
            Self::Mining => "Добыча",
            Self::Masonry => "Кладка",
            Self::Chemistry => "Химия",
            Self::Atomic => "Атом",
        }
    }

    /// Что даёт разработка.
    pub fn hint_ru(self) -> &'static str {
        match self {
            Self::Mining => "жители добывают руду и получают способности",
            Self::Masonry => "страна строит лабораторию",
            Self::Chemistry => "лаборатория собирает динамит",
            Self::Atomic => "страна собирает ядерную бомбу",
        }
    }

    /// Сколько очков исследований нужно.
    pub fn cost(self) -> u32 {
        match self {
            Self::Mining => 30,
            Self::Masonry => 90,
            Self::Chemistry => 170,
            Self::Atomic => 300,
        }
    }

    /// Сколько очков исследований даёт лаборатория за тик науки.
    pub fn laboratory_bonus(self) -> u32 {
        match self {
            Self::Chemistry => 2,
            Self::Atomic => 3,
            _ => 1,
        }
    }
}

/// Первая ещё не изученная разработка.
pub fn next_unknown(mask: u8) -> Option<Tech> {
    Tech::ALL
        .iter()
        .copied()
        .find(|tech| mask & tech.bit() == 0)
}

/// Сколько разработок изучено.
pub fn count(mask: u8) -> u32 {
    Tech::ALL
        .iter()
        .filter(|tech| mask & tech.bit() != 0)
        .count() as u32
}

/// Текстовая расшифровка: «Добыча, Кладка».
pub fn known_ru(mask: u8) -> String {
    let names: Vec<&str> = Tech::ALL
        .iter()
        .filter(|tech| mask & tech.bit() != 0)
        .map(|tech| tech.name_ru())
        .collect();
    if names.is_empty() {
        "нет разработок".to_string()
    } else {
        names.join(", ")
    }
}

/// Есть ли в маске хоть одна разработка.
pub fn any(mask: u8) -> bool {
    mask & Tech::ALL.iter().fold(0u8, |acc, tech| acc | tech.bit()) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_are_unique() {
        let mut mask = 0;
        for tech in Tech::ALL {
            assert_eq!(mask & tech.bit(), 0, "бит {} занят", tech.name_ru());
            mask |= tech.bit();
            assert!(!tech.hint_ru().is_empty());
            assert!(tech.cost() > 0);
        }
        assert!(any(mask));
        assert!(!any(0));
    }

    #[test]
    fn progression_is_ordered() {
        let mut mask = 0;
        assert_eq!(next_unknown(mask), Some(Tech::Mining));
        mask |= Tech::Mining.bit();
        assert_eq!(next_unknown(mask), Some(Tech::Masonry));
        mask |= Tech::Masonry.bit();
        mask |= Tech::Chemistry.bit();
        mask |= Tech::Atomic.bit();
        assert_eq!(next_unknown(mask), None);
        assert_eq!(count(mask), 4);
        assert_eq!(known_ru(mask), "Добыча, Кладка, Химия, Атом");
        assert_eq!(known_ru(0), "нет разработок");
        // Дорожает по ходу прогресса.
        assert!(Tech::Atomic.cost() > Tech::Chemistry.cost());
        assert!(Tech::Chemistry.cost() > Tech::Masonry.cost());
    }
}
