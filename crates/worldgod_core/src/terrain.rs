//! Типы рельефа и правило «высота + влажность → тип тайла».

/// Уровень моря: всё, что ниже, — вода.
pub const SEA_LEVEL: u16 = 320;

/// Максимальная высота тайла.
pub const MAX_ELEVATION: u16 = 1000;

/// Тип поверхности тайла.
///
/// Тип не хранится в тайле напрямую — он вычисляется из высоты и влажности
/// функцией [`terrain_at`]. Благодаря этому кисти работают с рельефом
/// («поднять сушу», «срезать гору»), а не подменяют картинку.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Terrain {
    /// Глубокий океан.
    DeepWater = 0,
    /// Открытый океан.
    Ocean = 1,
    /// Мелководье у берега.
    ShallowWater = 2,
    /// Песчаный пляж.
    Beach = 3,
    /// Луг.
    Grass = 4,
    /// Лес: древесина, еда, укрытие.
    Forest = 5,
    /// Холмы: проходимы, но медленнее.
    Hills = 6,
    /// Горы: непроходимы.
    Mountain = 7,
    /// Снежные вершины: непроходимы.
    Snow = 8,
}

impl Terrain {
    /// Количество вариантов рельефа.
    pub const COUNT: usize = 9;

    /// Все варианты по порядку — удобно для таблиц и статистики.
    pub const ALL: [Terrain; 9] = [
        Self::DeepWater,
        Self::Ocean,
        Self::ShallowWater,
        Self::Beach,
        Self::Grass,
        Self::Forest,
        Self::Hills,
        Self::Mountain,
        Self::Snow,
    ];

    /// Это вода?
    pub fn is_water(self) -> bool {
        matches!(self, Self::DeepWater | Self::Ocean | Self::ShallowWater)
    }

    /// Это суша?
    pub fn is_land(self) -> bool {
        !self.is_water()
    }

    /// Можно ли пройти по тайлу пешком.
    pub fn is_walkable(self) -> bool {
        matches!(
            self,
            Self::Beach | Self::Grass | Self::Forest | Self::Hills
        )
    }

    /// Лес — источник дерева и еды.
    pub fn is_forest(self) -> bool {
        self == Self::Forest
    }

    /// Может ли здесь появиться дом.
    pub fn is_buildable(self) -> bool {
        matches!(self, Self::Beach | Self::Grass)
    }

    /// Название для интерфейса.
    pub fn name_ru(self) -> &'static str {
        match self {
            Self::DeepWater => "Глубина",
            Self::Ocean => "Океан",
            Self::ShallowWater => "Мелководье",
            Self::Beach => "Пляж",
            Self::Grass => "Луг",
            Self::Forest => "Лес",
            Self::Hills => "Холмы",
            Self::Mountain => "Горы",
            Self::Snow => "Снега",
        }
    }

    /// Числовой код для сериализации.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Обратное преобразование кода.
    pub fn from_u8(value: u8) -> Option<Self> {
        Self::ALL.get(value as usize).copied()
    }
}

/// Вычисляет тип рельефа по высоте и влажности.
///
/// * `elevation` — 0..1000, уровень моря [`SEA_LEVEL`];
/// * `moisture` — 0..255, высокая влажность превращает луг в лес.
pub fn terrain_at(elevation: u16, moisture: u8) -> Terrain {
    match elevation {
        0..=120 => Terrain::DeepWater,
        121..=230 => Terrain::Ocean,
        231..=319 => Terrain::ShallowWater,
        320..=364 => Terrain::Beach,
        365..=619 => {
            if moisture > 165 {
                Terrain::Forest
            } else {
                Terrain::Grass
            }
        }
        620..=759 => Terrain::Hills,
        760..=899 => Terrain::Mountain,
        _ => Terrain::Snow,
    }
}

/// Высота по умолчанию для каждого типа рельефа.
pub fn default_elevation(terrain: Terrain) -> u16 {
    match terrain {
        Terrain::DeepWater => 90,
        Terrain::Ocean => 180,
        Terrain::ShallowWater => 280,
        Terrain::Beach => SEA_LEVEL + 20,
        Terrain::Grass => SEA_LEVEL + 120,
        Terrain::Forest => SEA_LEVEL + 160,
        Terrain::Hills => 680,
        Terrain::Mountain => 820,
        Terrain::Snow => 950,
    }
}

/// Влажность по умолчанию для каждого типа рельефа.
pub fn default_moisture(terrain: Terrain) -> u8 {
    match terrain {
        Terrain::Forest => 220,
        Terrain::Grass => 60,
        Terrain::Beach => 10,
        Terrain::Hills => 90,
        Terrain::Mountain | Terrain::Snow => 20,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elevation_thresholds() {
        assert_eq!(terrain_at(0, 0), Terrain::DeepWater);
        assert_eq!(terrain_at(200, 0), Terrain::Ocean);
        assert_eq!(terrain_at(300, 0), Terrain::ShallowWater);
        assert_eq!(terrain_at(SEA_LEVEL, 0), Terrain::Beach);
        assert_eq!(terrain_at(450, 0), Terrain::Grass);
        assert_eq!(terrain_at(450, 255), Terrain::Forest);
        assert_eq!(terrain_at(700, 0), Terrain::Hills);
        assert_eq!(terrain_at(800, 0), Terrain::Mountain);
        assert_eq!(terrain_at(1000, 0), Terrain::Snow);
    }

    #[test]
    fn defaults_roundtrip() {
        for terrain in Terrain::ALL {
            let restored = terrain_at(default_elevation(terrain), default_moisture(terrain));
            assert_eq!(restored, terrain, "{terrain:?} не восстановился");
        }
    }

    #[test]
    fn codes_roundtrip() {
        for terrain in Terrain::ALL {
            assert_eq!(Terrain::from_u8(terrain.as_u8()), Some(terrain));
        }
        assert_eq!(Terrain::from_u8(200), None);
    }

    #[test]
    fn walkability() {
        assert!(!Terrain::DeepWater.is_walkable());
        assert!(Terrain::Grass.is_walkable());
        assert!(Terrain::Forest.is_walkable());
        assert!(!Terrain::Mountain.is_walkable());
        assert!(!Terrain::Snow.is_walkable());
        assert!(Terrain::Ocean.is_water());
        assert!(Terrain::Snow.is_land());
    }
}
