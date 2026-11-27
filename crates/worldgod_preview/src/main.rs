//! Генератор скриншотов: собирает несколько сцен и сохраняет их в PNG.
//!
//! Сцены детерминированы (у каждой своё зерно), поэтому картинки в `docs/`
//! можно перегенерировать в любой момент и сравнить глазами или побайтово.
//!
//! ```text
//! cargo run --release -p worldgod_preview -- --out docs --px 4
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use worldgod_core::entity::EntityId;
use worldgod_core::{Simulation, Species, TilePos, Tool, ToolConfig, WorldSize};
use worldgod_render::{render_scene, Frame, RenderOptions};

/// Настройки запуска.
struct Options {
    out_dir: PathBuf,
    px_per_tile: u32,
}

fn main() -> ExitCode {
    let options = match parse_args(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(error) = std::fs::create_dir_all(&options.out_dir) {
        eprintln!("не удалось создать {}: {error}", options.out_dir.display());
        return ExitCode::FAILURE;
    }

    let scenes: [(&str, fn(u32) -> Frame); 4] = [
        ("preview-1-ocean.png", ocean_scene),
        ("preview-2-island.png", island_scene),
        ("preview-3-nation.png", nation_scene),
        ("preview-4-monsters.png", monsters_scene),
    ];

    let mut failed = false;
    for (name, build) in scenes {
        let frame = build(options.px_per_tile);
        let path = options.out_dir.join(name);
        match frame.save_png(&path) {
            Ok(()) => println!(
                "{} — {}×{} px",
                path.display(),
                frame.width(),
                frame.height()
            ),
            Err(error) => {
                eprintln!("не удалось сохранить {}: {error}", path.display());
                failed = true;
            }
        }
    }

    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options {
        out_dir: PathBuf::from("docs"),
        px_per_tile: 4,
    };
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                let value = args.next().ok_or("--out требует путь")?;
                options.out_dir = PathBuf::from(value);
            }
            "--px" => {
                let value = args.next().ok_or("--px требует число")?;
                let px: u32 = value.parse().map_err(|_| format!("не число: {value}"))?;
                options.px_per_tile = px.clamp(1, 12);
            }
            "--help" | "-h" => {
                return Err("использование: worldgod_preview [--out DIR] [--px N]".to_string());
            }
            other => return Err(format!("неизвестный аргумент: {other}")),
        }
    }

    Ok(options)
}

// --- сцены ------------------------------------------------------------------

fn ocean_scene(px: u32) -> Frame {
    let sim = Simulation::new(WorldSize::MEDIUM, 2024);
    render(&sim, px, 0.0)
}

fn island_scene(px: u32) -> Frame {
    let mut sim = Simulation::new(WorldSize::MEDIUM, 77);
    build_island(&mut sim);
    let _ = spawn_people(&mut sim, 4);
    for _ in 0..2 {
        let _ = spawn_near_center(&mut sim, Species::Deer);
    }
    sim.run(180);
    render(&sim, px, 4.0)
}

fn nation_scene(px: u32) -> Frame {
    let mut sim = Simulation::new(WorldSize::MEDIUM, 4242);
    build_island(&mut sim);
    let _ = spawn_people(&mut sim, 5);
    sim.run(3600);
    report("страна", &sim);
    render(&sim, px, 8.0)
}

fn monsters_scene(px: u32) -> Frame {
    let mut sim = Simulation::new(WorldSize::MEDIUM, 4242);
    build_island(&mut sim);
    let _ = spawn_people(&mut sim, 5);
    sim.run(3600);

    let center = center_of(&sim);
    for offset in [(-18, -6), (16, 8), (-8, 18)] {
        sim.apply_tool(
            ToolConfig::new(Tool::Wolf, 3),
            center.offset(offset.0, offset.1),
        );
    }
    for offset in [(22, -10), (-22, 4)] {
        sim.apply_tool(
            ToolConfig::new(Tool::Zombie, 3),
            center.offset(offset.0, offset.1),
        );
    }
    sim.run(400);
    report("нашествие", &sim);
    render(&sim, px, 12.0)
}

// --- сценарий ---------------------------------------------------------------

/// Рисует остров так, как это сделал бы игрок: несколько мазков суши,
/// лес, пляж и горный хребет.
fn build_island(sim: &mut Simulation) {
    let center = center_of(sim);

    let land_strokes = [
        (-34, -4, 13),
        (-18, -16, 14),
        (2, -20, 13),
        (20, -10, 13),
        (30, 4, 12),
        (16, 16, 14),
        (-6, 18, 13),
        (-26, 10, 12),
    ];
    for (dx, dy, radius) in land_strokes {
        sim.apply_tool(
            ToolConfig::new(Tool::Raise, radius),
            center.offset(dx, dy),
        );
    }

    for (dx, dy, radius) in [(-16, -10, 7), (2, -14, 6), (14, 4, 7), (-6, 10, 5)] {
        sim.apply_tool(
            ToolConfig::new(Tool::Forest, radius),
            center.offset(dx, dy),
        );
    }

    for (dx, dy, radius) in [(-4, 20, 5), (10, 20, 4)] {
        sim.apply_tool(
            ToolConfig::new(Tool::Sand, radius),
            center.offset(dx, dy),
        );
    }

    for (dx, dy, radius) in [(-8, -22, 4), (6, -24, 3)] {
        sim.apply_tool(
            ToolConfig::new(Tool::Mountain, radius),
            center.offset(dx, dy),
        );
    }
}

fn spawn_people(sim: &mut Simulation, count: u32) -> usize {
    let mut spawned = 0;
    for _ in 0..count {
        if spawn_near_center(sim, Species::Human).is_some() {
            spawned += 1;
        }
    }
    spawned
}

fn spawn_near_center(sim: &mut Simulation, species: Species) -> Option<EntityId> {
    let center = center_of(sim);
    let spot = sim
        .world
        .find_terrain_near(center, 16, |terrain| terrain.is_walkable())?;
    sim.spawn(species, spot)
}

fn center_of(sim: &Simulation) -> TilePos {
    let (width, height) = sim.size();
    TilePos::new(width as i32 / 2, height as i32 / 2)
}

fn render(sim: &Simulation, px: u32, time: f32) -> Frame {
    render_scene(
        sim,
        RenderOptions {
            px_per_tile: px,
            time,
            alpha: 1.0,
            show_entities: true,
        },
    )
}

fn report(label: &str, sim: &Simulation) {
    let stats = sim.stats;
    println!(
        "{label}: тик {}, люди {}, страны {}, дома {}, монстры {}, суша {:.1}%",
        stats.tick,
        stats.population,
        stats.nations,
        stats.houses,
        stats.monsters,
        stats.land_fraction * 100.0
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenes_have_size_and_pixels() {
        for px in [1u32, 3, 4] {
            let frame = ocean_scene(px);
            assert_eq!(
                frame.size(),
                (WorldSize::MEDIUM.width * px, WorldSize::MEDIUM.height * px)
            );
            assert!(frame.to_png().len() > 1000);
        }
    }

    #[test]
    fn args_are_parsed() {
        use std::path::Path;

        let options =
            parse_args(["--out".to_string(), "target/out".to_string(), "--px".to_string(), "2".to_string()]
                .into_iter())
            .expect("параметры");
        assert_eq!(options.out_dir, Path::new("target/out"));
        assert_eq!(options.px_per_tile, 2);

        let options = parse_args(std::iter::empty()).expect("по умолчанию");
        assert_eq!(options.px_per_tile, 4);

        assert!(parse_args(["--px".to_string(), "abc".to_string()].into_iter()).is_err());
        assert!(parse_args(["--bad".to_string()].into_iter()).is_err());
    }

    #[test]
    fn island_has_land_and_people() {
        let sim = {
            let mut sim = Simulation::new(WorldSize::SMALL, 5);
            build_island(&mut sim);
            let _ = spawn_people(&mut sim, 2);
            sim.run(20);
            sim
        };
        assert!(sim.stats.land_tiles > 100);
        assert!(sim.stats.population >= 1);
    }
}
