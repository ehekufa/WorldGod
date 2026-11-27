//! Детерминированный генератор случайных чисел (SplitMix64).
//!
//! Никаких зависимостей и глобального состояния: один [`Rng`] живёт внутри
//! [`crate::Simulation`], поэтому любой мир воспроизводится по своему seed.

/// Компактный генератор на 64-битном состоянии.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Создаёт генератор с заданным зерном.
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
        }
    }

    /// Текущее внутреннее состояние (полезно для отладки и сохранений).
    pub fn state(&self) -> u64 {
        self.state
    }

    /// Восстанавливает генератор из сохранённого состояния.
    pub fn from_state(state: u64) -> Self {
        Self { state: state | 1 }
    }

    /// Следующее 64-битное число.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Следующее 32-битное число.
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Число в диапазоне `[0.0, 1.0)`.
    pub fn next_f32(&mut self) -> f32 {
        // 24 старших бита дают ровно 2^24 возможных значений — без потери точности.
        ((self.next_u64() >> 40) as f32) / 16_777_216.0
    }

    /// Число в диапазоне `[lo, hi)`. Если `hi <= lo`, возвращает `lo`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            lo
        } else {
            let span = (hi - lo) as u32;
            lo + (self.next_u32() % span) as i32
        }
    }

    /// Число в диапазоне `[lo, hi]`.
    pub fn range_inclusive(&mut self, lo: i32, hi: i32) -> i32 {
        self.range(lo, hi.saturating_add(1))
    }

    /// Дробное число в диапазоне `[lo, hi)`.
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }

    /// `true` с вероятностью `p`.
    pub fn chance(&mut self, p: f32) -> bool {
        self.next_f32() < p
    }

    /// Случайный элемент непустого среза.
    ///
    /// # Panics
    ///
    /// Паникует, если срез пуст. Для пустых срезов используйте [`Rng::choose`].
    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        assert!(!items.is_empty(), "Rng::pick: пустой срез");
        items[(self.next_u32() as usize) % items.len()]
    }

    /// Случайный элемент или `None` для пустого среза.
    pub fn choose<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            Some(&items[(self.next_u32() as usize) % items.len()])
        }
    }

    /// Случайный знак: `-1` или `1`.
    pub fn sign(&mut self) -> i32 {
        if self.chance(0.5) {
            1
        } else {
            -1
        }
    }

    /// Случайный тайл в квадрате вокруг `center`.
    pub fn tile_in_square(&mut self, center: crate::grid::TilePos, radius: i32) -> crate::grid::TilePos {
        let r = radius.max(0);
        center.offset(self.range(-r, r + 1), self.range(-r, r + 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..64 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_differ() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn ranges_are_respected() {
        let mut rng = Rng::new(99);
        for _ in 0..10_000 {
            let v = rng.range(-3, 5);
            assert!((-3..5).contains(&v));
            let f = rng.next_f32();
            assert!((0.0..1.0).contains(&f));
            let i = rng.range_inclusive(2, 4);
            assert!((2..=4).contains(&i));
        }
    }

    #[test]
    fn chance_extremes() {
        let mut rng = Rng::new(5);
        for _ in 0..256 {
            assert!(!rng.chance(0.0));
            assert!(rng.chance(1.0));
        }
    }
}
