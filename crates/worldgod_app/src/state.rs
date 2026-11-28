//! Состояние приложения: мир, камера, кисть и тайминги.

use worldgod_core::{
    NationId, Relation, Simulation, TilePos, Tool, ToolConfig, Vec2, ViewTransform, WorldSize, SIM_HZ,
};

use crate::textures::SceneTexture;

/// Сглаженный счётчик кадров.
#[derive(Clone, Copy, Debug, Default)]
pub struct FpsMeter {
    smoothed_dt: f32,
}

impl FpsMeter {
    /// Учитывает длительность кадра.
    pub fn push(&mut self, dt: f64) {
        let dt = dt as f32;
        if !(0.0..1.0).contains(&dt) {
            return;
        }
        if self.smoothed_dt <= 0.0 {
            self.smoothed_dt = dt;
        } else {
            self.smoothed_dt = self.smoothed_dt * 0.9 + dt * 0.1;
        }
    }

    /// Кадров в секунду.
    pub fn fps(&self) -> f32 {
        if self.smoothed_dt > 0.0 {
            1.0 / self.smoothed_dt
        } else {
            0.0
        }
    }
}

/// Всё, что нужно интерфейсу для работы.
#[derive(Debug)]
pub struct AppState {
    /// Мир и его обитатели.
    pub sim: Simulation,
    /// Камера карты.
    pub view: ViewTransform,
    /// Выбранный инструмент и радиус.
    pub tool: ToolConfig,
    /// Кэш текстуры.
    pub textures: SceneTexture,
    /// Текущий размер мира (для меню «новая планета»).
    pub world_size: WorldSize,
    /// Зерно текущего мира.
    pub seed: u64,
    /// Доля пути между тиками для плавного движения.
    pub alpha: f32,
    /// Секунды с запуска (монотонные).
    pub time: f64,
    /// Открыто ли окно настроек.
    pub settings_open: bool,
    /// Показывать ли строку событий.
    pub show_events: bool,
    /// Счётчик FPS.
    pub fps: FpsMeter,
    /// Текстуру нужно перерисовать немедленно (игрок только что мазнул кистью).
    pub texture_dirty: bool,
    /// Камера уже подстроена под реальный размер области карты.
    pub view_initialized: bool,
    /// Первая страна, выбранная кистью «Война / Мир».
    pub diplomacy_pick: Option<NationId>,

    accumulator: f64,
    last_paint_tile: Option<TilePos>,
    last_paint_time: f64,
    last_spawn_time: f64,
}

impl AppState {
    /// Новое состояние с пустым океаном.
    pub fn new(size: WorldSize, seed: u64) -> Self {
        let sim = Simulation::new(size, seed);
        let view = ViewTransform::new(sim.size(), Vec2::ZERO, Vec2::new(800.0, 600.0));

        Self {
            sim,
            view,
            tool: ToolConfig::default(),
            textures: SceneTexture::default(),
            world_size: size,
            seed,
            alpha: 1.0,
            time: 0.0,
            settings_open: false,
            show_events: true,
            fps: FpsMeter::default(),
            texture_dirty: true,
            view_initialized: false,
            diplomacy_pick: None,
            accumulator: 0.0,
            last_paint_tile: None,
            last_paint_time: 0.0,
            last_spawn_time: 0.0,
        }
    }

    /// Создаёт новый мир (и новое зерно).
    pub fn new_world(&mut self, size: WorldSize) {
        self.world_size = size;
        self.seed = self
            .seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.sim = Simulation::new(size, self.seed);
        self.accumulator = 0.0;
        self.alpha = 1.0;
        self.textures.invalidate();
        self.texture_dirty = true;
        self.view_initialized = false;
        self.diplomacy_pick = None;
        self.tool = ToolConfig::default();
        self.view
            .set_viewport(self.view.origin, self.view.size);
        self.view.fit(self.sim.size());
    }

    /// Вписывает карту в экран.
    pub fn reset_view(&mut self) {
        self.view.fit(self.sim.size());
    }

    /// Продвигает время и симуляцию.
    pub fn advance(&mut self, dt: f64) {
        self.time += dt;
        self.fps.push(dt);

        if dt <= 0.0 {
            return;
        }

        self.accumulator += dt * SIM_HZ as f64 * self.sim.speed.multiplier() as f64;
        let steps = self.accumulator.floor().clamp(0.0, 8.0) as u32;
        self.accumulator -= steps as f64;

        for _ in 0..steps {
            self.sim.step();
        }

        self.alpha = self.accumulator.clamp(0.0, 1.0) as f32;
    }

    /// Тап кистью дипломатии: первая страна запоминается, второй тап
    /// переключает отношения пары (мир → война → союз → мир).
    pub fn pick_nation_at(&mut self, tile: TilePos) -> Option<Relation> {
        let nation = self.sim.nation_at(tile)?;
        let Some(first) = self.diplomacy_pick else {
            self.diplomacy_pick = Some(nation);
            return None;
        };
        if first == nation {
            self.diplomacy_pick = None;
            return None;
        }
        self.diplomacy_pick = None;
        self.sim.cycle_relation(first, nation)
    }

    /// Применяет текущую кисть к тайлу.
    pub fn apply_tool_at(&mut self, tile: TilePos) -> u32 {
        if self.tool.tool.is_diplomacy() {
            self.pick_nation_at(tile);
            return 0;
        }

        let changed = self.sim.apply_tool(self.tool, tile);
        if changed > 0 || self.tool.tool == Tool::Inspect {
            self.texture_dirty = true;
        }
        changed
    }

    /// Нужно ли применять кисть к тайлу прямо сейчас (антидребезг).
    pub fn brush_tick(&mut self, tile: TilePos) -> bool {
        if self.tool.tool.species().is_some() {
            if self.time - self.last_spawn_time < 0.3 {
                return false;
            }
            self.last_spawn_time = self.time;
            return true;
        }

        let moved = self.last_paint_tile != Some(tile);
        if moved || self.time - self.last_paint_time >= 0.08 {
            self.last_paint_tile = Some(tile);
            self.last_paint_time = self.time;
            true
        } else {
            false
        }
    }

    /// Строка состояния для нижней панели.
    pub fn status_line(&self) -> String {
        if self.tool.tool.is_diplomacy() {
            return match self.diplomacy_pick {
                Some(nation) => format!(
                    "{} выбрана · тапните по второй стране",
                    self.sim.nation_name(Some(nation))
                ),
                None => "Война / Мир: тапните по первой стране".to_string(),
            };
        }
        if self.tool.tool.is_terrain() {
            format!("{} · радиус {}", self.tool.tool.label_ru(), self.tool.clamped_radius())
        } else {
            self.tool.tool.hint_ru().to_string()
        }
    }

    /// Время мира в удобном виде: «12:34».
    pub fn world_clock(&self) -> String {
        let total = self.sim.tick / SIM_HZ as u64;
        format!("{:02}:{:02}", total / 60, total % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_steps_the_simulation() {
        let mut state = AppState::new(WorldSize::SMALL, 7);
        let before = state.sim.tick;
        state.advance(0.25);
        assert!(state.sim.tick > before);
        assert!(state.alpha >= 0.0 && state.alpha < 1.0);
    }

    #[test]
    fn paused_world_does_not_tick() {
        let mut state = AppState::new(WorldSize::SMALL, 7);
        state.sim.speed = worldgod_core::Speed::Paused;
        let before = state.sim.tick;
        state.advance(1.0);
        assert_eq!(state.sim.tick, before);
    }

    #[test]
    fn speed_multiplier_affects_tick_rate() {
        let mut slow = AppState::new(WorldSize::SMALL, 3);
        let mut fast = AppState::new(WorldSize::SMALL, 3);
        fast.sim.speed = worldgod_core::Speed::Turbo;
        slow.advance(0.5);
        fast.advance(0.5);
        assert!(fast.sim.tick > slow.sim.tick);
    }

    #[test]
    fn brush_tick_throttles_terrain() {
        let mut state = AppState::new(WorldSize::SMALL, 1);
        let tile = TilePos::new(10, 10);
        assert!(state.brush_tick(tile));
        assert!(!state.brush_tick(tile));
        state.time += 0.2;
        assert!(state.brush_tick(tile));
        assert!(state.brush_tick(TilePos::new(11, 10)));
    }

    #[test]
    fn new_world_resets_everything() {
        let mut state = AppState::new(WorldSize::MEDIUM, 5);
        state.advance(1.0);
        state.new_world(WorldSize::SMALL);
        assert_eq!(state.sim.tick, 0);
        assert_eq!(state.sim.size(), (WorldSize::SMALL.width, WorldSize::SMALL.height));
        assert!(state.textures.id().is_none());
    }

    #[test]
    fn diplomacy_brush_cycles_between_two_nations() {
        let mut state = AppState::new(WorldSize::SMALL, 11);
        let a = state.sim.found_nation_at(TilePos::new(60, 60));
        let b = state.sim.found_nation_at(TilePos::new(70, 70));
        state.tool.tool = Tool::Diplomacy;

        // Первый тап только запоминает страну.
        assert_eq!(state.pick_nation_at(TilePos::new(60, 60)), None);
        assert_eq!(state.diplomacy_pick, Some(a));
        // Второй тап по другой стране — сразу война.
        assert_eq!(state.pick_nation_at(TilePos::new(70, 70)), Some(Relation::War));
        assert!(state.sim.at_war(a, b));
        assert_eq!(state.sim.stats.wars, 1);

        // Ещё раз — союз против монстров.
        state.pick_nation_at(TilePos::new(60, 60));
        assert_eq!(state.pick_nation_at(TilePos::new(70, 70)), Some(Relation::Alliance));
        assert!(state.sim.allied(a, b));
        assert_eq!(state.sim.stats.alliances, 1);
    }

    #[test]
    fn world_clock_formats_minutes() {
        let mut state = AppState::new(WorldSize::SMALL, 1);
        state.sim.tick = 750;
        assert_eq!(state.world_clock(), "01:15");
    }

    #[test]
    fn fps_meter_smooths() {
        let mut meter = FpsMeter::default();
        assert_eq!(meter.fps(), 0.0);
        for _ in 0..60 {
            meter.push(1.0 / 60.0);
        }
        assert!((meter.fps() - 60.0).abs() < 2.0);
    }
}
