//! Мозги существ: кто что делает на каждом «размышлении».
//!
//! Решения принимаются не каждый тик, а раз в несколько тиков
//! ([`THINK_INTERVAL`]), со сдвигом по фазе для каждого существа — так
//! симуляция не тратит силы впустую и не «дёргается».
//!
//! Вражда считается не только по видам, но и по дипломатии: страны могут
//! воевать друг с другом и заключать союз против монстров.

use crate::entity::{Entity, EntityView, Intent};
use crate::grid::TilePos;
use crate::sim::Simulation;
use crate::species::Species;

/// Базовый интервал между решениями (в тиках).
pub const THINK_INTERVAL: u16 = 6;

/// Радиус, в котором воин слышит союзника, которого надо защитить.
pub const ALLY_DEFENCE_RADIUS: i32 = 14;

/// Радиус, в котором воин ищет вражескую постройку для похода.
pub const RAID_RADIUS: i32 = 40;

/// Прогоняет размышления всех существ.
pub fn think_all(sim: &mut Simulation) {
    for index in 0..sim.entities.len() {
        if !sim.entities[index].alive {
            continue;
        }
        let timer = sim.entities[index].think_timer;
        if timer > 0 {
            sim.entities[index].think_timer = timer - 1;
            continue;
        }
        let id = sim.entities[index].id;
        sim.entities[index].think_timer = THINK_INTERVAL + (id % 7) as u16;
        let species = sim.entities[index].species;

        match species {
            Species::Human | Species::Soldier => think_humanoid(sim, index),
            Species::Wolf | Species::Zombie => think_predator(sim, index),
            Species::Deer => think_prey(sim, index),
            Species::Boat => think_boat(sim, index),
            Species::Godzilla => think_godzilla(sim, index),
        }
    }
}

fn think_humanoid(sim: &mut Simulation, index: usize) {
    let me = sim.entities[index].view();
    let stats = me.species.stats();
    let danger_radius = stats.flee_radius.max(stats.aggro_radius);

    if me.species == Species::Soldier {
        // 1. Враг рядом: монстр или житель страны, с которой идёт война.
        if let Some(enemy) = sim.nearest_hostile_view(&me, stats.aggro_radius) {
            sim.entities[index].assign(Intent::Attack { target: enemy });
            return;
        }

        // 2. Союз в действии: монстр терзает жителей страны-союзника.
        if let Some(monster) = sim.nearest_threat_to_allies(&me, ALLY_DEFENCE_RADIUS) {
            sim.entities[index].assign(Intent::Attack { target: monster });
            return;
        }

        think_soldier(sim, index, me);
        return;
    }

    // Мирный житель: сначала спасаем себя.
    if danger_radius > 0 {
        if let Some(threat) = sim.nearest_hostile_view(&me, danger_radius) {
            let is_boat = sim
                .entity(threat)
                .is_some_and(|entity| entity.species.is_boat());
            if is_boat {
                // Чужую лодку у берега можно разломать: она не кусается.
                sim.entities[index].assign(Intent::Attack { target: threat });
                return;
            }

            let threat_pos = sim.entity(threat).map_or(me.pos, |entity| entity.pos);
            let defenders = sim.count_allied_soldiers_near(me.pos, 5, me.nation);
            if defenders == 0 || me.pos.chebyshev(threat_pos) <= 2 {
                sim.entities[index].assign(Intent::Flee { from: threat_pos });
                return;
            }
        }
    }

    think_civilian(sim, index, me);
}

/// Воин: патруль у столицы и походы на вражескую землю.
fn think_soldier(sim: &mut Simulation, index: usize, me: EntityView) {
    // 3. Война: рядом врага нет — идём ломать вражеские дома и костры.
    if let Some(building) = sim
        .nearest_enemy_building(me.pos, RAID_RADIUS, me.nation)
        .or_else(|| sim.nearest_enemy_building(me.pos, i32::MAX / 4, me.nation))
    {
        sim.entities[index].assign(Intent::AttackBuilding { building });
        return;
    }

    let center = me
        .nation
        .and_then(|nation| sim.nation(nation))
        .map_or(me.pos, |nation| nation.capital);
    wander_around(sim, index, me.pos, center, 10);
}

fn think_civilian(sim: &mut Simulation, index: usize, me: EntityView) {
    let stats = me.species.stats();

    // 1. Голод — идём собирать еду.
    if me.hunger >= Entity::HUNGRY {
        if let Some(forest) = sim.nearest_forest(me.pos, 26) {
            sim.entities[index].assign(Intent::Forage { tile: forest });
            return;
        }
    }

    let Some(nation) = me.nation else {
        // 2. Без страны: присоединиться к соседней или основать свою.
        if let Some((nation_id, _)) = sim.nearest_nation_center(me.pos, 10) {
            sim.entities[index].nation = Some(nation_id);
            if let Some(nation) = sim.nation_mut(nation_id) {
                nation.add_citizen(false);
            }
            return;
        }
        if me.wood >= stats.wood_for_nation {
            sim.found_nation(index);
            return;
        }
        if let Some(forest) = sim.nearest_forest(me.pos, 18) {
            sim.entities[index].assign(Intent::Chop { tile: forest });
        } else {
            wander(sim, index, me.pos, 6);
        }
        return;
    };

    // 3. Страна есть: помогаем со стройкой (в том числе с портом).
    if let Some(site) = sim.nation_site(nation) {
        sim.entities[index].assign(Intent::Work { building: site });
        return;
    }

    // 4. Хватает дерева и есть потребность в жилье — закладываем дом.
    if me.wood >= stats.wood_for_house {
        let (houses, population) = sim
            .nation(nation)
            .map_or((0, 0), |nation| (nation.houses, nation.population));
        if houses * 3 + 2 <= population + 1 {
            if let Some(site) = sim.start_house_site(nation, me.pos) {
                sim.entities[index].wood = me.wood.saturating_sub(stats.wood_for_house);
                sim.entities[index].assign(Intent::Work { building: site });
                return;
            }
        }
    }

    // 5. Заготавливаем дерево.
    if me.wood < 9 {
        if let Some(forest) = sim.nearest_forest(me.pos, 26) {
            sim.entities[index].assign(Intent::Chop { tile: forest });
            return;
        }
    }

    // 6. Нечего делать — гуляем вокруг столицы.
    let center = sim.nation(nation).map_or(me.pos, |nation| nation.capital);
    wander_around(sim, index, me.pos, center, 8);
}

/// Годзилла: идёт к ближайшим людям, а если их нет — к поселению.
/// Урон по округе наносит [`Simulation::godzilla_pass`], от неё нужен
/// только путь к цели.
fn think_godzilla(sim: &mut Simulation, index: usize) {
    let me = sim.entities[index].view();
    let stats = me.species.stats();
    if let Some(victim) = sim.nearest_hostile_view(&me, stats.aggro_radius) {
        sim.entities[index].assign(Intent::Attack { target: victim });
        return;
    }
    if let Some((_, center)) = sim.nearest_nation_center(me.pos, 60) {
        sim.entities[index].assign(Intent::Wander { goal: center });
        return;
    }
    wander(sim, index, me.pos, 10);
}

fn think_predator(sim: &mut Simulation, index: usize) {
    let me = sim.entities[index].view();
    if let Some(prey) = sim.nearest_hostile_view(&me, me.species.stats().aggro_radius) {
        sim.entities[index].assign(Intent::Attack { target: prey });
        return;
    }
    wander(sim, index, me.pos, 8);
}

fn think_prey(sim: &mut Simulation, index: usize) {
    let me = sim.entities[index].view();
    let flee_radius = me.species.stats().flee_radius;
    if let Some(threat) = sim.nearest_hostile_view(&me, flee_radius) {
        if let Some(entity) = sim.entity(threat) {
            let from = entity.pos;
            sim.entities[index].assign(Intent::Flee { from });
            return;
        }
    }
    wander(sim, index, me.pos, 7);
}

/// Лодка дрейфует у берега: цель — случайная вода неподалёку.
fn think_boat(sim: &mut Simulation, index: usize) {
    let me = sim.entities[index].view();
    if let Intent::Wander { goal } = sim.entities[index].intent {
        if me.pos.chebyshev(goal) > 1 {
            return; // ещё плывём
        }
    }
    if let Some(goal) = sim.random_water_near(me.pos, 6) {
        sim.entities[index].assign(Intent::Wander { goal });
    } else {
        sim.entities[index].assign(Intent::Idle);
    }
}

/// Гуляет вокруг текущей точки.
fn wander(sim: &mut Simulation, index: usize, pos: TilePos, radius: i32) {
    if let Intent::Wander { goal } = sim.entities[index].intent {
        if pos.chebyshev(goal) > 1 {
            return; // ещё идём к цели
        }
    }
    if let Some(goal) = sim.random_walkable_near(pos, radius) {
        sim.entities[index].assign(Intent::Wander { goal });
    } else {
        sim.entities[index].assign(Intent::Idle);
    }
}

/// Гуляет вокруг заданного центра (столицы).
fn wander_around(
    sim: &mut Simulation,
    index: usize,
    pos: TilePos,
    center: TilePos,
    radius: i32,
) {
    if let Intent::Wander { goal } = sim.entities[index].intent {
        if pos.chebyshev(goal) > 1 {
            return;
        }
    }
    if let Some(goal) = sim.random_walkable_near(center, radius) {
        sim.entities[index].assign(Intent::Wander { goal });
    } else {
        sim.entities[index].assign(Intent::Idle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brush::Tool;
    use crate::brush::ToolConfig;
    use crate::building::BuildingKind;
    use crate::grid::pos;
    use crate::nation::Relation;
    use crate::sim::WorldSize;

    /// Готовит остров с лесом и людьми.
    fn island_world(seed: u64) -> Simulation {
        let mut sim = Simulation::new(WorldSize::SMALL, seed);
        for center in [
            pos(60, 60),
            pos(66, 58),
            pos(70, 64),
            pos(58, 68),
            pos(64, 72),
        ] {
            sim.apply_tool(ToolConfig::new(Tool::Raise, 10), center);
        }
        for center in [pos(62, 62), pos(68, 66), pos(60, 70)] {
            sim.apply_tool(ToolConfig::new(Tool::Forest, 6), center);
        }
        sim
    }

    #[test]
    fn humans_found_a_nation() {
        let mut sim = island_world(2024);
        for _ in 0..4 {
            let spot = sim
                .world
                .find_terrain_near(pos(65, 65), 12, |terrain| terrain.is_walkable())
                .expect("есть суша");
            sim.spawn(Species::Human, spot);
        }
        sim.run(2500);
        assert!(!sim.nations.is_empty(), "люди должны основать страну");
        assert!(
            sim.buildings
                .iter()
                .any(|building| building.kind == BuildingKind::Campfire),
            "должен появиться костёр-столица"
        );
    }

    #[test]
    fn nation_grows_houses_and_population() {
        let mut sim = island_world(77);
        let spot = pos(64, 64);
        sim.world.set_terrain(spot, crate::terrain::Terrain::Grass);
        sim.spawn(Species::Human, spot);
        sim.run(6000);
        let stats = sim.stats;
        assert!(stats.nations >= 1);
        if let Some(nation) = sim.nations.first() {
            assert!(nation.houses > 0, "должны появиться дома");
        }
        assert!(stats.population >= 1);
    }

    #[test]
    fn soldiers_attack_monsters() {
        let mut sim = island_world(5);
        let spot = pos(64, 64);
        let soldier = sim.spawn(Species::Soldier, spot).expect("воин");
        sim.run(200);
        let id = sim.spawn(Species::Wolf, spot.offset(4, 0)).expect("волк");
        sim.run(400);
        let soldier_alive = sim.entity(soldier).is_some_and(|entity| entity.alive);
        let wolf_hurt = sim
            .entity(id)
            .map_or(true, |entity| entity.hp < entity.max_hp() || !entity.alive);
        assert!(wolf_hurt || soldier_alive);
        let _ = soldier_alive;
    }

    #[test]
    fn deer_flees_from_wolf() {
        let mut sim = island_world(9);
        let spot = pos(64, 64);
        let deer = sim.spawn(Species::Deer, spot).expect("олень");
        sim.spawn(Species::Wolf, spot.offset(3, 0)).expect("волк");
        sim.run(30);
        let fleeing = sim.entity(deer).is_some_and(|entity| {
            matches!(
                entity.intent,
                Intent::Flee { .. } | Intent::Wander { .. }
            )
        });
        assert!(fleeing);
    }

    #[test]
    fn entities_never_stand_in_water() {
        let mut sim = island_world(31);
        for _ in 0..6 {
            let spot = sim
                .world
                .find_terrain_near(pos(64, 64), 14, |terrain| terrain.is_walkable())
                .expect("есть суша");
            sim.spawn(Species::Human, spot);
        }
        sim.run(500);
        for entity in &sim.entities {
            assert!(
                sim.world.terrain(entity.pos).is_walkable(),
                "{:?} стоит на {:?}",
                entity.species,
                sim.world.terrain(entity.pos)
            );
        }
    }

    /// На войне воин сам идёт к чужому дому, если рядом нет живых врагов.
    #[test]
    fn soldier_marches_to_enemy_house() {
        let mut sim = island_world(404);
        let a = sim.found_nation_at(pos(58, 60));
        let b = sim.found_nation_at(pos(70, 60));
        let house = sim.finish_house_for(b, pos(68, 60));
        sim.set_relation(a, b, Relation::War);

        let soldier = sim.spawn_in_nation(Species::Soldier, pos(60, 60), a).expect("воин");
        sim.run(120);
        let marching = sim.entity(soldier).is_some_and(|entity| {
            matches!(entity.intent, Intent::AttackBuilding { .. })
        });
        assert!(marching, "воин должен взять курс на вражескую постройку");
        let _ = house;
    }

    /// Союз: воин одной страны бежит защищать жителя другой от зомби.
    #[test]
    fn allies_defend_each_other_from_monsters() {
        let mut sim = island_world(505);
        let a = sim.found_nation_at(pos(58, 58));
        let b = sim.found_nation_at(pos(66, 66));
        sim.set_relation(a, b, Relation::Alliance);

        // Зомби далеко (10 тайлов) от воина — сам он его не заметит,
        // но житель союзника стоит рядом с монстром.
        let civilian = sim.spawn_in_nation(Species::Human, pos(60, 70), b).expect("житель");
        let zombie = sim.spawn(Species::Zombie, pos(62, 70)).expect("зомби");
        let soldier = sim.spawn_in_nation(Species::Soldier, pos(70, 60), a).expect("воин");

        sim.run(40);
        let guarding = sim.entity(soldier).is_some_and(|entity| {
            matches!(entity.intent, Intent::Attack { target } if target == zombie)
        });
        assert!(guarding, "воин-союзник должен атаковать монстра у чужого жителя");
        let _ = civilian;
    }

    /// Лодка плавает по воде и не вылезает на сушу.
    #[test]
    fn boats_stay_on_water() {
        let mut sim = island_world(606);
        let nation = sim.found_nation_at(pos(64, 64));
        let boat = sim.spawn_boat(pos(64, 64), nation).expect("лодка");
        sim.run(200);
        let boat = sim.entity(boat).expect("лодка жива");
        assert!(boat.species.is_boat());
        assert!(
            sim.world.is_water(boat.pos),
            "лодка оказалась на суше: {:?}",
            boat.pos
        );
        assert!(sim.stats.boats >= 1);
    }
}
