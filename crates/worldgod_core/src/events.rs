//! Журнал событий мира: короткие новости для ленты интерфейса.

use std::collections::VecDeque;

/// Категория события — по ней интерфейс выбирает цвет.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    /// Основание страны.
    Nation,
    /// Постройка дома.
    Build,
    /// Рождение жителя.
    Birth,
    /// Смерть существа.
    Death,
    /// Стычка, охота.
    Combat,
    /// Изменение природы игроком.
    Nature,
}

impl EventKind {
    /// Значок-префикс для ленты.
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Nation => "⚑",
            Self::Build => "⌂",
            Self::Birth => "＋",
            Self::Death => "✖",
            Self::Combat => "⚔",
            Self::Nature => "❖",
        }
    }
}

/// Одно событие мира.
#[derive(Clone, Debug)]
pub struct GameEvent {
    /// Тик, на котором случилось.
    pub tick: u64,
    /// Категория.
    pub kind: EventKind,
    /// Текст для ленты.
    pub text: String,
}

/// Кольцевой журнал с ограниченной длиной.
#[derive(Clone, Debug, Default)]
pub struct EventLog {
    entries: VecDeque<GameEvent>,
}

impl EventLog {
    /// Максимальное число записей.
    pub const CAPACITY: usize = 120;

    /// Добавляет событие, вытесняя самые старые.
    pub fn push(&mut self, tick: u64, kind: EventKind, text: impl Into<String>) {
        self.entries.push_back(GameEvent {
            tick,
            kind,
            text: text.into(),
        });
        while self.entries.len() > Self::CAPACITY {
            self.entries.pop_front();
        }
    }

    /// Последние `count` событий в хронологическом порядке.
    pub fn recent(&self, count: usize) -> impl Iterator<Item = &GameEvent> {
        let skip = self.entries.len().saturating_sub(count);
        self.entries.iter().skip(skip)
    }

    /// Самое свежее событие.
    pub fn last(&self) -> Option<&GameEvent> {
        self.entries.back()
    }

    /// Все события.
    pub fn iter(&self) -> impl Iterator<Item = &GameEvent> {
        self.entries.iter()
    }

    /// Количество событий.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Журнал пуст?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Очищает журнал.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_keeps_last_entries() {
        let mut log = EventLog::default();
        assert!(log.is_empty());
        for i in 0..(EventLog::CAPACITY + 25) {
            log.push(i as u64, EventKind::Nature, format!("событие {i}"));
        }
        assert_eq!(log.len(), EventLog::CAPACITY);
        assert_eq!(log.last().map(|e| e.tick), Some((EventLog::CAPACITY + 24) as u64));
        let recent: Vec<_> = log.recent(3).collect();
        assert_eq!(recent.len(), 3);
        assert_eq!(
            recent.last().map(|e| e.tick),
            Some((EventLog::CAPACITY + 24) as u64)
        );
        log.clear();
        assert!(log.is_empty());
    }

    #[test]
    fn recent_handles_short_log() {
        let mut log = EventLog::default();
        log.push(1, EventKind::Nation, "Велдор основан");
        assert_eq!(log.recent(10).count(), 1);
    }
}
