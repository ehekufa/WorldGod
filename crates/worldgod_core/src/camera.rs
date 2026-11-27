//! Камера: пересчёт мировых координат в экранные и обратно.
//!
//! Камера не зависит от GUI-библиотеки: интерфейс просто передаёт ей размер
//! области карты в точках и получает готовые преобразования.

use crate::grid::TilePos;
use crate::math::Vec2;

/// Преобразование «тайлы ↔ точки экрана».
#[derive(Clone, Copy, Debug)]
pub struct ViewTransform {
    /// Левый верхний угол области карты на экране.
    pub origin: Vec2,
    /// Размер области карты на экране.
    pub size: Vec2,
    /// Мировая точка (в тайлах), которая находится в центре области.
    pub center: Vec2,
    /// Сколько экранных точек приходится на один тайл.
    pub scale: f32,
}

impl ViewTransform {
    /// Максимальный зум: один тайл — 64 точки.
    pub const MAX_SCALE: f32 = 64.0;
    /// Насколько можно отдалиться от «мир целиком в экране».
    pub const ZOOM_OUT_LIMIT: f32 = 0.55;

    /// Новая камера, показывающая мир целиком.
    pub fn new(world: (u32, u32), origin: Vec2, size: Vec2) -> Self {
        let mut camera = Self {
            origin,
            size,
            center: Vec2::new(world.0 as f32 * 0.5, world.1 as f32 * 0.5),
            scale: 8.0,
        };
        camera.fit(world);
        camera
    }

    /// Обновляет положение и размер области карты на экране.
    pub fn set_viewport(&mut self, origin: Vec2, size: Vec2) {
        self.origin = origin;
        self.size = Vec2::new(size.x.max(1.0), size.y.max(1.0));
    }

    /// Вписывает весь мир в область карты.
    pub fn fit(&mut self, world: (u32, u32)) {
        self.center = Vec2::new(world.0 as f32 * 0.5, world.1 as f32 * 0.5);
        self.scale = self.fit_scale(world) * 0.95;
    }

    /// Масштаб, при котором мир виден целиком.
    pub fn fit_scale(&self, world: (u32, u32)) -> f32 {
        let sx = self.size.x / world.0.max(1) as f32;
        let sy = self.size.y / world.1.max(1) as f32;
        sx.min(sy).max(0.05)
    }

    /// Минимально допустимый масштаб.
    pub fn min_scale(&self, world: (u32, u32)) -> f32 {
        (self.fit_scale(world) * Self::ZOOM_OUT_LIMIT).max(0.02)
    }

    /// Верхний левый угол мира в экранных координатах.
    pub fn world_origin(&self) -> Vec2 {
        self.origin + self.size * 0.5 - self.center * self.scale
    }

    /// Размер мира на экране в точках.
    pub fn world_screen_size(&self, world: (u32, u32)) -> Vec2 {
        Vec2::new(world.0 as f32, world.1 as f32) * self.scale
    }

    /// Переводит мировую точку (в тайлах, дробную) в экранную.
    pub fn tile_to_screen(&self, p: Vec2) -> Vec2 {
        self.world_origin() + p * self.scale
    }

    /// Переводит экранную точку в мировую (в тайлах, дробную).
    pub fn screen_to_tile(&self, p: Vec2) -> Vec2 {
        (p - self.world_origin()) / self.scale
    }

    /// Тайл под экранной точкой.
    pub fn tile_at(&self, p: Vec2) -> TilePos {
        let tile = self.screen_to_tile(p);
        TilePos::new(tile.x.floor() as i32, tile.y.floor() as i32)
    }

    /// Центр тайла на экране.
    pub fn tile_center_screen(&self, p: TilePos) -> Vec2 {
        self.tile_to_screen(Vec2::new(p.x as f32 + 0.5, p.y as f32 + 0.5))
    }

    /// Сдвиг камеры пальцем/мышью (в экранных точках).
    pub fn pan_pixels(&mut self, delta: Vec2, world: (u32, u32)) {
        if self.scale > f32::EPSILON {
            self.center -= delta / self.scale;
        }
        self.clamp_center(world);
    }

    /// Зум вокруг экранной точки: тайл под курсором остаётся на месте.
    pub fn zoom_at(&mut self, anchor: Vec2, factor: f32, world: (u32, u32)) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let before = self.screen_to_tile(anchor);
        self.scale = (self.scale * factor).clamp(self.min_scale(world), Self::MAX_SCALE);
        let after = self.screen_to_tile(anchor);
        self.center += before - after;
        self.clamp_center(world);
    }

    /// Не даёт камере уехать за пределы карты.
    pub fn clamp_center(&mut self, world: (u32, u32)) {
        let w = world.0 as f32;
        let h = world.1 as f32;
        self.center.x = self.center.x.clamp(0.0, w);
        self.center.y = self.center.y.clamp(0.0, h);
    }

    /// Видимый прямоугольник тайлов (включительно).
    pub fn visible_tiles(&self, world: (u32, u32)) -> (TilePos, TilePos) {
        let min = self.screen_to_tile(self.origin);
        let max = self.screen_to_tile(self.origin + self.size);
        let min_x = (min.x.floor() as i32).clamp(0, world.0 as i32 - 1);
        let min_y = (min.y.floor() as i32).clamp(0, world.1 as i32 - 1);
        let max_x = (max.x.ceil() as i32).clamp(0, world.0 as i32 - 1);
        let max_y = (max.y.ceil() as i32).clamp(0, world.1 as i32 - 1);
        (TilePos::new(min_x, min_y), TilePos::new(max_x, max_y))
    }
}

impl Default for ViewTransform {
    fn default() -> Self {
        Self {
            origin: Vec2::ZERO,
            size: Vec2::new(800.0, 600.0),
            center: Vec2::new(128.0, 96.0),
            scale: 4.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORLD: (u32, u32) = (256, 192);

    fn camera() -> ViewTransform {
        ViewTransform::new(WORLD, Vec2::new(0.0, 100.0), Vec2::new(400.0, 300.0))
    }

    #[test]
    fn world_fits_on_screen() {
        let camera = camera();
        assert_eq!(camera.center, Vec2::new(128.0, 96.0));
        assert!(camera.scale > 0.0);
        let screen = camera.world_screen_size(WORLD);
        assert!(screen.x <= camera.size.x + 0.01);
        assert!(screen.y <= camera.size.y + 0.01);
    }

    #[test]
    fn screen_roundtrip() {
        let camera = camera();
        let tile = Vec2::new(42.5, 17.25);
        let screen = camera.tile_to_screen(tile);
        let back = camera.screen_to_tile(screen);
        assert!((back.x - tile.x).abs() < 1e-4);
        assert!((back.y - tile.y).abs() < 1e-4);
    }

    #[test]
    fn tile_at_works() {
        let camera = camera();
        let screen = camera.tile_center_screen(TilePos::new(10, 20));
        assert_eq!(camera.tile_at(screen), TilePos::new(10, 20));
    }

    #[test]
    fn zoom_keeps_anchor() {
        let mut camera = camera();
        let anchor = Vec2::new(120.0, 180.0);
        let before = camera.screen_to_tile(anchor);
        camera.zoom_at(anchor, 2.5, WORLD);
        let after = camera.screen_to_tile(anchor);
        assert!((before.x - after.x).abs() < 1e-3, "{before:?} != {after:?}");
        assert!((before.y - after.y).abs() < 1e-3, "{before:?} != {after:?}");
        assert!(camera.scale > 0.0);
    }

    #[test]
    fn zoom_is_limited() {
        let mut camera = camera();
        for _ in 0..50 {
            camera.zoom_at(Vec2::new(200.0, 200.0), 2.0, WORLD);
        }
        assert!(camera.scale <= ViewTransform::MAX_SCALE + 0.001);
        for _ in 0..50 {
            camera.zoom_at(Vec2::new(200.0, 200.0), 0.5, WORLD);
        }
        assert!(camera.scale >= camera.min_scale(WORLD) - 0.001);
    }

    #[test]
    fn pan_moves_view_and_stays_in_bounds() {
        let mut camera = camera();
        let center_before = camera.center;
        camera.pan_pixels(Vec2::new(-100.0, 0.0), WORLD);
        assert!(camera.center.x > center_before.x);

        camera.pan_pixels(Vec2::new(100_000.0, 0.0), WORLD);
        assert_eq!(camera.center.x, 0.0);
        camera.pan_pixels(Vec2::new(-100_000.0, 0.0), WORLD);
        assert_eq!(camera.center.x, WORLD.0 as f32);
    }

    #[test]
    fn visible_tiles_are_inside_world() {
        let camera = camera();
        let (min, max) = camera.visible_tiles(WORLD);
        assert!(min.x >= 0 && min.y >= 0);
        assert!(max.x < WORLD.0 as i32 && max.y < WORLD.1 as i32);
        assert!(max.x > min.x && max.y > min.y);
    }

    #[test]
    fn degenerate_viewport_is_safe() {
        let mut camera = camera();
        camera.set_viewport(Vec2::ZERO, Vec2::ZERO);
        assert!(camera.size.x >= 1.0);
        camera.zoom_at(Vec2::ZERO, 0.0, WORLD); // не должно паниковать
        camera.zoom_at(Vec2::ZERO, f32::NAN, WORLD);
    }
}
