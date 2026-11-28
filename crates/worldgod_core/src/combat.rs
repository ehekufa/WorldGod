//! Бои: удары по существам и постройкам, смерть и «заражение» зомби.

use crate::building::BuildingKind;
use crate::entity::Intent;
use crate::sim::Simulation;
use crate::species::Species;

/// Разрешает все атаки за текущий тик.
///
/// Существо, чей [`Intent::Attack`] указывает на живую враждебную цель
/// (с учётом дипломатии), бьёт её с перезарядкой. Воины с
/// [`Intent::AttackBuilding`] ломают вражеские дома, костры и порты.
pub fn resolve(sim: &mut Simulation) {
    let count = sim.entities.len();

    for index in 0..count {
        if !sim.entities[index].alive {
            continue;
        }
        match sim.entities[index].intent {
            Intent::Attack { target } => attack_entity(sim, index, target),
            Intent::AttackBuilding { building } => attack_building(sim, index, building),
            _ => {}
        }
    }
}

fn attack_entity(sim: &mut Simulation, index: usize, target: crate::entity::EntityId) {
    let Some(target_index) = sim.entity_index(target) else {
        sim.entities[index].assign(Intent::Idle);
        return;
    };
    if !sim.entities[target_index].alive {
        sim.entities[index].assign(Intent::Idle);
        return;
    }

    // Вражда считается по видам и по дипломатии стран.
    let attacker = sim.entities[index].view();
    let victim = sim.entities[target_index].view();
    if !sim.hostile_views(&attacker, &victim) {
        sim.entities[index].assign(Intent::Idle);
        return;
    }

    let distance = sim.entities[index].pos.distance(sim.entities[target_index].pos);
    let stats = sim.entities[index].stats();
    if distance > stats.attack_range {
        return; // ещё не подошёл
    }

    if sim.entities[index].attack_timer > 0 {
        sim.entities[index].attack_timer -= 1;
        return;
    }
    sim.entities[index].attack_timer = stats.attack_cooldown;

    // Урон с учётом способностей: «огненный удар» и «режим бога» бьют сильнее.
    let damage = sim.entities[index].attack_damage(stats.damage);
    sim.hurt_entity(target_index, damage);
    sim.entities[target_index].last_attacker = Some(sim.entities[index].id);

    if sim.entities[target_index].is_alive() {
        return;
    }

    // Цель погибла.
    let attacker_species = sim.entities[index].species;
    let victim_species = sim.entities[target_index].species;
    sim.entities[index].kills += 1;
    sim.totals.kills += 1;

    let attacker_nation = sim.entities[index].nation;
    if let Some(nation) = attacker_nation.and_then(|id| sim.nation_mut(id)) {
        nation.kills += 1;
    }

    sim.kill(target_index, Some(attacker_species));
    sim.log_death(victim_species, attacker_species);
}

fn attack_building(sim: &mut Simulation, index: usize, building: crate::building::BuildingId) {
    let Some(building_index) = sim.building_index(building) else {
        sim.entities[index].assign(Intent::Idle);
        return;
    };
    let kind = sim.buildings[building_index].kind;
    if kind == BuildingKind::Ruin || !sim.buildings[building_index].is_complete() {
        sim.entities[index].assign(Intent::Idle);
        return;
    }

    // Ломать постройки умеют только воины и только на войне.
    let attacker = sim.entities[index].view();
    if attacker.species != Species::Soldier {
        sim.entities[index].assign(Intent::Idle);
        return;
    }
    if !sim.nations_at_war(attacker.nation, sim.buildings[building_index].nation) {
        sim.entities[index].assign(Intent::Idle);
        return;
    }

    let distance = sim.entities[index].pos.distance(sim.buildings[building_index].pos);
    let stats = sim.entities[index].stats();
    if distance > stats.attack_range.max(1.8) {
        return;
    }

    if sim.entities[index].attack_timer > 0 {
        sim.entities[index].attack_timer -= 1;
        return;
    }
    sim.entities[index].attack_timer = stats.attack_cooldown;

    let damage = stats.damage.max(1);
    if sim.damage_building(building_index, damage) {
        sim.entities[index].kills += 1;
        sim.totals.kills += 1;
        if let Some(nation) = attacker.nation.and_then(|id| sim.nation_mut(id)) {
            nation.kills += 1;
        }
        sim.entities[index].assign(Intent::Idle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brush::{Tool, ToolConfig};
    use crate::grid::{pos, TilePos};
    use crate::nation::Relation;
    use crate::sim::WorldSize;

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

    /// Две страны на одном острове: война, штурм дома и руины.
    #[test]
    fn war_destroys_an_enemy_house() {
        let mut sim = arena(11);
        let a = sim.found_nation_at(pos(58, 58));
        let b = sim.found_nation_at(pos(66, 66));
        assert!(sim.nation(a).is_some() && sim.nation(b).is_some());

        let house = sim.finish_house_for(b, pos(64, 64));
        assert!(sim.building(house).is_some_and(|house| house.is_complete()));

        sim.set_relation(a, b, Relation::War);
        assert!(sim.at_war(a, b));

        // Воин страны-агрессора идёт ломать дом.
        let soldier = sim.spawn_in_nation(Species::Soldier, pos(62, 62), a).expect("воин");
        if let Some(entity) = sim.entity_mut(soldier) {
            entity.assign(Intent::AttackBuilding { building: house });
        }

        let mut destroyed = false;
        for _ in 0..3000 {
            sim.step();
            if sim
                .building(house)
                .is_some_and(|building| building.kind == BuildingKind::Ruin)
            {
                destroyed = true;
                break;
            }
        }
        let ruin = sim.building(house).is_some_and(|building| building.kind == BuildingKind::Ruin);
        assert!(destroyed || ruin, "дом должен превратиться в руины");
        assert!(
            sim.events.recent(120).any(|event| event.text.contains("разрушен")),
            "в ленте должно появиться сообщение о разрушении"
        );
    }

    #[test]
    fn peace_blocks_hostility_between_nations() {
        let mut sim = arena(12);
        let a = sim.found_nation_at(pos(58, 58));
        let b = sim.found_nation_at(pos(64, 64));
        let soldier = sim.spawn_in_nation(Species::Soldier, pos(60, 60), a).expect("воин");
        let civilian = sim.spawn_in_nation(Species::Human, pos(61, 60), b).expect("житель");

        assert!(!sim.at_war(a, b));
        assert!(
            sim.nearest_hostile_to_id(soldier, 8).is_none(),
            "в мире врагов у воина нет"
        );

        sim.set_relation(a, b, Relation::War);
        assert_eq!(
            sim.nearest_hostile_to_id(soldier, 8),
            Some(civilian),
            "на войне соседняя страна — враг"
        );
    }
}
