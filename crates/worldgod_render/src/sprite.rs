//! Пиксельные спрайты существ и построек.
//!
//! Спрайты описаны символьными «артами»: каждая строка — ряд пикселей,
//! каждый символ — цвет из легенды. Такой формат легко читать и править,
//! и он одинаково работает при любом масштабе карты.

use worldgod_core::building::BuildingKind;
use worldgod_core::Species;

use crate::framebuffer::Frame;
use crate::palette;

/// Мини-холст для рисования «арт-пикселями» с целочисленным масштабом.
#[derive(Debug)]
pub struct Painter<'a> {
    frame: &'a mut Frame,
    origin_x: i32,
    origin_y: i32,
    scale: i32,
}

impl<'a> Painter<'a> {
    /// Холст с левым верхним углом в `(origin_x, origin_y)`.
    pub fn new(frame: &'a mut Frame, origin_x: i32, origin_y: i32, scale: i32) -> Self {
        Self {
            frame,
            origin_x,
            origin_y,
            scale: scale.max(1),
        }
    }

    /// Один арт-пиксель.
    pub fn px(&mut self, x: i32, y: i32, color: [u8; 3]) {
        self.frame.fill_rect(
            self.origin_x + x * self.scale,
            self.origin_y + y * self.scale,
            self.scale as u32,
            self.scale as u32,
            color,
        );
    }

    /// Полупрозрачный арт-пиксель.
    pub fn blend_px(&mut self, x: i32, y: i32, color: [u8; 3], alpha: f32) {
        self.frame.blend_rect(
            self.origin_x + x * self.scale,
            self.origin_y + y * self.scale,
            self.scale as u32,
            self.scale as u32,
            color,
            alpha,
        );
    }

    /// Прямоугольник в арт-пикселях.
    pub fn rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: [u8; 3]) {
        self.frame.fill_rect(
            self.origin_x + x * self.scale,
            self.origin_y + y * self.scale,
            (width.max(0) * self.scale) as u32,
            (height.max(0) * self.scale) as u32,
            color,
        );
    }

    /// Рисует арт по строкам, раскрашивая символы функцией `color_of`.
    pub fn art<F>(&mut self, rows: &[&str], mut color_of: F)
    where
        F: FnMut(char) -> Option<[u8; 3]>,
    {
        for (y, row) in rows.iter().enumerate() {
            for (x, symbol) in row.chars().enumerate() {
                if let Some(color) = color_of(symbol) {
                    self.px(x as i32, y as i32, color);
                }
            }
        }
    }
}

/// Размер арта (ширина, высота) в арт-пикселях.
pub fn art_size(rows: &[&str]) -> (i32, i32) {
    let width = rows.iter().map(|row| row.chars().count()).max().unwrap_or(0);
    (width as i32, rows.len() as i32)
}

const HUMAN: [&str; 4] = [".o.", "###", ".#.", "#.#"];
const HUMAN_STEP: [&str; 4] = [".o.", "###", ".#.", ".##"];
const SOLDIER: [&str; 5] = [".s.", "###", "###", ".#.", "#.#"];
const DEER: [&str; 3] = ["l.l.l", "#####", ".#.#."];
const WOLF: [&str; 3] = ["#..#", "####", ".##."];
const ZOMBIE: [&str; 4] = [".o.", "###", ".#.", "#.#"];
const HOUSE: [&str; 5] = [".rrr.", "rrrrr", "wwwww", "wwdww", "wwwww"];
const CAMPFIRE: [&str; 4] = [".f..", "f#f.", ".#w#", "wwww"];
const RUIN: [&str; 3] = ["#.#.", ".###", "#.##"];

/// Рисует существо, стоящее на тайле.
///
/// * `center_x` — центр тайла по горизонтали в пикселях кадра;
/// * `bottom_y` — нижняя граница тайла в пикселях кадра;
/// * `cloth` — цвет одежды (цвет страны);
/// * `moving` — идёт ли существо (меняет кадр анимации).
pub fn draw_entity(
    frame: &mut Frame,
    species: Species,
    center_x: i32,
    bottom_y: i32,
    px_per_tile: i32,
    cloth: [u8; 3],
    variant: u8,
    moving: bool,
) {
    let scale = (px_per_tile / 3).max(1);
    let rows: &[&str] = match species {
        Species::Human => {
            if moving {
                &HUMAN_STEP[..]
            } else {
                &HUMAN[..]
            }
        }
        Species::Soldier => &SOLDIER[..],
        Species::Deer => &DEER[..],
        Species::Wolf => &WOLF[..],
        Species::Zombie => &ZOMBIE[..],
    };

    let (art_w, art_h) = art_size(rows);
    let origin_x = center_x - art_w * scale / 2;
    let origin_y = bottom_y - art_h * scale - scale / 2;

    // Мягкая тень под ногами.
    frame.blend_rect(
        center_x - scale,
        bottom_y - scale,
        (scale * 2) as u32,
        scale as u32,
        palette::SHADOW,
        0.30,
    );

    let mut painter = Painter::new(frame, origin_x, origin_y, scale);
    painter.art(rows, |symbol| match symbol {
        'o' => Some(match species {
            Species::Zombie => palette::ZOMBIE_DARK,
            Species::Human if variant % 2 == 0 => palette::SKIN,
            Species::Human => palette::SKIN_DARK,
            _ => palette::SKIN,
        }),
        's' => Some(palette::STEEL),
        '#' => match species {
            Species::Wolf => Some(if variant % 2 == 0 {
                palette::WOLF
            } else {
                palette::WOLF_DARK
            }),
            Species::Zombie => Some(if variant % 2 == 0 {
                palette::ZOMBIE
            } else {
                palette::ZOMBIE_DARK
            }),
            Species::Deer => Some(palette::DEER),
            _ => Some(cloth),
        },
        'l' => Some(palette::DEER_LIGHT),
        _ => None,
    });

    // Глаза хищников — заметная деталь на маленьком экране.
    if species == Species::Wolf {
        painter.blend_px(art_w / 2, 0, palette::EMBER, 0.85);
    }
}

/// Рисует постройку.
pub fn draw_building(
    frame: &mut Frame,
    kind: BuildingKind,
    center_x: i32,
    bottom_y: i32,
    px_per_tile: i32,
    nation_color: [u8; 3],
    time: f32,
    completion: f32,
    variant: u8,
) {
    let scale = (px_per_tile / 3).max(1);

    let rows: &[&str] = match kind {
        BuildingKind::House => &HOUSE[..],
        BuildingKind::Campfire => &CAMPFIRE[..],
        BuildingKind::Ruin => &RUIN[..],
    };
    let (art_w, art_h) = art_size(rows);
    let origin_x = center_x - art_w * scale / 2;
    let origin_y = bottom_y - art_h * scale + scale;

    frame.blend_rect(
        center_x - (art_w * scale) / 2,
        bottom_y - scale,
        (art_w * scale) as u32,
        scale as u32,
        palette::SHADOW,
        0.32,
    );

    let mut painter = Painter::new(frame, origin_x, origin_y, scale);

    match kind {
        BuildingKind::Campfire => {
            let flicker = ((time * 7.0 + variant as f32).sin() * 0.5 + 0.5) > 0.35;
            painter.art(rows, |symbol| match symbol {
                'f' => Some(if flicker {
                    palette::FLAME
                } else {
                    palette::EMBER
                }),
                '#' => Some(palette::EMBER),
                'w' => Some(palette::WOOD_DARK),
                _ => None,
            });
        }
        BuildingKind::House => {
            if completion >= 1.0 {
                painter.art(rows, |symbol| match symbol {
                    'r' => Some(palette::ROOF),
                    'w' => Some(palette::WOOD),
                    'd' => Some(palette::WOOD_DARK),
                    _ => None,
                });
                // Флаг страны на крыше.
                painter.px(art_w - 2, 0, nation_color);
                painter.blend_px(art_w - 2, 1, nation_color, 0.6);
            } else {
                draw_site(&mut painter, rows, completion, nation_color);
            }
        }
        BuildingKind::Ruin => {
            painter.art(rows, |symbol| match symbol {
                '#' => Some(palette::STONE),
                _ => None,
            });
        }
    }
}

/// Строительная площадка: план дома и уже возведённая часть.
fn draw_site(painter: &mut Painter<'_>, rows: &[&str], completion: f32, nation_color: [u8; 3]) {
    let (width, height) = art_size(rows);
    let completion = completion.clamp(0.0, 1.0);

    // Пунктирный план будущего дома.
    for x in 0..width {
        painter.blend_px(x, height - 1, palette::WOOD_DARK, 0.55);
    }
    for y in 0..height {
        painter.blend_px(0, y, palette::WOOD_DARK, 0.35);
        painter.blend_px(width - 1, y, palette::WOOD_DARK, 0.35);
    }

    // Возведённая снизу часть.
    let filled = (height as f32 * completion).ceil() as i32;
    for y in (height - filled)..height {
        for x in 0..width {
            painter.px(x, y, palette::WOOD);
        }
    }

    // Флажок страны у площадки.
    if completion > 0.25 {
        painter.px(width - 1, 0, nation_color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn art_size_matches_longest_row() {
        assert_eq!(art_size(&["###", "#", "##"]), (3, 3));
        assert_eq!(art_size(&[]), (0, 0));
    }

    #[test]
    fn human_is_drawn_above_the_tile() {
        let mut frame = Frame::filled(32, 32, [0, 0, 0]);
        draw_entity(
            &mut frame,
            Species::Human,
            16,
            24,
            3,
            [200, 60, 60],
            false,
            false,
        );
        // Пиксели появились выше нижней границы тайла.
        let mut painted = 0;
        for y in 0..32 {
            for x in 0..32 {
                if frame.get(x, y) != Some([0, 0, 0]) {
                    painted += 1;
                }
            }
        }
        assert!(painted > 8, "спрайт почти не нарисовался: {painted}");
    }

    #[test]
    fn buildings_draw_without_panic() {
        let mut frame = Frame::filled(48, 48, [0, 0, 0]);
        for kind in BuildingKind::ALL {
            draw_building(&mut frame, kind, 24, 36, 6, [90, 160, 210], 1.5, 1.0, 1);
            draw_building(&mut frame, kind, 24, 36, 3, [90, 160, 210], 1.5, 0.4, 2);
        }
    }

    #[test]
    fn every_species_has_a_sprite() {
        for species in Species::ALL {
            let mut frame = Frame::filled(24, 24, [0, 0, 0]);
            draw_entity(&mut frame, species, 12, 18, 3, [180, 180, 180], 0, true);
            assert_ne!(frame.pixels(), Frame::filled(24, 24, [0, 0, 0]).pixels());
        }
    }
}
