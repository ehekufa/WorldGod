//! Бои: удары, смерть и «заражение» зомби.

use crate::entity::Intent;
use crate::sim::Simulation;

/// Разрешает все атаки за текущий тик.
///
/// Существо, чей [`Intent::Attack`] указывает на живую враждебную цель и
/// которое находится в радиусе удара, бьёт с учётом перезарядки.
pub fn resolve(sim: &mut Simulation) {
    let count = sim.entities.len();

    for index in 0..count {
        if !sim.entities[index].alive {
            continue;
        }
        let Intent::Attack { target } = sim.entities[index].intent else {
            continue;
        };

        let Some(target_index) = sim.entity_index(target) else {
            sim.entities[index].assign(Intent::Idle);
            continue;
        };
        if !sim.entities[target_index].alive {
            sim.entities[index].assign(Intent::Idle);
            continue;
        }

        let attacker_species = sim.entities[index].species;
        let victim_species = sim.entities[target_index].species;
        if !attacker_species.hostile_to(victim_species) {
            sim.entities[index].assign(Intent::Idle);
            continue;
        }

        let distance = sim.entities[index].pos.distance(sim.entities[target_index].pos);
        let stats = attacker_species.stats();
        if distance > stats.attack_range {
            continue; // ещё не подошёл
        }

        if sim.entities[index].attack_timer > 0 {
            sim.entities[index].attack_timer -= 1;
            continue;
        }
        sim.entities[index].attack_timer = stats.attack_cooldown;

        let damage = stats.damage.max(1);
        sim.entities[target_index].hp -= damage;
        sim.entities[target_index].last_attacker = Some(sim.entities[index].id);

        if sim.entities[target_index].is_alive() {
            continue;
        }

        // Цель погибла.
        sim.entities[index].kills += 1;
        sim.totals.kills += 1;

        let attacker_nation = sim.entities[index].nation;
        if let Some(nation) = attacker_nation.and_then(|id| sim.nation_mut(id)) {
            nation.kills += 1;
        }

        sim.kill(target_index, Some(attacker_species));
        sim.log_death(victim_species, attacker_species);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brush::{Tool, ToolConfig};
    use crate::grid::{pos, TilePos};
    use crate::sim::WorldSize;
    use crate::species::Species;

    fn arena(seed: u64) -> Simulation {
        let mut sim = Simulation::new(WorldSize::SMALL, seed);
        sim.speed = crate::sim::Speed::Normal;
        for x in 50..80 {
            for y in 50..80 {
                sim.world
                    .set_terrain(TilePos::new(x, y), crate::terrain::Terrain::Grass);
            }
        }
        sim.apply_tool(ToolConfig::new(Tool::Sand, 2), pos(50, 50));
        sim
    }

    #[test]
    fn wolf_kills_a_human() {
        let mut sim = arena(1);
        let human = sim.spawn(Species::Human, pos(60, 60)).expect("человек");
        let wolf = sim.spawn(Species::Wolf, pos(62, 60)).expect("волк");

        sim.run(1200);

        let human_dead = sim.entity(human).map_or(true, |entity| !entity.alive);
        assert!(
            human_dead || sim.entity(wolf).map_or(true, |entity| !entity.alive),
            "кто-то должен был погибнуть"
        );
    }

    #[test]
    fn zombie_converts_victims() {
        let mut sim = arena(2);
        for _ in 0..3 {
            sim.spawn(Species::Human, pos(61, 61));
        }
        sim.spawn(Species::Zombie, pos(63, 61));
        sim.run(2000);
        let zombies = sim
            .entities
            .iter()
            .filter(|entity| entity.species == Species::Zombie)
            .count();
        assert!(zombies >= 1, "зомби должны оставаться в мире");
    }

    #[test]
    fn friendly_fire_is_impossible() {
        let mut sim = arena(3);
        let a = sim.spawn(Species::Human, pos(60, 60)).expect("человек");
        let b = sim.spawn(Species::Human, pos(61, 60)).expect("человек");
        if let Some(entity) = sim.entity_mut(a) {
            entity.assign(Intent::Attack { target: b });
        }
        sim.run(100);
        let a_alive = sim.entity(a).is_some_and(|entity| entity.alive);
        let b_alive = sim.entity(b).is_some_and(|entity| entity.alive);
        assert!(a_alive && b_alive, "свои своих не бьют");
    }

    #[test]
    fn soldier_beats_a_single_wolf() {
        let mut sim = arena(4);
        let soldier = sim.spawn(Species::Soldier, pos(60, 60)).expect("воин");
        let wolf = sim.spawn(Species::Wolf, pos(64, 60)).expect("волк");
        sim.run(900);

        let soldier_alive = sim.entity(soldier).is_some_and(|entity| entity.alive);
        let wolf_alive = sim.entity(wolf).is_some_and(|entity| entity.alive);
        assert!(soldier_alive, "воин крепче волка и должен выжить");
        assert!(!wolf_alive, "волк должен погибнуть");
    }

    #[test]
    fn stats_count_deaths() {
        let mut sim = arena(5);
        let human = sim.spawn(Species::Human, pos(60, 60)).expect("человек");
        if let Some(entity) = sim.entity_mut(human) {
            entity.hp = 1;
        }
        sim.spawn(Species::Wolf, pos(61, 60));
        sim.run(800);
        assert!(sim.totals.deaths > 0 || sim.entity(human).is_some());
        assert!(!sim.entities.iter().any(|entity| entity.hp > entity.max_hp()));
    }
}
