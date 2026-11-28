//! RGBA-кадр и базовые операции рисования.

use std::fmt;
use std::path::Path;

use crate::png;

/// Кадр RGBA8: 4 байта на пиксель, строки идут сверху вниз.
#[derive(Clone)]
pub struct Frame {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Frame {
    /// Пустой (чёрный, непрозрачный) кадр.
    pub fn new(width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        Self {
            width,
            height,
            pixels: vec![0; (width as usize) * (height as usize) * 4],
        }
    }

    /// Кадр, заполненный цветом.
    pub fn filled(width: u32, height: u32, color: [u8; 3]) -> Self {
        let mut frame = Self::new(width, height);
        frame.clear(color);
        frame
    }

    /// Ширина в пикселях.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Высота в пикселях.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Размер `(ширина, высота)`.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Сырые RGBA-данные (для текстур GPU).
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Изменяемые сырые RGBA-данные.
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    /// Заливает весь кадр цветом.
    pub fn clear(&mut self, color: [u8; 3]) {
        for pixel in self.pixels.chunks_exact_mut(4) {
            pixel[0] = color[0];
            pixel[1] = color[1];
            pixel[2] = color[2];
            pixel[3] = 255;
        }
    }

    /// Цвет пикселя.
    pub fn get(&self, x: i32, y: i32) -> Option<[u8; 3]> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        let index = ((y as usize) * (self.width as usize) + x as usize) * 4;
        Some([
            self.pixels[index],
            self.pixels[index + 1],
            self.pixels[index + 2],
        ])
    }

    /// Ставит непрозрачный пиксель (за границей кадра ничего не делает).
    pub fn put(&mut self, x: i32, y: i32, color: [u8; 3]) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let index = ((y as usize) * (self.width as usize) + x as usize) * 4;
        self.pixels[index] = color[0];
        self.pixels[index + 1] = color[1];
        self.pixels[index + 2] = color[2];
        self.pixels[index + 3] = 255;
    }

    /// Смешивает цвет с пикселем (alpha 0..1).
    pub fn blend(&mut self, x: i32, y: i32, color: [u8; 3], alpha: f32) {
        let Some(under) = self.get(x, y) else {
            return;
        };
        let alpha = alpha.clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }
        let mixed = [
            (under[0] as f32 + (color[0] as f32 - under[0] as f32) * alpha) as u8,
            (under[1] as f32 + (color[1] as f32 - under[1] as f32) * alpha) as u8,
            (under[2] as f32 + (color[2] as f32 - under[2] as f32) * alpha) as u8,
        ];
        self.put(x, y, mixed);
    }

    /// Заливает прямоугольник.
    pub fn fill_rect(&mut self, x: i32, y: i32, width: u32, height: u32, color: [u8; 3]) {
        for row in 0..height as i32 {
            for col in 0..width as i32 {
                self.put(x + col, y + row, color);
            }
        }
    }

    /// Смешивает прямоугольник с фоном.
    pub fn blend_rect(
        &mut self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        color: [u8; 3],
        alpha: f32,
    ) {
        for row in 0..height as i32 {
            for col in 0..width as i32 {
                self.blend(x + col, y + row, color, alpha);
            }
        }
    }

    /// Горизонтальная линия (включительно).
    pub fn hline(&mut self, x0: i32, x1: i32, y: i32, color: [u8; 3]) {
        let (from, to) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        for x in from..=to {
            self.put(x, y, color);
        }
    }

    /// Вертикальная линия (включительно).
    pub fn vline(&mut self, x: i32, y0: i32, y1: i32, color: [u8; 3]) {
        let (from, to) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        for y in from..=to {
            self.put(x, y, color);
        }
    }

    /// Рамка толщиной 1 пиксель.
    pub fn outline_rect(&mut self, x: i32, y: i32, width: u32, height: u32, color: [u8; 3]) {
        if width == 0 || height == 0 {
            return;
        }
        let right = x + width as i32 - 1;
        let bottom = y + height as i32 - 1;
        self.hline(x, right, y, color);
        self.hline(x, right, bottom, color);
        self.vline(x, y, bottom, color);
        self.vline(right, y, bottom, color);
    }

    /// Увеличивает кадр в целое число раз (пиксель в квадрат).
    pub fn scaled(&self, factor: u32) -> Frame {
        let factor = factor.max(1);
        let mut out = Frame::new(self.width * factor, self.height * factor);
        for y in 0..self.height {
            for x in 0..self.width {
                let Some(color) = self.get(x as i32, y as i32) else {
                    continue;
                };
                out.fill_rect(
                    (x * factor) as i32,
                    (y * factor) as i32,
                    factor,
                    factor,
                    color,
                );
            }
        }
        out
    }

    /// Кодирует кадр в PNG.
    pub fn to_png(&self) -> Vec<u8> {
        png::encode_rgba(self.width, self.height, &self.pixels)
    }

    /// Сохраняет кадр в PNG-файл.
    pub fn save_png(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        std::fs::write(path, self.to_png())
    }
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bytes", &self.pixels.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_and_get() {
        let mut frame = Frame::new(4, 3);
        frame.put(1, 1, [10, 20, 30]);
        assert_eq!(frame.get(1, 1), Some([10, 20, 30]));
        assert_eq!(frame.get(9, 9), None);
        frame.put(-5, -5, [255, 255, 255]); // не паникует
    }

    #[test]
    fn rects_and_lines() {
        let mut frame = Frame::new(8, 8);
        frame.clear([0, 0, 0]);
        frame.fill_rect(1, 1, 3, 2, [255, 0, 0]);
        assert_eq!(frame.get(3, 2), Some([255, 0, 0]));
        assert_eq!(frame.get(4, 2), Some([0, 0, 0]));

        frame.outline_rect(0, 0, 8, 8, [1, 2, 3]);
        assert_eq!(frame.get(0, 4), Some([1, 2, 3]));
        assert_eq!(frame.get(7, 7), Some([1, 2, 3]));
        assert_eq!(frame.get(4, 4), Some([0, 0, 0]));
    }

    #[test]
    fn blending_mixes_colors() {
        let mut frame = Frame::filled(2, 2, [0, 0, 0]);
        frame.blend(0, 0, [255, 255, 255], 0.5);
        assert_eq!(frame.get(0, 0), Some([127, 127, 127]));
        frame.blend(0, 1, [255, 255, 255], 0.0);
        assert_eq!(frame.get(0, 1), Some([0, 0, 0]));
    }

    #[test]
    fn scaling_duplicates_pixels() {
        let mut frame = Frame::new(2, 2);
        frame.put(0, 0, [9, 9, 9]);
        let scaled = frame.scaled(2);
        assert_eq!(scaled.size(), (4, 4));
        assert_eq!(scaled.get(1, 1), Some([9, 9, 9]));
        assert_eq!(scaled.get(2, 2), Some([0, 0, 0]));
    }

    #[test]
    fn png_roundtrip_size() {
        let frame = Frame::filled(3, 3, [12, 34, 56]);
        let png = frame.to_png();
        assert!(png.len() > 60);
        assert_eq!(&png[1..4], b"PNG");
    }
}
