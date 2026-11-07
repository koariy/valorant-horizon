mod support;
use support::*;
use valorant_horizon_engine::{
    agents::harbor::*,
    gameplay::{geometry::*, shields::*, zones::*, *},
    server::{Player, Session},
};
fn shield() -> Shield {
    Shield {
        owner: 1,
        center: [0., 2., 4.],
        radius: 2.,
        ticks: 240,
        health: 10.,
        active: true,
        spent: true,
    }
}
#[test]
fn shield_blocks_at_boundary_breaks_and_does_not_regenerate() {
    let mut w = TestWorld::default();
    w.actors.clear();
    let mut s = vec![shield()];
    assert!(intercept(&w, &mut s, [0., 2., 0.], [0., 2., 8.], 6.));
    assert_eq!(s[0].health, 4.);
    assert!(intercept(&w, &mut s, [0., 2., 0.], [0., 2., 8.], 6.));
    assert!(!s[0].active);
    assert!(!intercept(&w, &mut s, [0., 2., 0.], [0., 2., 8.], 6.));
    assert_eq!(s[0].ticks, 240);
}
#[test]
fn shield_does_not_steal_hits_before_it_or_behind_terrain() {
    let mut w = TestWorld {
        actors: vec![actor(2, [0., 1., 1.])],
        ..Default::default()
    };
    let mut s = vec![shield()];
    assert!(!intercept(&w, &mut s, [0., 2., 0.], [0., 2., 8.], 6.));
    w.actors.clear();
    w.wall = Some(Box3 {
        min: [-2., 0., 1.],
        max: [2., 5., 1.2],
    });
    assert!(!intercept(&w, &mut s, [0., 2., 0.], [0., 2., 8.], 6.));
    assert_eq!(s[0].health, 10.);
}
#[test]
fn shield_blocks_shots_from_inside_and_does_not_block_inside_targets() {
    let mut w = TestWorld::default();
    w.actors.clear();
    let mut s = vec![shield()];
    assert!(intercept(&w, &mut s, [0., 2., 4.], [0., 2., 8.], 1.));
    w.actors = vec![actor(2, [0., 1., 5.])];
    assert!(!intercept(&w, &mut s, [0., 2., 4.], [0., 2., 8.], 1.));
}
#[test]
fn cove_placement_and_one_time_shield_are_separate() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let mut h = Harbor::default();
    let (mut p, mut s) = (vec![], vec![]);
    let mut c = Context {
        world: &mut w,
        projectiles: &mut p,
        shields: &mut s,
    };
    assert!(h.activate(2, &a, &mut c));
    h.preview = Some([0., 1., 5.]);
    assert!(h.activate(2, &a, &mut c));
    assert!(!c.shields[0].active);
    assert_eq!(h.slots[2].charges, 0);
    assert!(h.activate(2, &a, &mut c));
    assert!(c.shields[0].active);
    assert!(!h.activate(2, &a, &mut c));
}
#[test]
fn high_tide_guides_then_keeps_direction_on_release_and_can_stop() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let mut h = Harbor::default();
    let (mut p, mut s, mut z) = (vec![], vec![], vec![]);
    assert!(h.activate(
        1,
        &a,
        &mut Context {
            world: &mut w,
            projectiles: &mut p,
            shields: &mut s
        }
    ));
    assert!(h.fire(
        false,
        &a,
        &mut Context {
            world: &mut w,
            projectiles: &mut p,
            shields: &mut s
        }
    ));
    h.tick(&a, &mut w, &mut z);
    assert_eq!(z.len(), 1);
    h.guiding = false;
    let mut turned = a.clone();
    turned.direction = [1., 0., 0.];
    h.tick(&turned, &mut w, &mut z);
    assert!(z[1].bounds.min[2] > z[0].bounds.min[2]);
    assert!(h.fire(
        true,
        &a,
        &mut Context {
            world: &mut w,
            projectiles: &mut p,
            shields: &mut s
        }
    ));
    assert!(h.tide.is_none());
}
#[test]
fn water_wall_slows_without_fire_damage_and_expires() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.zones.push(Zone {
        owner: 1,
        kind: ZoneKind::WaterWall,
        bounds: Box3 {
            min: [-3., 0., -3.],
            max: [3., 4., 3.],
        },
        ticks: 2,
    });
    s.tick(&mut w);
    assert!(w.damage.is_empty());
    assert!(w.effects.iter().any(|(_, n)| n == "minecraft:slowness"));
    s.tick(&mut w);
    assert!(s.zones.is_empty());
}
#[test]
fn storm_waits_before_nearsight_and_reckoning_hits_once() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.zones.push(Zone {
        owner: 1,
        kind: ZoneKind::Whirlpool,
        bounds: Box3 {
            min: [-3., 0., -3.],
            max: [3., 4., 3.],
        },
        ticks: 2,
    });
    s.tick(&mut w);
    assert!(w.effects.is_empty());
    s.tick(&mut w);
    assert_eq!(s.vision[&2].nearsight, 60);
    w.effects.clear();
    s.projectiles.push(projectiles::Projectile {
        owner: 1,
        kind: projectiles::ProjectileKind::Reckoning,
        position: [0., 1., 0.],
        velocity: [0., 0., 0.1],
        ticks: 4,
        guided: false,
        victims: vec![],
    });
    for _ in 0..4 {
        s.tick(&mut w);
    }
    assert_eq!(
        w.effects
            .iter()
            .filter(|(_, n)| n == "minecraft:slowness")
            .count(),
        1
    );
    assert!(s.projectiles.is_empty());
}
#[test]
fn cancellation_and_selection_preserve_spent_charges() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let mut h = Harbor::default();
    let (mut p, mut v) = (vec![], vec![]);
    h.activate(
        3,
        &a,
        &mut Context {
            world: &mut w,
            projectiles: &mut p,
            shields: &mut v,
        },
    );
    assert!(h.fire(
        false,
        &a,
        &mut Context {
            world: &mut w,
            projectiles: &mut p,
            shields: &mut v
        }
    ));
    h.cancel();
    assert_eq!(h.slots[3].charges, 0);
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 9,
            harbor: h,
            ..Default::default()
        },
    );
    assert_eq!(s.snapshot(1).unwrap().agent, 9);
    assert_eq!(s.snapshot(1).unwrap().charges[3], 0);
}
#[test]
fn cove_and_wall_have_bounded_lifetimes() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    let mut v = shield();
    v.ticks = 1;
    s.shields.push(v);
    s.tick(&mut w);
    assert!(s.shields.is_empty());
    let mut h = Harbor {
        tide: Some(([0., 1., 0.], [0., 0., 1.], 0)),
        guiding: true,
        ..Default::default()
    };
    let mut dead = w.actors[0].clone();
    dead.eligible = false;
    h.tick(&dead, &mut w, &mut vec![]);
    assert!(h.tide.is_none());
}
#[test]
fn closest_divide_or_shield_receives_the_hit() {
    let mut w = TestWorld::default();
    w.actors.clear();
    let mut s = vec![shield()];
    let mut d = valorant_horizon_engine::gameplay::divide::Divide::new([-2., 1., 1.], [2., 1., 1.])
        .unwrap();
    assert!(intercept_barriers(
        &w,
        &mut s,
        &[d.clone()],
        [0., 2., 0.],
        [0., 2., 8.],
        3.,
        CollisionPadding::default()
    ));
    assert_eq!(s[0].health, 10.);
    d.point[2] = 7.;
    assert!(intercept_barriers(
        &w,
        &mut s,
        &[d.clone()],
        [0., 2., 0.],
        [0., 2., 8.],
        3.,
        CollisionPadding::default()
    ));
    assert_eq!(s[0].health, 7.);
    w.actors = vec![actor(2, [0., 1., 1.])];
    assert!(!intercept_barriers(
        &w,
        &mut s,
        &[d],
        [0., 2., 0.],
        [0., 2., 8.],
        3.,
        CollisionPadding::default()
    ));
    assert_eq!(s[0].health, 7.);
}
#[test]
fn simultaneous_shields_damage_only_nearest_boundary() {
    let mut w = TestWorld::default();
    w.actors.clear();
    let mut farther = shield();
    farther.center[2] = 7.;
    let mut s = vec![farther, shield()];
    assert!(intercept(&w, &mut s, [0., 2., 0.], [0., 2., 12.], 2.));
    assert_eq!(s[0].health, 10.);
    assert_eq!(s[1].health, 8.);
}
#[test]
fn prepared_tide_remains_equipped_until_stopped() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 9,
            ..Default::default()
        },
    );
    s.players.get_mut(&1).unwrap().harbor.tide = Some(([0., 1., 0.], [0., 0., 1.], 0));
    assert!(s.snapshot(1).unwrap().equipped);
    let mut p = vec![];
    let mut shields = vec![];
    let a = w.actors[0].clone();
    let h = &mut s.players.get_mut(&1).unwrap().harbor;
    assert!(h.fire(
        false,
        &a,
        &mut Context {
            world: &mut w,
            projectiles: &mut p,
            shields: &mut shields
        }
    ));
    assert!(h.tide.is_some());
    assert!(h.fire(
        true,
        &a,
        &mut Context {
            world: &mut w,
            projectiles: &mut p,
            shields: &mut shields
        }
    ));
    assert!(!s.snapshot(1).unwrap().equipped);
}

#[test]
fn native_arrow_padding_preserves_a_grazing_target_before_cove() {
    let w = TestWorld {
        actors: vec![actor(2, [0.5, 1., 1.])],
        ..Default::default()
    };
    let mut s = vec![shield()];
    assert!(!intercept_barriers(
        &w,
        &mut s,
        &[],
        [0., 2., 0.],
        [0., 2., 8.],
        3.,
        CollisionPadding {
            actors: 0.3,
            terrain: 0.
        }
    ));
    assert_eq!(s[0].health, 10.);
}
