//! Дешёвый value-noise и фрактальный шум для генерации рельефа.

use crate::math::smoothstep;

/// Хэш целочисленной точки в 32-битное значение.
fn hash2(x: i32, y: i32, seed: u64) -> u32 {
    let mut h = seed
        ^ (x as i64 as u64).wrapping_mul(0x9E37_79B1_85EB_CA87)
        ^ (y as i64 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    h ^= h >> 33;
    (h >> 32) as u32
}

/// Значение в узле сетки в диапазоне `[0, 1]`.
fn lattice(x: i32, y: i32, seed: u64) -> f32 {
    (hash2(x, y, seed) as f32) / (u32::MAX as f32)
}

/// Гладкий value-noise в диапазоне `[0, 1]`.
pub fn value_noise(x: f32, y: f32, seed: u64) -> f32 {
    let x0 = x.floor();
    let y0 = y.floor();
    let ix = x0 as i32;
    let iy = y0 as i32;
    let fx = smoothstep(x - x0);
    let fy = smoothstep(y - y0);

    let n00 = lattice(ix, iy, seed);
    let n10 = lattice(ix + 1, iy, seed);
    let n01 = lattice(ix, iy + 1, seed);
    let n11 = lattice(ix + 1, iy + 1, seed);

    let top = n00 + (n10 - n00) * fx;
    let bottom = n01 + (n11 - n01) * fx;
    top + (bottom - top) * fy
}

/// Фрактальный шум: несколько октав value-noise, результат в `[0, 1]`.
pub fn fbm(x: f32, y: f32, seed: u64, octaves: u32) -> f32 {
    let mut sum = 0.0;
    let mut amplitude = 1.0;
    let mut total = 0.0;
    let mut frequency = 1.0;

    for octave in 0..octaves {
        let octave_seed = seed.wrapping_add((octave as u64 + 1).wrapping_mul(0x9E37_79B9));
        sum += value_noise(x * frequency, y * frequency, octave_seed) * amplitude;
        total += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    if total > 0.0 {
        sum / total
    } else {
        0.5
    }
}

/// «Островная» маска: 1 в центре, 0 по краям. Полезно для тестовых карт.
pub fn island_mask(x: f32, y: f32, width: f32, height: f32, falloff: f32) -> f32 {
    let cx = width * 0.5;
    let cy = height * 0.5;
    let nx = (x - cx) / (width * 0.5);
    let ny = (y - cy) / (height * 0.5);
    let d = (nx * nx + ny * ny).sqrt();
    smoothstep(1.0 - (d * falloff).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_in_unit_range() {
        for i in 0..200 {
            let x = i as f32 * 0.37;
            let y = i as f32 * 0.11;
            let v = value_noise(x, y, 1234);
            assert!((0.0..=1.0).contains(&v), "value_noise вне диапазона: {v}");
            let f = fbm(x, y, 1234, 4);
            assert!((0.0..=1.0).contains(&f), "fbm вне диапазона: {f}");
        }
    }

    #[test]
    fn noise_is_smooth() {
        let a = value_noise(3.0, 4.0, 9);
        let b = value_noise(3.0001, 4.0, 9);
        assert!((a - b).abs() < 0.01);
    }

    #[test]
    fn noise_is_deterministic() {
        assert_eq!(value_noise(1.5, 2.5, 77), value_noise(1.5, 2.5, 77));
        assert_ne!(value_noise(1.5, 2.5, 77), value_noise(1.5, 2.5, 78));
    }

    #[test]
    fn island_mask_is_centered() {
        assert!(island_mask(50.0, 50.0, 100.0, 100.0, 1.0) > 0.9);
        assert!(island_mask(0.0, 0.0, 100.0, 100.0, 1.0) < 0.1);
    }
}
