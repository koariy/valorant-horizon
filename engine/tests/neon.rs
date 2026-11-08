mod support;
use support::{extra::Scene, *};
use valorant_horizon_engine::{
    advanced::{Agent, Expansion, neon::*},
    gameplay::{geometry::*, objects::*, *},
    server::{Player, Session},
};
fn activate(k: &mut Neon, c: &mut Scene, slot: usize) {
    let a = c.w.actors[0].clone();
    assert!(k.activate(slot, &a, &mut c.ctx()));
}
#[test]
fn high_gear_drains_while_moving_then_recovers_and_cannot_reset_by_cancel() {
    let (mut k, mut c) = (Neon::default(), Scene::default());
    let mut a = c.w.actors[0].clone();
    a.movement = [0., 0., 1.];
    activate(&mut k, &mut c, 2);
    for _ in 0..20 {
        c.step(&mut k, &a);
    }
    assert_eq!(k.energy, 90.);
    assert!(c.w.actors[0].velocity[2] > 0.4);
    k.cancel();
    assert!(k.busy());
    for _ in 0..50 {
        c.step(&mut k, &a);
    }
    assert_eq!(k.energy, 100.);
    assert!(!k.running);
}
#[test]
fn empty_energy_stops_running_and_rejects_restart_until_regenerated() {
    let (mut k, mut c) = (Neon::default(), Scene::default());
    activate(&mut k, &mut c, 2);
    k.energy = 0.5;
    let mut a = c.w.actors[0].clone();
    a.movement = [0., 0., 1.];
    c.step(&mut k, &a);
    assert!(!k.running);
    assert_eq!(k.energy, 0.);
    assert!(!k.activate(2, &a, &mut c.ctx()));
}
#[test]
fn slide_is_swept_stops_at_wall_and_requires_charge_and_ground() {
    let (mut k, mut c) = (Neon::default(), Scene::default());
    activate(&mut k, &mut c, 2);
    let a = c.w.actors[0].clone();
    c.w.wall = Some(Box3 {
        min: [-2., 0., 1.2],
        max: [2., 4., 2.],
    });
    assert!(k.fire(true, &a, &mut c.ctx()));
    c.step(&mut k, &a);
    assert!(c.w.actors[0].bounds.max[2] < 1.21);
    assert!(!k.slide_charge);
    assert!(!k.fire(true, &a, &mut c.ctx()));
    let mut airborne = a.clone();
    airborne.ground = false;
    k.slide_charge = true;
    k.slide = 0;
    assert!(!k.fire(true, &airborne, &mut c.ctx()));
}
#[test]
fn two_confirmed_kills_restore_slide_without_double_credit() {
    let (mut k, mut c) = (Neon::default(), Scene::default());
    activate(&mut k, &mut c, 2);
    let a = c.w.actors[0].clone();
    k.fire(true, &a, &mut c.ctx());
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 14,
            extra: Expansion {
                neon: k,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    s.credit_kill(1, Entity::Mob(21));
    s.credit_kill(1, Entity::Mob(21));
    assert!(!s.players[&1].extra.neon.slide_charge);
    s.credit_kill(1, Entity::Mob(22));
    assert!(s.players[&1].extra.neon.slide_charge);
}
#[test]
fn overdrive_fires_real_damage_while_held_and_release_stops_channel() {
    let (mut k, mut c) = (Neon::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 6.]);
    activate(&mut k, &mut c, 3);
    let a = c.w.actors[0].clone();
    assert!(k.fire(false, &a, &mut c.ctx()));
    c.step(&mut k, &a);
    assert_eq!(c.w.actors[1].health, 16.);
    assert!(!c.beams.is_empty());
    k.release(&a, &mut c.ctx());
    for _ in 0..20 {
        c.step(&mut k, &a);
    }
    assert_eq!(c.w.actors[1].health, 16.);
}
#[test]
fn overdrive_respects_terrain_protected_actors_and_cosmic_divide() {
    for mode in 0..3 {
        let (mut k, mut c) = (Neon::default(), Scene::default());
        c.w.actors[1] = actor(2, [0., 1., 6.]);
        if mode == 0 {
            c.w.wall = Some(Box3 {
                min: [-1., 0., 3.],
                max: [1., 4., 3.5],
            });
        }
        if mode == 1 {
            c.w.protected.push(Entity::Player(2));
        }
        if mode == 2 {
            c.divides
                .push(divide::Divide::new([-2., 1., 3.], [2., 1., 3.]).unwrap());
        }
        activate(&mut k, &mut c, 3);
        let a = c.w.actors[0].clone();
        k.fire(false, &a, &mut c.ctx());
        c.step(&mut k, &a);
        assert_eq!(c.w.actors[1].health, 20.);
    }
}
#[test]
fn nearest_utility_and_cove_take_damage_before_target() {
    let (mut k, mut c) = (Neon::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 6.]);
    c.objects.push(Object {
        last_hit: None,
        id: 101,
        owner: 2,
        kind: ObjectKind::Haunt,
        position: [0., 2.3, 3.],
        health: 8.,
        ticks: 100,
        velocity: [0.; 3],
        target: None,
    });
    activate(&mut k, &mut c, 3);
    let a = c.w.actors[0].clone();
    k.fire(false, &a, &mut c.ctx());
    c.step(&mut k, &a);
    assert_eq!(c.objects[0].health, 4.);
    assert_eq!(c.w.actors[1].health, 20.);
    c.objects.clear();
    c.shields.push(shields::Shield {
        owner: 2,
        center: [0., 2., 3.],
        radius: 1.,
        health: 4.,
        ticks: 100,
        active: true,
        spent: true,
    });
    for _ in 0..10 {
        c.step(&mut k, &a);
    }
    assert!(!c.shields[0].active);
    assert_eq!(c.w.actors[1].health, 20.);
}
#[test]
fn friendly_body_blocks_beam_without_taking_damage() {
    let (mut k, mut c) = (Neon::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 6.]);
    c.w.actors.push(actor(3, [0., 1., 3.]));
    c.w.protected.push(Entity::Player(3));
    activate(&mut k, &mut c, 3);
    let a = c.w.actors[0].clone();
    k.fire(false, &a, &mut c.ctx());
    c.step(&mut k, &a);
    assert!(c.w.damage.is_empty());
}
#[test]
fn ultimate_kill_refreshes_duration_but_cancel_keeps_spent_charge() {
    let (mut k, mut c) = (Neon::default(), Scene::default());
    activate(&mut k, &mut c, 3);
    k.ultimate = 12;
    k.kill();
    assert_eq!(k.ultimate, 200);
    k.cancel();
    assert_eq!(k.ultimate, 0);
    assert_eq!(k.core.slots[3].charges, 0);
    assert!(k.core.slots[3].remaining > 0);
}
#[test]
fn relay_bolt_is_instant_bounces_once_and_creates_two_delayed_concussions() {
    let (mut k, mut c) = (Neon::default(), Scene::default());
    c.w.actors[1] = actor(2, [1., 1., 3.]);
    activate(&mut k, &mut c, 1);
    assert_eq!(k.core.slots[1].charges, 1);
    assert!(matches!(c.objects[0].kind, ObjectKind::RelayBolt));
    c.objects[0].position = [0., 1.2, 3.];
    c.objects[0].velocity = [0., -0.5, 0.];
    let a = c.w.actors[0].clone();
    c.step(&mut k, &a);
    assert!(c.objects[0].velocity[1] > 0.);
    assert_eq!(c.zones.len(), 1);
    assert!(c.w.effects.is_empty());
    for _ in 0..45 {
        c.step(&mut k, &a);
    }
    assert_eq!(c.zones.len(), 2);
    assert!(c.w.effects.iter().any(|(_, n)| n == "minecraft:weakness"));
    assert!(c.objects[0].health == 0.);
}
#[test]
fn relay_grenade_is_not_an_artificial_shootable_target() {
    let o = Object {
        last_hit: None,
        id: 1,
        owner: 1,
        kind: ObjectKind::RelayBolt,
        position: [0., 2., 3.],
        health: 1.,
        ticks: 80,
        velocity: [0.; 3],
        target: None,
    };
    assert!(o.hit([0., 2., 0.], [0., 2., 6.], 0.1).is_none());
    let mut w = TestWorld::default();
    let mut objects = vec![o];
    explosions::apply(
        &mut w,
        &mut objects,
        explosions::Explosion {
            owner: 1,
            position: [0., 2., 3.],
            radius: 3.,
            damage: 20.,
            push: 0.,
            self_damage: false,
        },
    );
    assert_eq!(objects[0].health, 1.);
}
#[test]
fn fast_lane_creates_two_grounded_real_walls_and_terrain_stops_them() {
    let (mut k, mut c) = (Neon::default(), Scene::default());
    activate(&mut k, &mut c, 0);
    let a = c.w.actors[0].clone();
    assert!(k.fire(false, &a, &mut c.ctx()));
    assert_eq!(c.zones.len(), 16);
    assert!(
        c.zones
            .iter()
            .all(|z| matches!(z.kind, zones::ZoneKind::ElectricWall))
    );
    let (mut k, mut c) = (Neon::default(), Scene::default());
    c.w.wall = Some(Box3 {
        min: [-2., 0., 3.],
        max: [2., 5., 4.],
    });
    activate(&mut k, &mut c, 0);
    assert!(k.fire(false, &a, &mut c.ctx()));
    assert!(c.zones.len() < 16);
}
#[test]
fn electric_walls_damage_only_enemies_and_dont_double_hit_at_overlap() {
    let (mut k, mut c) = (Neon::default(), Scene::default());
    activate(&mut k, &mut c, 0);
    let a = c.w.actors[0].clone();
    k.fire(false, &a, &mut c.ctx());
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 14,
            extra: Expansion {
                neon: k,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    s.zones = c.zones;
    c.w.actors[1] = actor(2, [0.8, 1., 2.]);
    for _ in 0..10 {
        s.tick(&mut c.w);
    }
    assert!(c.w.actors[1].health < 20.);
    assert_eq!(c.w.actors[0].health, 20.);
}
#[test]
fn capacity_refusal_keeps_wall_charge_and_bolt_charge() {
    let (mut k, mut c) = (Neon::default(), Scene::default());
    c.zones = vec![
        zones::Zone {
            owner: 1,
            kind: zones::ZoneKind::Slow,
            ticks: 100,
            bounds: c.w.actors[0].bounds
        };
        MAX_ZONES
    ];
    activate(&mut k, &mut c, 0);
    let a = c.w.actors[0].clone();
    assert!(!k.fire(false, &a, &mut c.ctx()));
    assert_eq!(k.core.slots[0].charges, 1);
    c.w.unloaded_z = Some(0);
    assert!(!k.activate(1, &a, &mut c.ctx()));
    assert_eq!(k.core.slots[1].charges, 2);
}
#[test]
fn inactive_resource_recovery_and_dead_cleanup_do_not_reset_cooldowns() {
    let (mut k, mut c) = (Neon::default(), Scene::default());
    activate(&mut k, &mut c, 2);
    let a = c.w.actors[0].clone();
    k.fire(true, &a, &mut c.ctx());
    k.energy = 20.;
    activate(&mut k, &mut c, 1);
    k.inactive_tick();
    assert!(!k.running);
    assert!(k.busy());
    assert!(k.core.slots[1].remaining > 0);
    for _ in 0..600 {
        k.core.tick();
        k.inactive_tick();
    }
    assert!(k.slide_charge);
    assert_eq!(k.energy, 100.);
}
#[test]
fn exhausted_actor_or_ray_budget_cannot_damage_unknown_nearer_target() {
    let mut c = Scene::default();
    c.w.actors[1] = actor(2, [0., 1., 3.]);
    let a = c.w.actors[0].clone();
    c.budget.actors = 1;
    c.ctx().hitscan(1, a.eye(), add(a.eye(), [0., 0., 10.]), 4.);
    assert!(c.w.damage.is_empty());
    c.budget.actors = 4096;
    c.budget.rays = 0;
    c.ctx().hitscan(1, a.eye(), add(a.eye(), [0., 0., 10.]), 4.);
    assert!(c.w.damage.is_empty());
}
