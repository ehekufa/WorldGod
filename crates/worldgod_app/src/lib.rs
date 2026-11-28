//! # WorldGod (приложение)
//!
//! Оболочка для Android и десктопа: окно `eframe`, карта как текстура
//! программного рендера и «нативный» интерфейс на `egui`.
//!
//! * [`app`] — точка сборки UI и жизненный цикл кадра;
//! * [`map_view`] — карта, жесты и кисть;
//! * [`state`] — состояние мира и камеры;
//! * [`hud`] — панели интерфейса;
//! * [`theme`] — цвета и шрифты.

// `deny`, а не `forbid`: единственное исключение — экспортируемый символ
// `android_main`, для которого нужен атрибут `#[no_mangle]` (см. `android.rs`).
#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

#[cfg(target_os = "android")]
pub mod android;

pub mod app;
pub mod hud;
pub mod map_view;
pub mod state;
pub mod textures;
pub mod theme;
pub mod ui_icons;

pub use app::WorldGodApp;

/// Запуск на десктопе (Linux/macOS/Windows).
#[cfg(not(target_os = "android"))]
pub fn run_desktop() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 720.0])
            .with_min_inner_size([360.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "WorldGod",
        options,
        Box::new(|cc| Ok(Box::new(app::WorldGodApp::new(cc)))),
    )
}
