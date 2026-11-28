//! # WorldGod Core
//!
//! Чистое, детерминированное ядро симуляции бога-песочницы в духе WorldBox.
//!
//! Ядро ничего не знает о графике, окнах и вводе: это обычная библиотека
//! на `std`, без внешних зависимостей и без `unsafe`.
//!
//! ## Модель мира
//!
//! * [`World`] — сетка тайлов. У тайла есть высота (`elevation`), влажность
//!   (`moisture`) и декоративный вариант. Тип рельефа ([`Terrain`]) выводится
//!   из этих двух чисел, поэтому кисти работают с рельефом, а не с "картинкой".
//! * [`Simulation`] — владелец мира, жителей, построек, стран и журнала событий.
//!   Шаг симуляции — [`Simulation::step`], 10 тиков в секунду ([`SIM_HZ`]).
//! * [`brush`] — инструменты игрока: поднять сушу, опустить в воду, вырастить
//!   лес, насыпать пляж, поднять горы, заселить мир существами и объявить
//!   войну или союз между странами.
//! * [`nation`] — страны и таблица дипломатии: у каждой пары стран есть
//!   статус «мир», «союз» или «война»; на войне воины идут на чужую землю,
//!   а в союзе защищают жителей союзника от монстров.
//!
//! ## Быстрый старт
//!
//! ```
//! use worldgod_core::{Simulation, WorldSize, ToolConfig, Tool, TilePos};
//!
//! let mut sim = Simulation::new(WorldSize::SMALL, 42);
//! assert!(sim.world.land_fraction() == 0.0, "мир начинается с океана");
//!
//! // Поднимаем сушу кистью.
//! sim.apply_tool(ToolConfig::new(Tool::Raise, 4), TilePos::new(70, 70));
//! sim.apply_tool(ToolConfig::new(Tool::Raise, 4), TilePos::new(76, 72));
//! assert!(sim.world.land_fraction() > 0.0);
//!
//! // Заселяем остров людьми.
//! let mut rng = worldgod_core::Rng::new(1);
//! let land = sim.world.random_walkable(&mut rng).expect("есть суша");
//! sim.spawn(worldgod_core::Species::Human, land);
//!
//! // Живём.
//! for _ in 0..600 {
//!     sim.step();
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod ai;
pub mod brush;
pub mod building;
pub mod camera;
pub mod combat;
pub mod entity;
pub mod events;
pub mod grid;
pub mod math;
pub mod names;
pub mod nation;
pub mod noise;
pub mod pathfind;
pub mod rng;
pub mod sim;
pub mod species;
pub mod terrain;
pub mod world;

pub use brush::{Tool, ToolConfig};
pub use building::{Building, BuildingId, BuildingKind};
pub use camera::ViewTransform;
pub use entity::{Entity, EntityId, Intent};
pub use events::{EventKind, EventLog, GameEvent};
pub use grid::{pos, TilePos};
pub use math::Vec2;
pub use nation::{Diplomacy, Nation, NationId, Relation};
pub use rng::Rng;
pub use sim::{Simulation, Speed, Stats, WorldSize, SIM_HZ};
pub use species::Species;
pub use terrain::{Terrain, SEA_LEVEL};
pub use world::{Tile, World};

/// Версия симуляции: увеличивается при несовместимых изменениях формата мира.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
