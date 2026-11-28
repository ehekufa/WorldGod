//! # WorldGod Render
//!
//! Пиксельный программный рендер мира в RGBA-кадр.
//!
//! Зачем свой рендер, а не GPU-сцена: карта — это сетка из десятков тысяч
//! тайлов, и её проще всего рисовать в обычный буфер. Буфер отдаётся в
//! `egui` одной текстурой (на Android это один `glTexSubImage2D`), а тот же
//! код без изменений генерирует скриншоты для документации и тестов.
//!
//! ```
//! use worldgod_core::{Simulation, WorldSize, Tool, ToolConfig, TilePos};
//! use worldgod_render::{render_scene, RenderOptions};
//!
//! let mut sim = Simulation::new(WorldSize::SMALL, 1);
//! sim.apply_tool(ToolConfig::new(Tool::Raise, 5), TilePos::new(60, 60));
//!
//! let frame = render_scene(&sim, RenderOptions::default());
//! assert!(frame.width() > 0);
//! ```

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod framebuffer;
pub mod palette;
pub mod png;
pub mod scene;
pub mod sprite;
pub mod terrain_pass;

pub use framebuffer::Frame;
pub use scene::{render_scene, scene_size, RenderOptions};
