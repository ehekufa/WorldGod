//! Страны: имя, цвет, столица, счётчики и дипломатия.

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

/// Отношения между двумя странами.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Relation {
    /// Нейтралитет: чужих не трогаем, но и не защищаем.
    #[default]
    Peace = 0,
    /// Союз: воины помогают защищать жителей союзника от монстров.
    Alliance = 1,
    /// Война: воины идут на чужую землю, дома и лодки можно ломать.
    War = 2,
}

impl Relation {
    /// Все состояния в порядке панели.
    pub const ALL: [Relation; 3] = [Self::Peace, Self::Alliance, Self::War];

    /// Подпись для интерфейса.
    pub fn label_ru(self) -> &'static str {
        match self {
            Self::Peace => "Мир",
            Self::Alliance => "Союз",
            Self::War => "Война",
        }
    }

    /// Код для сохранений.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Восстановление из кода.
    pub fn from_u8(value: u8) -> Option<Self> {
        Self::ALL.get(value as usize).copied()
    }

    /// Следующее состояние по кругу: мир → война → союз → мир.
    pub fn next(self) -> Self {
        match self {
            Self::Peace => Self::War,
            Self::War => Self::Alliance,
            Self::Alliance => Self::Peace,
        }
    }
}

/// Таблица дипломатии: по одному статусу на каждую пару стран.
///
/// Хранится квадратной матрицей `n × n`, где `n` — число когда-либо
/// основанных стран; идентификаторы стран начинаются с единицы, поэтому
/// индекс пары — `(a - 1) * n + (b - 1)`. Матрица растёт по мере появления
/// новых стран, прежние договорённости при этом сохраняются.
#[derive(Clone, Debug, Default)]
pub struct Diplomacy {
    size: usize,
    cells: Vec<Relation>,
}

impl Diplomacy {
    /// Пустая таблица: стран ещё нет.
    pub fn new() -> Self {
        Self::default()
    }

    /// Сколько стран влезает в таблицу.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Таблица пуста?
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Расширяет таблицу под `nations` стран, сохраняя старые записи.
    pub fn ensure(&mut self, nations: usize) {
        if nations <= self.size {
            return;
        }
        let size = nations;
        let mut cells = vec![Relation::Peace; size * size];
        for a in 0..self.size {
            for b in 0..self.size {
                cells[a * size + b] = self.cells[a * self.size + b];
            }
        }
        self.size = size;
        self.cells = cells;
    }

    fn index(&self, a: NationId, b: NationId) -> Option<usize> {
        if a == 0 || b == 0 {
            return None;
        }
        let (a, b) = (a as usize - 1, b as usize - 1);
        if a >= self.size || b >= self.size {
            return None;
        }
        Some(a * self.size + b)
    }

    /// Статус пары стран (симметрично; страна сама с собой — [`Relation::Peace`]).
    pub fn get(&self, a: NationId, b: NationId) -> Relation {
        if a == b {
            return Relation::Peace;
        }
        self.index(a, b)
            .and_then(|index| self.cells.get(index).copied())
            .unwrap_or(Relation::Peace)
    }

    /// Записывает статус пары в обе стороны.
    pub fn set(&mut self, a: NationId, b: NationId, relation: Relation) {
        if a == b {
            return;
        }
        let top = a.max(b) as usize;
        self.ensure(top);
        if let (Some(ab), Some(ba)) = (self.index(a, b), self.index(b, a)) {
            self.cells[ab] = relation;
            self.cells[ba] = relation;
        }
    }

    /// Между странами война?
    pub fn at_war(&self, a: NationId, b: NationId) -> bool {
        self.get(a, b) == Relation::War
    }

    /// Страны в союзе?
    pub fn allied(&self, a: NationId, b: NationId) -> bool {
        self.get(a, b) == Relation::Alliance
    }

    /// Все пары в заданном статусе (`a < b`, каждая пара один раз).
    pub fn pairs(&self, relation: Relation) -> Vec<(NationId, NationId)> {
        let mut pairs = Vec::new();
        for a in 1..=self.size as NationId {
            for b in (a + 1)..=self.size as NationId {
                if self.get(a, b) == relation {
                    pairs.push((a, b));
                }
            }
        }
        pairs
    }

    /// Сколько сейчас войн.
    pub fn war_count(&self) -> u32 {
        self.pairs(Relation::War).len() as u32
    }

    /// Сколько сейчас союзов.
    pub fn alliance_count(&self) -> u32 {
        self.pairs(Relation::Alliance).len() as u32
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

    #[test]
    fn diplomacy_starts_as_peace() {
        let mut table = Diplomacy::new();
        assert!(table.is_empty());
        table.ensure(3);
        assert_eq!(table.size(), 3);
        assert_eq!(table.get(1, 2), Relation::Peace);
        assert_eq!(table.get(3, 1), Relation::Peace);
        assert_eq!(table.get(2, 2), Relation::Peace);
        assert_eq!(table.war_count(), 0);
    }

    #[test]
    fn diplomacy_is_symmetric_and_keeps_history() {
        let mut table = Diplomacy::new();
        table.ensure(2);
        table.set(1, 2, Relation::War);
        assert!(table.at_war(1, 2));
        assert!(table.at_war(2, 1));
        assert_eq!(table.war_count(), 1);

        // Новый мир с третьей страной не стирает старую войну.
        table.ensure(3);
        assert!(table.at_war(2, 1));
        table.set(2, 3, Relation::Alliance);
        assert!(table.allied(3, 2));
        assert_eq!(table.alliance_count(), 1);
        assert_eq!(table.pairs(Relation::War), vec![(1, 2)]);

        // Новая страна входит в мир со всеми.
        assert_eq!(table.get(1, 3), Relation::Peace);
    }

    #[test]
    fn relation_cycles_and_labels() {
        for relation in Relation::ALL {
            assert_eq!(Relation::from_u8(relation.as_u8()), Some(relation));
            assert!(!relation.label_ru().is_empty());
        }
        assert_eq!(Relation::from_u8(9), None);
        assert_eq!(Relation::Peace.next(), Relation::War);
        assert_eq!(Relation::War.next(), Relation::Alliance);
        assert_eq!(Relation::Alliance.next(), Relation::Peace);
    }
}
