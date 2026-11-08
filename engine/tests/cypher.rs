mod support;
use support::{extra::Scene, *};
use valorant_horizon_engine::{
    advanced::{Agent, Corpse, cypher::*},
    gameplay::{geometry::*, objects::*, *},
    server::{Player, Session},
};
fn hallway(c: &mut Scene) {
    for y in 1..=3 {
        for x in -2..=2 {
            for z in [-2, 2] {
                c.w.blocks.insert([x, y, z], "minecraft:stone".into());
            }
        }
    }
}
fn place_wire(k: &mut Cypher, c: &mut Scene) {
    let a = c.w.actors[0].clone();
    assert!(k.activate(0, &a, &mut c.ctx()));
    assert!(k.fire(false, &a, &mut c.ctx()));
}
#[test]
fn wire_requires_two_surfaces_and_spends_once() {
    let (mut k, mut c) = (Cypher::default(), Scene::default());
    let a = c.w.actors[0].clone();
    k.activate(0, &a, &mut c.ctx());
    assert!(!k.fire(false, &a, &mut c.ctx()));
    assert_eq!(k.core.slots[0].charges, 2);
    hallway(&mut c);
    assert!(k.fire(false, &a, &mut c.ctx()));
    assert_eq!(k.core.slots[0].charges, 1);
    assert!(matches!(c.objects[0].kind, ObjectKind::Trapwire));
    assert!(distance(c.objects[0].position, c.objects[0].velocity) > 2.);
}
#[test]
fn wire_crossing_tethers_reveals_then_concusses() {
    let (mut k, mut c) = (Cypher::default(), Scene::default());
    hallway(&mut c);
    place_wire(&mut k, &mut c);
    c.w.actors[1] = actor(2, [0., 1., 0.]);
    let a = c.w.actors[0].clone();
    for _ in 0..42 {
        c.step(&mut k, &a);
    }
    assert!(!c.intel.snapshot(1).is_empty());
    assert!(c.w.effects.iter().any(|(_, n)| n == "minecraft:weakness"));
    assert!(c.objects.is_empty());
}
#[test]
fn destroyed_wire_never_triggers() {
    let (mut k, mut c) = (Cypher::default(), Scene::default());
    hallway(&mut c);
    place_wire(&mut k, &mut c);
    c.objects[0].health = 0.;
    c.w.actors[1] = actor(2, [0., 1., 0.]);
    let a = c.w.actors[0].clone();
    c.step(&mut k, &a);
    assert!(c.intel.snapshot(1).is_empty());
}
#[test]
fn wire_capsule_does_not_block_empty_diagonal_box() {
    let o = Object {
        last_hit: None,
        id: 1,
        owner: 1,
        kind: ObjectKind::Trapwire,
        position: [0., 2., 0.],
        velocity: [4., 2., 4.],
        health: 1.,
        ticks: 100,
        target: None,
    };
    assert!(o.hit([2., 0., 2.], [2., 4., 2.], 0.08).is_some());
    assert!(o.hit([0., 0., 4.], [0., 4., 4.], 0.08).is_none());
}
#[test]
fn camera_is_destructible_and_view_does_not_move_body() {
    let (mut k, mut c) = (Cypher::default(), Scene::default());
    hallway(&mut c);
    let a = c.w.actors[0].clone();
    k.activate(2, &a, &mut c.ctx());
    assert!(k.fire(false, &a, &mut c.ctx()));
    assert!(k.activate(2, &a, &mut c.ctx()));
    assert!(k.camera().is_some());
    c.step(&mut k, &a);
    assert_eq!(c.w.actors[0].position, a.position);
    assert!(k.protection(&a).disarmed);
    assert!(!k.protection(&a).invulnerable);
    c.objects[0].health = 0.;
    c.step(&mut k, &a);
    assert!(k.camera().is_none());
}
#[test]
fn camera_dart_comes_from_camera_and_has_cooldown() {
    let (mut k, mut c) = (Cypher::default(), Scene::default());
    hallway(&mut c);
    let a = c.w.actors[0].clone();
    k.activate(2, &a, &mut c.ctx());
    k.fire(false, &a, &mut c.ctx());
    k.activate(2, &a, &mut c.ctx());
    assert!(k.fire(false, &a, &mut c.ctx()));
    assert!(c.projectiles[0].position[2] > 1.);
    assert!(!k.fire(false, &a, &mut c.ctx()));
}
#[test]
fn cage_throws_then_activates_a_real_zone_and_slows() {
    let (mut k, mut c) = (Cypher::default(), Scene::default());
    let a = c.w.actors[0].clone();
    assert!(k.activate(1, &a, &mut c.ctx()));
    assert_eq!(c.objects.len(), 1);
    c.objects[0].position = [0., 1., 3.];
    c.objects[0].velocity = [0.; 3];
    let mut aim = a.clone();
    aim.direction = unit(sub(c.objects[0].position, a.eye()));
    assert!(k.activate(1, &aim, &mut c.ctx()));
    assert_eq!(c.zones.len(), 1);
    c.w.actors[1] = actor(2, [0., 1., 3.]);
    c.step(&mut k, &a);
    assert!(c.w.effects.iter().any(|(_, n)| n == "minecraft:slowness"));
    assert!(c.w.damage.is_empty());
}
#[test]
fn theft_requires_real_enemy_corpse_and_reveals_through_walls() {
    let (mut k, mut c) = (Cypher::default(), Scene::default());
    let mut a = c.w.actors[0].clone();
    a.direction = unit(sub([0., 1., 2.], a.eye()));
    assert!(!k.activate(3, &a, &mut c.ctx()));
    c.corpses.push(Corpse {
        target: Entity::Mob(7),
        killer: 1,
        at: [0., 1., 2.],
        ticks: 100,
        used: false,
    });
    c.w.wall = Some(Box3 {
        min: [0.5, 0., 0.5],
        max: [0.7, 4., 0.7],
    });
    assert!(k.activate(3, &a, &mut c.ctx()));
    assert!(!c.intel.snapshot(1).is_empty());
    assert!(c.corpses[0].used);
    assert_eq!(k.core.slots[3].charges, 0);
}
#[test]
fn cancel_does_not_refund_and_recall_requires_proximity() {
    let (mut k, mut c) = (Cypher::default(), Scene::default());
    hallway(&mut c);
    place_wire(&mut k, &mut c);
    k.cancel();
    assert_eq!(k.core.slots[0].charges, 1);
    let mut a = c.w.actors[0].clone();
    a.position = [0., 1., 20.];
    assert!(!k.recall(&a, &mut c.ctx()));
}
#[test]
fn session_registration_covert_wire_and_corpse_cleanup() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 12,
            ..Default::default()
        },
    );
    w.actors[1].health = 0.;
    w.actors[1].eligible = false;
    s.queue_death(1, Entity::Player(2), [0., 1., 2.], false);
    s.tick(&mut w);
    assert_eq!(s.corpses.len(), 1);
    assert_eq!(s.snapshot(1).unwrap().agent, 12);
    for _ in 0..201 {
        s.tick(&mut w);
    }
    assert!(s.corpses.is_empty());
}
#[test]
fn operation_budget_bounds_dense_world_scan() {
    let (mut k, mut c) = (Cypher::default(), Scene::default());
    let mut a = c.w.actors[0].clone();
    a.direction = unit(sub([0., 1., 2.], a.eye()));
    c.corpses.push(Corpse {
        target: Entity::Mob(9),
        killer: 1,
        at: [0., 1., 2.],
        ticks: 100,
        used: false,
    });
    c.w.actors
        .extend((10..10010).map(|id| actor(id, [1., 1., 1.])));
    assert!(k.activate(3, &a, &mut c.ctx()));
    assert!(c.intel.snapshot(1).len() <= 8);
    assert!(c.budget.actors >= 4096 - 40);
}

#[test]
fn entry_normal_at_block_edge_is_not_a_parallel_face() {
    let mut c = Scene::default();
    hallway(&mut c);
    let from = c.w.actors[0].eye();
    let hit = block_collision(&c.w, from, add(from, [0., 0., 6.]), 0.).unwrap();
    assert_eq!(hit.normal, [0., 0., -1.]);
    let hit = block_collision(&c.w, from, add(from, [0., 0., -6.]), 0.).unwrap();
    assert_eq!(hit.normal, [0., 0., 1.]);
}
#[test]
fn unloaded_placement_never_consumes_charge() {
    let (mut k, mut c) = (Cypher::default(), Scene::default());
    c.w.unloaded_z = Some(2);
    let a = c.w.actors[0].clone();
    k.activate(2, &a, &mut c.ctx());
    assert!(!k.fire(false, &a, &mut c.ctx()));
    assert_eq!(k.core.slots[2].charges, 1);
    assert!(c.objects.is_empty());
}
#[test]
fn remote_view_cannot_throw_from_the_stationary_body() {
    let (mut k, mut c) = (Cypher::default(), Scene::default());
    hallway(&mut c);
    let a = c.w.actors[0].clone();
    k.activate(2, &a, &mut c.ctx());
    k.fire(false, &a, &mut c.ctx());
    k.activate(2, &a, &mut c.ctx());
    assert!(!k.activate(1, &a, &mut c.ctx()));
    assert_eq!(k.core.slots[1].charges, 2);
    k.activate(0, &a, &mut c.ctx());
    assert!(k.camera().is_none());
}
#[test]
fn exhausted_visibility_budget_fails_closed() {
    let mut c = Scene::default();
    c.budget.rays = 1;
    assert!(!c.ctx().visible([0., 2., 0.], [8., 2., 8.]));
    c.budget.rays = 128;
    assert!(c.ctx().visible([0., 2., 0.], [8., 2., 8.]));
    assert!(c.budget.rays < 128);
}

#[test]
fn semantic_teleport_releases_wire_without_a_delayed_concussion() {
    let (mut k, mut c) = (Cypher::default(), Scene::default());
    hallway(&mut c);
    place_wire(&mut k, &mut c);
    c.w.actors[1] = actor(2, [0., 1., 0.]);
    let a = c.w.actors[0].clone();
    c.step(&mut k, &a);
    let target = c.w.actors[1].clone();
    assert!(teleport::safe(&mut c.w, &target, [4., 1., 0.]));
    for _ in 0..45 {
        c.step(&mut k, &a);
    }
    assert!(
        !c.w.effects
            .iter()
            .any(|(_, name)| name == "minecraft:weakness")
    );
    assert_eq!(k.stats()[0], 1);
}
