//! Кэш текстуры карты: программный кадр → текстура egui.

use eframe::egui;
use worldgod_core::Simulation;
use worldgod_render::{render_scene, RenderOptions};

/// Сколько пикселей на тайл рисует рендер (текстура потом растягивается).
pub const PX_PER_TILE: u32 = 3;

/// Текстура мира с ограничением частоты обновлений.
#[derive(Debug, Default)]
pub struct SceneTexture {
    handle: Option<egui::TextureHandle>,
    last_render: f64,
    last_size: (u32, u32),
}

impl SceneTexture {
    /// Забыть текстуру (например, после создания нового мира).
    pub fn invalidate(&mut self) {
        self.handle = None;
        self.last_size = (0, 0);
        self.last_render = -1.0;
    }

    /// Идентификатор текстуры, если она уже создана.
    pub fn id(&self) -> Option<egui::TextureId> {
        self.handle.as_ref().map(|handle| handle.id())
    }

    /// Перерисовывает кадр, если прошло достаточно времени (или `force`).
    pub fn ensure(
        &mut self,
        ctx: &egui::Context,
        sim: &Simulation,
        alpha: f32,
        time: f64,
        force: bool,
    ) {
        let running = !sim.speed.is_paused();
        let interval = if running { 1.0 / 30.0 } else { 1.0 / 8.0 };

        if !force && self.handle.is_some() && time - self.last_render < interval {
            return;
        }

        let options = RenderOptions {
            px_per_tile: PX_PER_TILE,
            time: time as f32,
            alpha,
            show_entities: true,
        };
        let frame = render_scene(sim, options);
        let (width, height) = frame.size();
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [width as usize, height as usize],
            frame.pixels(),
        );

        match &mut self.handle {
            Some(handle) if self.last_size == (width, height) => {
                handle.set(image, egui::TextureOptions::NEAREST);
            }
            _ => {
                self.handle = Some(ctx.load_texture(
                    "worldgod-map",
                    image,
                    egui::TextureOptions::NEAREST,
                ));
                self.last_size = (width, height);
            }
        }

        self.last_render = time;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldgod_core::WorldSize;

    #[test]
    fn texture_is_created_and_updated() {
        let ctx = egui::Context::default();
        let sim = Simulation::new(WorldSize::SMALL, 1);
        let mut texture = SceneTexture::default();
        assert!(texture.id().is_none());

        texture.ensure(&ctx, &sim, 1.0, 5.0, true);
        assert!(texture.id().is_some());

        // Повторный вызов раньше интервала ничего не ломает.
        texture.ensure(&ctx, &sim, 1.0, 5.01, false);
        texture.ensure(&ctx, &sim, 1.0, 9.0, false);
        assert!(texture.id().is_some());

        texture.invalidate();
        assert!(texture.id().is_none());
    }
}
