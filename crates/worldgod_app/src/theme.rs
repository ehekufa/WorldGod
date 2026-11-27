//! Тема оформления: цвета, шрифты и «нативная» плотность интерфейса.
//!
//! Стиль сознательно близок к системному Android: тёмная поверхность,
//! скруглённые кнопки, шрифт DejaVu Sans (полная кириллица) и минимум
//! декоративных эффектов.

use eframe::egui;
use egui::{Color32, FontFamily, FontId, Stroke, TextStyle, Visuals};

/// Самый тёмный фон (под картой).
pub const BACKGROUND: Color32 = Color32::from_rgb(10, 16, 24);
/// Поверхность панелей.
pub const SURFACE: Color32 = Color32::from_rgb(21, 27, 35);
/// Чуть более светлая поверхность (кнопки).
pub const SURFACE_ALT: Color32 = Color32::from_rgb(31, 39, 49);
/// Наведение.
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(42, 52, 64);
/// Нажатие.
pub const SURFACE_ACTIVE: Color32 = Color32::from_rgb(54, 66, 80);

/// Основной текст.
pub const TEXT: Color32 = Color32::from_rgb(232, 236, 242);
/// Второстепенный текст.
pub const TEXT_DIM: Color32 = Color32::from_rgb(148, 160, 176);
/// Акцент (системный синий Android).
pub const ACCENT: Color32 = Color32::from_rgb(96, 172, 255);
/// Тёмный акцент для фонов.
pub const ACCENT_DARK: Color32 = Color32::from_rgb(26, 66, 106);
/// Опасность / монстры.
pub const DANGER: Color32 = Color32::from_rgb(255, 112, 100);
/// Успех / жизнь.
pub const SUCCESS: Color32 = Color32::from_rgb(124, 210, 134);
/// Предупреждение / золото.
pub const WARNING: Color32 = Color32::from_rgb(240, 190, 92);

/// Скругление углов кнопок и карточек.
pub const RADIUS: u8 = 12;

/// Устанавливает шрифты и стиль приложения.
pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    install_style(ctx);
}

fn install_fonts(ctx: &egui::Context) {
    const DEJAVU: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");

    // Стиль игры тёмный независимо от системной темы.
    ctx.set_theme(egui::ThemePreference::Dark);


    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "dejavu".to_owned(),
        egui::FontData::from_static(DEJAVU).into(),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "dejavu".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push("dejavu".to_owned());
    ctx.set_fonts(fonts);
}

fn install_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.text_styles = [
        (TextStyle::Heading, FontId::proportional(18.0)),
        (TextStyle::Body, FontId::proportional(14.0)),
        (TextStyle::Button, FontId::proportional(14.0)),
        (TextStyle::Small, FontId::proportional(11.5)),
        (TextStyle::Monospace, FontId::monospace(13.0)),
    ]
    .into();

    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 7.0);
    style.spacing.slider_width = 150.0;
    style.spacing.interact_size.y = 34.0;
    style.visuals = visuals();

    ctx.set_style(style);
    ctx.set_visuals(visuals());
}

/// Тёмная тема в духе системного интерфейса.
pub fn visuals() -> Visuals {
    let mut visuals = Visuals::dark();

    visuals.panel_fill = SURFACE;
    visuals.window_fill = SURFACE_ALT;
    visuals.extreme_bg_color = BACKGROUND;
    visuals.faint_bg_color = SURFACE_ALT;
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = ACCENT_DARK;
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = WARNING;
    visuals.error_fg_color = DANGER;

    visuals.widgets.noninteractive.bg_fill = SURFACE;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_DIM);
    visuals.widgets.inactive.bg_fill = SURFACE_ALT;
    visuals.widgets.inactive.weak_bg_fill = SURFACE_ALT;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.hovered.bg_fill = SURFACE_HOVER;
    visuals.widgets.hovered.weak_bg_fill = SURFACE_HOVER;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, TEXT);
    visuals.widgets.active.bg_fill = SURFACE_ACTIVE;
    visuals.widgets.active.weak_bg_fill = SURFACE_ACTIVE;
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, TEXT);

    visuals
}

/// Цвет для страны (для чипов и полосок).
pub fn nation_color(color: [u8; 3]) -> Color32 {
    Color32::from_rgb(color[0], color[1], color[2])
}

/// Полупрозрачный цвет.
pub fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}
