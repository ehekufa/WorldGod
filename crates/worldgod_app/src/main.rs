//! Десктопная точка входа.
//!
//! На Android этот файл не используется: там приложение собирается как
//! `cdylib`, а точкой входа служит `android_main` из `src/android.rs`.

#[cfg(not(target_os = "android"))]
fn main() {
    if let Err(error) = worldgod_app::run_desktop() {
        eprintln!("WorldGod не запустился: {error}");
    }
}

#[cfg(target_os = "android")]
fn main() {
    // Ничего: приложение стартует через NativeActivity (см. android.rs).
}
