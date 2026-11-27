//! Целочисленные координаты тайлов и обход соседей.

use std::ops::{Add, AddAssign, Sub, SubAssign};

/// Позиция тайла на карте. Ось `y` растёт вниз, как в экранных координатах.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
}

/// Короткий конструктор: `pos(3, 7)`.
pub const fn pos(x: i32, y: i32) -> TilePos {
    TilePos { x, y }
}

/// Смещения четырёх ортогональных соседей.
pub const NEIGHBORS4: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// Смещения восьми соседей (включая диагонали).
pub const NEIGHBORS8: [(i32, i32); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

impl TilePos {
    /// Начало координат.
    pub const ZERO: Self = Self { x: 0, y: 0 };

    /// Новый тайл.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Расстояние Чебышёва — «сколько шагов» по сетке, включая диагонали.
    pub fn chebyshev(self, other: Self) -> i32 {
        (self.x - other.x).abs().max((self.y - other.y).abs())
    }

    /// Манхэттенское расстояние.
    pub fn manhattan(self, other: Self) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    /// Квадрат евклидова расстояния (без `sqrt`).
    pub fn euclid2(self, other: Self) -> i32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }

    /// Евклидово расстояние.
    pub fn distance(self, other: Self) -> f32 {
        (self.euclid2(other) as f32).sqrt()
    }

    /// Смещение на вектор.
    pub fn offset(self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    /// Один шаг в сторону цели (по обеим осям одновременно).
    pub fn step_towards(self, target: Self) -> Self {
        Self {
            x: self.x + (target.x - self.x).signum(),
            y: self.y + (target.y - self.y).signum(),
        }
    }

    /// Итератор по четырём соседям.
    pub fn neighbors4(self) -> impl Iterator<Item = TilePos> {
        NEIGHBORS4
            .iter()
            .map(move |&(dx, dy)| self.offset(dx, dy))
    }

    /// Итератор по восьми соседям.
    pub fn neighbors8(self) -> impl Iterator<Item = TilePos> {
        NEIGHBORS8
            .iter()
            .map(move |&(dx, dy)| self.offset(dx, dy))
    }

    /// Пара чисел как `f32` — удобно для камеры и рендера.
    pub fn to_vec2(self) -> crate::math::Vec2 {
        crate::math::Vec2::new(self.x as f32, self.y as f32)
    }
}

impl Add for TilePos {
    type Output = TilePos;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl AddAssign for TilePos {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for TilePos {
    type Output = TilePos;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl SubAssign for TilePos {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

/// Обходит квадрат вокруг `center` и вызывает `f` для тайлов внутри круга.
///
/// Второй аргумент — нормированное расстояние: `0.0` в центре, `1.0` на краю
/// кисти. Это позволяет делать мягкие, «органические» мазки.
pub fn for_each_in_radius(center: TilePos, radius: i32, mut f: impl FnMut(TilePos, f32)) {
    let radius = radius.max(0);
    let radius_f = radius.max(1) as f32;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let dist = ((dx * dx + dy * dy) as f32).sqrt() / radius_f;
            if dist <= 1.0 {
                f(center.offset(dx, dy), dist);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances() {
        let a = pos(2, 3);
        let b = pos(5, 7);
        assert_eq!(a.chebyshev(b), 4);
        assert_eq!(a.manhattan(b), 7);
        assert_eq!(a.euclid2(b), 25);
        assert_eq!(a.distance(b), 5.0);
    }

    #[test]
    fn stepping() {
        assert_eq!(pos(0, 0).step_towards(pos(5, -5)), pos(1, -1));
        assert_eq!(pos(4, 4).step_towards(pos(0, 4)), pos(3, 4));
        assert_eq!(pos(2, 2).step_towards(pos(2, 2)), pos(2, 2));
    }

    #[test]
    fn radius_covers_circle() {
        let mut tiles = Vec::new();
        for_each_in_radius(pos(10, 10), 3, |p, d| tiles.push((p, d)));
        assert!(tiles.len() > 20);
        assert!(tiles.iter().all(|(p, d)| pos(10, 10).chebyshev(*p) <= 3 && *d <= 1.0));
        assert!(tiles.iter().any(|(p, d)| *p == pos(10, 10) && *d == 0.0));
    }
}
