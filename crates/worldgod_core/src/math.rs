//! Мелкая математика: скалярные помощники, цветовые смеси и тип [`Vec2`].

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// Ограничивает значение диапазоном `0.0..=1.0`.
pub fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

/// Линейная интерполяция.
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Обратная интерполяция: где находится `v` между `a` и `b`.
pub fn inv_lerp(a: f32, b: f32, v: f32) -> f32 {
    if (b - a).abs() < f32::EPSILON {
        0.0
    } else {
        (v - a) / (b - a)
    }
}

/// Плавная S-кривая на отрезке `0..=1`.
pub fn smoothstep(t: f32) -> f32 {
    let t = clamp01(t);
    t * t * (3.0 - 2.0 * t)
}

/// Плавное «приближение» к единице: быстро в начале, медленно в конце.
pub fn ease_out(t: f32) -> f32 {
    let t = clamp01(t);
    1.0 - (1.0 - t) * (1.0 - t)
}

/// Интерполяция целых чисел.
pub fn lerp_i32(a: i32, b: i32, t: f32) -> i32 {
    (lerp(a as f32, b as f32, t)).round() as i32
}

/// Смешивает два RGB-цвета.
pub fn mix_rgb(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = clamp01(t);
    [
        lerp(a[0] as f32, b[0] as f32, t).round() as u8,
        lerp(a[1] as f32, b[1] as f32, t).round() as u8,
        lerp(a[2] as f32, b[2] as f32, t).round() as u8,
    ]
}

/// Умножает цвет на коэффициент (яркость).
pub fn brightness(c: [u8; 3], factor: f32) -> [u8; 3] {
    [
        (c[0] as f32 * factor).clamp(0.0, 255.0) as u8,
        (c[1] as f32 * factor).clamp(0.0, 255.0) as u8,
        (c[2] as f32 * factor).clamp(0.0, 255.0) as u8,
    ]
}

/// Двумерный вектор с плавающей точкой (экранные и мировые координаты).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    /// Нулевой вектор.
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    /// Вектор из двух чисел.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Одинаковые компоненты.
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v }
    }

    /// Длина вектора.
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// Квадрат длины.
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Скалярное произведение.
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// Единичный вектор; для нулевого возвращает [`Vec2::ZERO`].
    pub fn normalized(self) -> Self {
        let len = self.length();
        if len <= f32::EPSILON {
            Self::ZERO
        } else {
            self / len
        }
    }

    /// Расстояние до другой точки.
    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }

    /// Округление компонент.
    pub fn round(self) -> Self {
        Self::new(self.x.round(), self.y.round())
    }

    /// Покомпонентное ограничение.
    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self::new(self.x.clamp(min.x, max.x), self.y.clamp(min.y, max.y))
    }

    /// Линейная интерполяция между точками.
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self::new(lerp(self.x, other.x, t), lerp(self.y, other.y, t))
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl Mul<Vec2> for f32 {
    type Output = Vec2;

    fn mul(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self * rhs.x, self * rhs.y)
    }
}

impl MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl DivAssign<f32> for Vec2 {
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl Neg for Vec2 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_helpers() {
        assert_eq!(lerp(0.0, 10.0, 0.25), 2.5);
        assert_eq!(clamp01(-3.0), 0.0);
        assert_eq!(clamp01(3.0), 1.0);
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-6);
        assert!(ease_out(0.0).abs() < 1e-6);
        assert!((ease_out(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn colors() {
        assert_eq!(mix_rgb([0, 0, 0], [255, 255, 255], 0.5), [128, 128, 128]);
        assert_eq!(brightness([100, 100, 100], 2.0), [200, 200, 200]);
        assert_eq!(brightness([200, 200, 200], 2.0), [255, 255, 255]);
    }

    #[test]
    fn vectors() {
        let a = Vec2::new(3.0, 4.0);
        assert_eq!(a.length(), 5.0);
        assert_eq!(a.normalized().length(), 1.0);
        assert_eq!((a * 2.0).x, 6.0);
        assert_eq!((2.0 * a).y, 8.0);
        assert_eq!((a / 2.0).x, 1.5);
        assert_eq!(Vec2::ZERO.normalized(), Vec2::ZERO);
        assert_eq!(a.lerp(Vec2::new(5.0, 8.0), 0.5), Vec2::new(4.0, 6.0));
    }
}
