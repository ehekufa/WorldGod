//! Точка входа Android (`NativeActivity`).
//!
//! Android сам вызывает `android_main` из нашей `cdylib`. Всё остальное —
//! обычный `eframe`: окно создаётся нативным слоем winit, а мы получаем
//! тот же `egui`, что и на десктопе.
//!
//! Это единственное место в приложении, где разрешён «небезопасный»
//! атрибут: `#[no_mangle]` относится к линту `unsafe_code`, хотя самого
//! `unsafe`-кода здесь нет — задаётся только имя символа для линковщика.

#![allow(unsafe_code)]

use eframe::NativeOptions;
use winit::platform::android::activity::AndroidApp;

/// Точка входа, которую вызывает Android из `NativeActivity`.
#[no_mangle]
pub fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    let options = NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };

    if let Err(error) = eframe::run_native(
        "WorldGod",
        options,
        Box::new(|cc| Ok(Box::new(crate::app::WorldGodApp::new(cc)))),
    ) {
        log::error!("WorldGod: не удалось запустить приложение: {error}");
    }
}
