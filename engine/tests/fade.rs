mod support;
use support::{extra::Scene, *};
use valorant_horizon_engine::{
    advanced::{Agent, Expansion, fade::*},
    gameplay::{divide, geometry::*, objects::*, toxin, *},
    server::{Player, Session},
};
fn cast(k: &mut Fade, c: &mut Scene, slot: usize) {
    let a = c.w.actors[0].clone();
    assert!(k.activate(slot, &a, &mut c.ctx()));
    assert!(k.fire(false, &a, &mut c.ctx()));
}
fn land(k: &mut Fade, c: &mut Scene, slot: usize) {
    cast(k, c, slot);
    let o = c.objects.last_mut().unwrap();
    o.position = [0., 1.3, 3.];
    o.velocity = [0., -0.8, 0.];
    let a = c.w.actors[0].clone();
    c.step(k, &a);
}
#[test]
fn throw_consumes_once_and_reuse_drops_early() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    cast(&mut k, &mut c, 2);
    let a = c.w.actors[0].clone();
    assert_eq!(k.core.slots[2].charges, 0);
    assert!(k.activate(2, &a, &mut c.ctx()));
    assert_eq!(c.objects.len(), 1);
    assert_eq!(c.objects[0].velocity, [0., -0.8, 0.]);
    assert!(!k.fire(false, &a, &mut c.ctx()));
}
#[test]
fn haunt_lands_reveals_and_creates_private_trail() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 7.]);
    land(&mut k, &mut c, 2);
    assert!(matches!(c.objects[0].kind, ObjectKind::Haunt));
    let a = c.w.actors[0].clone();
    for _ in 0..5 {
        c.step(&mut k, &a);
    }
    assert!(!c.intel.snapshot(1).is_empty());
    assert!(c.intel.snapshot(2).is_empty());
    assert!(!k.trails().is_empty());
}
#[test]
fn haunt_cannot_reveal_through_solid_terrain() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 7.]);
    land(&mut k, &mut c, 2);
    c.w.wall = Some(Box3 {
        min: [-2., 0., 4.],
        max: [2., 5., 4.5],
    });
    let a = c.w.actors[0].clone();
    for _ in 0..10 {
        c.step(&mut k, &a);
    }
    assert!(c.intel.snapshot(1).is_empty());
    assert!(k.trails().is_empty());
}
#[test]
fn destroyed_haunt_stops_detecting() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 7.]);
    land(&mut k, &mut c, 2);
    c.objects[0].health = 0.;
    let a = c.w.actors[0].clone();
    for _ in 0..10 {
        c.step(&mut k, &a);
    }
    assert!(c.intel.snapshot(1).is_empty());
    assert_eq!(k.stats()[2], 0);
}
#[test]
fn orb_hitting_wall_falls_instead_of_becoming_a_floating_eye() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    cast(&mut k, &mut c, 2);
    c.w.wall = Some(Box3 {
        min: [-1., 0., 0.8],
        max: [1., 5., 1.],
    });
    let a = c.w.actors[0].clone();
    c.step(&mut k, &a);
    assert!(matches!(c.objects[0].kind, ObjectKind::HauntOrb));
    assert_eq!(c.objects[0].velocity[2], 0.);
    assert_eq!(k.stats()[2], 0);
}
#[test]
fn seize_captures_once_and_corrects_normal_player_movement() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors[1] = actor(2, [1., 1., 3.]);
    land(&mut k, &mut c, 1);
    assert_eq!(k.stats()[1], 1);
    assert!(
        c.zones
            .iter()
            .any(|z| matches!(z.kind, zones::ZoneKind::Seize))
    );
    c.w.actors[1] = actor(2, [6., 1., 3.]);
    let a = c.w.actors[0].clone();
    c.step(&mut k, &a);
    assert_eq!(c.w.actors[1].position, [1., 1., 3.]);
    assert!(c.w.damage.is_empty());
}
#[test]
fn seize_does_not_capture_late_arrivals_or_targets_behind_wall() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 12.]);
    land(&mut k, &mut c, 1);
    c.w.actors[1] = actor(2, [6., 1., 3.]);
    let a = c.w.actors[0].clone();
    c.step(&mut k, &a);
    assert_eq!(c.w.actors[1].position, [6., 1., 3.]);
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 5.]);
    c.w.wall = Some(Box3 {
        min: [-1., 0., 3.8],
        max: [1., 4., 4.1],
    });
    land(&mut k, &mut c, 1);
    c.w.actors[1] = actor(2, [0., 1., 7.]);
    c.step(&mut k, &a);
    assert_eq!(c.w.actors[1].position, [0., 1., 7.]);
}
#[test]
fn teleport_releases_tether_and_expiration_restores_movement() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors[1] = actor(2, [1., 1., 3.]);
    land(&mut k, &mut c, 1);
    c.w.actors[1] = actor(2, [20., 1., 3.]);
    let a = c.w.actors[0].clone();
    c.step(&mut k, &a);
    assert_eq!(c.w.actors[1].position, [20., 1., 3.]);
    for _ in 0..85 {
        c.step(&mut k, &a);
    }
    assert_eq!(k.stats()[1], 0);
}
#[test]
fn prowler_chases_nearsights_and_is_consumed_without_damage() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 7.]);
    cast(&mut k, &mut c, 0);
    let a = c.w.actors[0].clone();
    for _ in 0..40 {
        c.step(&mut k, &a);
    }
    assert!(c.vision.get(&2).is_some_and(|v| v.nearsight > 0));
    assert!(c.objects.iter().all(|o| o.health == 0.));
    assert!(c.w.damage.is_empty());
}
#[test]
fn holding_fire_steers_and_release_preserves_heading() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.protected.push(Entity::Player(2));
    cast(&mut k, &mut c, 0);
    let mut a = c.w.actors[0].clone();
    a.direction = [1., 0., 0.];
    c.step(&mut k, &a);
    assert!(c.objects[0].velocity[0] > 0.2);
    assert!(k.release(&a, &mut c.ctx()));
    a.direction = [-1., 0., 0.];
    c.step(&mut k, &a);
    assert!(c.objects[0].velocity[0] > 0.2);
    assert!(!k.equipped());
}
#[test]
fn prowler_does_not_tunnel_through_wall_or_seek_protected_targets() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 7.]);
    c.w.protected.push(Entity::Player(2));
    c.w.wall = Some(Box3 {
        min: [-12., 0., 2.],
        max: [12., 5., 3.],
    });
    cast(&mut k, &mut c, 0);
    let a = c.w.actors[0].clone();
    for _ in 0..35 {
        c.step(&mut k, &a);
    }
    assert!(c.objects[0].bounds().max[2] <= 2.);
    assert!(c.objects[0].target.is_none());
    assert!(c.vision.is_empty());
}
#[test]
fn destroyed_prowler_can_never_apply_nearsight() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    cast(&mut k, &mut c, 0);
    c.objects[0].health = 0.;
    let a = c.w.actors[0].clone();
    for _ in 0..30 {
        c.step(&mut k, &a);
    }
    assert!(c.vision.is_empty());
    assert_eq!(k.stats()[0], 0);
}
#[test]
fn nightfall_crosses_walls_decays_nonlethally_and_deafens_once() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 5.]);
    c.w.actors[1].health = 4.;
    c.w.wall = Some(Box3 {
        min: [-20., 0., 2.],
        max: [20., 8., 3.],
    });
    cast(&mut k, &mut c, 3);
    let a = c.w.actors[0].clone();
    for _ in 0..15 {
        c.step(&mut k, &a);
    }
    assert_eq!(c.w.actors[1].health, 1.);
    assert_eq!(c.decay.get(&Entity::Player(2)).unwrap().debt, 3.);
    assert!(c.vision.get(&2).is_some_and(|v| v.deafen > 0));
    assert!(c.intel.snapshot(1).is_empty()); // trail is not a global reveal
    assert!(!k.trails().is_empty());
    assert!(c.w.damage.is_empty());
}
#[test]
fn nightfall_respects_height_range_and_protection() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors = vec![
        actor(1, [0., 1., 0.]),
        actor(2, [0., 8., 5.]),
        actor(3, [12., 1., 5.]),
        actor(4, [0., 1., 5.]),
    ];
    c.w.protected.push(Entity::Player(4));
    cast(&mut k, &mut c, 3);
    let a = c.w.actors[0].clone();
    for _ in 0..30 {
        c.step(&mut k, &a);
    }
    assert!(c.decay.is_empty());
    assert!(c.vision.is_empty());
}
#[test]
fn nightfall_stops_at_unknown_chunks_and_requires_decay_storage() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 6.]);
    c.w.unloaded_z = Some(3);
    cast(&mut k, &mut c, 3);
    let a = c.w.actors[0].clone();
    for _ in 0..10 {
        c.step(&mut k, &a);
    }
    assert!(c.decay.is_empty());
    assert_eq!(k.stats()[3], 0);
    let mut ctx = c.ctx();
    ctx.decay = None;
    assert!(!Fade::default().activate(3, &a, &mut ctx));
}
#[test]
fn nightfall_uses_current_health_instead_of_healing_from_stale_cache() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 1.5]);
    cast(&mut k, &mut c, 3);
    let a = c.w.actors[0].clone();
    let mut ctx = c.ctx();
    ctx.world.temporary_health(Entity::Player(2), 4.); // actor cache still has 20 HP
    k.tick(&a, &mut ctx);
    assert_eq!(c.w.actors[1].health, 1.);
    assert_eq!(c.decay.get(&Entity::Player(2)).unwrap().debt, 3.);
}
#[test]
fn decay_repays_only_its_own_debt_after_delay() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 2.]);
    cast(&mut k, &mut c, 3);
    let a = c.w.actors[0].clone();
    c.step(&mut k, &a);
    c.step(&mut k, &a);
    assert_eq!(c.w.actors[1].health, 12.);
    c.w.hurt(1, Entity::Player(2), 3.);
    for _ in 0..200 {
        toxin::tick(&mut c.w, &[], &mut c.decay);
    }
    assert_eq!(c.w.actors[1].health, 17.);
    assert!(c.decay.is_empty());
}
#[test]
fn trail_privacy_includes_only_owner_and_scoreboard_allies() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 7.]);
    land(&mut k, &mut c, 2);
    let a = c.w.actors[0].clone();
    for _ in 0..5 {
        c.step(&mut k, &a);
    }
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 13,
            extra: Expansion {
                fade: k,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    s.players.insert(2, Player::default());
    s.players.insert(3, Player::default());
    s.viewers.insert(1, vec![1, 3]);
    assert!(!s.snapshot(1).unwrap().trails.is_empty());
    assert!(s.snapshot(2).unwrap().trails.is_empty());
    assert!(!s.snapshot(3).unwrap().trails.is_empty());
}
#[test]
fn audio_deafen_attenuates_and_restores_spatial_gain() {
    divide::set_deafened(false);
    let normal = divide::sound_gain([1., 2., 3.]);
    divide::set_deafened(true);
    assert!((divide::sound_gain([1., 2., 3.]) - normal * 0.03).abs() < 0.0001);
    divide::set_deafened(false);
    assert_eq!(divide::sound_gain([1., 2., 3.]), normal);
}
#[test]
fn death_cleans_objects_and_zones_without_refunding_cooldowns() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors[1] = actor(2, [1., 1., 3.]);
    land(&mut k, &mut c, 1);
    cast(&mut k, &mut c, 2);
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 13,
            extra: Expansion {
                fade: k,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    s.objects = c.objects;
    s.zones = c.zones;
    c.w.actors[0].eligible = false;
    c.w.actors[0].health = 0.;
    s.tick(&mut c.w);
    assert!(s.objects.is_empty());
    assert!(s.zones.is_empty());
    assert_eq!(s.players[&1].extra.fade.core.slots[2].charges, 0);
    assert!(s.players[&1].extra.fade.core.slots[2].remaining > 0);
}
#[test]
fn cancellation_keeps_spent_charges_and_cooldown_restores_them() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    cast(&mut k, &mut c, 0);
    k.cancel();
    assert_eq!(k.core.slots[0].charges, 1);
    assert!(!k.core.firing);
    for _ in 0..600 {
        k.core.tick();
    }
    assert_eq!(k.core.slots[0].charges, 2);
}
#[test]
fn capacity_or_unknown_spawn_does_not_consume_charge() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.unloaded_z = Some(0);
    let a = c.w.actors[0].clone();
    k.activate(0, &a, &mut c.ctx());
    assert!(!k.fire(false, &a, &mut c.ctx()));
    assert_eq!(k.core.slots[0].charges, 2);
    c.w.unloaded_z = None;
    for i in 0..MAX_OBJECTS {
        c.objects.push(Object {
            last_hit: None,
            id: i as u64,
            owner: 1,
            kind: ObjectKind::Haunt,
            position: [0., 2., 0.],
            health: 8.,
            ticks: 100,
            velocity: [0.; 3],
            target: None,
        });
    }
    assert!(!k.fire(false, &a, &mut c.ctx()));
    assert_eq!(k.core.slots[0].charges, 2);
}
#[test]
fn nightfall_actor_visits_are_bounded_in_dense_world() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors
        .extend((10..10010).map(|id| actor(id, [0., 1., 2.])));
    cast(&mut k, &mut c, 3);
    let a = c.w.actors[0].clone();
    c.step(&mut k, &a);
    assert!(c.vision.len() <= 32);
    assert!(c.budget.actors > 0);
    assert!(k.trails().len() <= 2);
}

#[test]
fn revealed_trail_allows_prowler_to_lock_behind_wall_then_expires() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 7.]);
    land(&mut k, &mut c, 2);
    let a = c.w.actors[0].clone();
    for _ in 0..5 {
        c.step(&mut k, &a);
    }
    c.objects[0].health = 0.;
    c.w.wall = Some(Box3 {
        min: [-10., 0., 4.],
        max: [10., 5., 4.5],
    });
    cast(&mut k, &mut c, 0);
    c.step(&mut k, &a);
    assert_eq!(c.objects.last().unwrap().target, Some(Entity::Player(2)));
    c.w.actors[1] = actor(2, [3., 1., 7.]);
    for _ in 0..4 {
        c.step(&mut k, &a);
    }
    assert_eq!(k.trails()[0].to, [3., 1., 7.]);
    for _ in 0..240 {
        c.step(&mut k, &a);
    }
    assert!(k.trails().is_empty());
}
#[test]
fn prowler_can_step_onto_low_obstacle_without_crossing_tall_solid() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.protected.push(Entity::Player(2));
    cast(&mut k, &mut c, 0);
    c.objects[0].position = [0., 1., 0.6];
    c.objects[0].velocity = [0., 0., 0.3];
    c.w.wall = Some(Box3 {
        min: [-2., 0., 1.],
        max: [2., 1.4, 2.],
    });
    let a = c.w.actors[0].clone();
    c.step(&mut k, &a);
    assert!(c.objects[0].position[1] >= 1.49);
    assert!(clear_box(&c.w, c.objects[0].bounds()));
}

#[test]
fn utility_mesh_has_fixed_cost_finite_vertices_and_rejects_invalid_input() {
    let mesh = smoke::orb([3., 4., 5.], 0.3);
    assert_eq!(mesh.len(), 16);
    assert!(mesh.iter().flatten().flatten().all(|v| v.is_finite()));
    assert!(smoke::orb([f64::NAN, 0., 0.], 0.3).is_empty());
    assert!(smoke::orb([0.; 3], f64::INFINITY).is_empty());
}

#[test]
fn short_semantic_teleport_releases_seize_while_dash_is_still_tethered() {
    let (mut k, mut c) = (Fade::default(), Scene::default());
    c.w.actors[1] = actor(2, [1., 1., 3.]);
    land(&mut k, &mut c, 1);
    let target = c.w.actors[1].clone();
    assert!(teleport::safe(&mut c.w, &target, [5., 1., 3.]));
    let a = c.w.actors[0].clone();
    c.step(&mut k, &a);
    assert_eq!(c.w.actors[1].position, [5., 1., 3.]);
    c.w.move_player(2, [6., 1., 3.]);
    c.step(&mut k, &a);
    assert_eq!(c.w.actors[1].position, [6., 1., 3.]);
}
#[test]
fn teleport_event_cursors_are_scoped_to_world_dimension_and_player() {
    let key = (777, "minecraft:overworld".to_owned(), 4242);
    let other = (778, "minecraft:overworld".to_owned(), 4242);
    let nether = (777, "minecraft:the_nether".to_owned(), 4242);
    let before = teleport::stamp(&key);
    teleport::record(key.clone());
    assert_ne!(teleport::stamp(&key), before);
    assert_eq!(teleport::stamp(&other), 0);
    assert_eq!(teleport::stamp(&nether), 0);
}
