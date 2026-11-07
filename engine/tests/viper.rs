mod support;
use support::*;
use valorant_horizon_engine::{
    agents::viper::*,
    gameplay::{toxin::*, *},
    server::{Player, Session},
};
#[test]
fn shared_fuel_and_regeneration() {
    let a = actor(1, [0., 1., 0.]);
    let mut v = Viper {
        cloud: Some(a.eye()),
        screen: Some(a.bounds),
        cloud_on: true,
        screen_on: true,
        placed_ticks: 1000,
        ..Default::default()
    };
    v.tick(&a, 8);
    assert!((v.fuel - 99.8).abs() < 0.01);
    v.fuel = 0.1;
    v.tick(&a, 8);
    assert!(!v.cloud_on && !v.screen_on);
    assert!(!v.activate(1, &a));
    for _ in 0..200 {
        v.tick(&a, 8);
    }
    assert!(v.fuel >= 20.);
    assert!(v.activate(1, &a));
}
#[test]
fn decay_is_nonlethal_reversible_and_does_not_refund_normal_damage() {
    let mut w = TestWorld::default();
    let fields = vec![Field::ball(1, w.actors[1].eye(), 3.)];
    let mut debts = Default::default();
    for _ in 0..100 {
        tick(&mut w, &fields, &mut debts);
    }
    assert!(w.actors[1].health >= 1.);
    assert_eq!(w.actors[0].health, 20.);
    w.hurt(1, Entity::Player(2), 1.);
    for _ in 0..200 {
        tick(&mut w, &[], &mut debts);
    }
    assert!((w.actors[1].health - 19.).abs() < 0.01);
    assert!(debts.is_empty());
}
#[test]
fn cancellation_and_death_remove_fields() {
    let a = actor(1, [0., 1., 0.]);
    let mut v = Viper {
        cloud: Some(a.eye()),
        cloud_on: true,
        placed_ticks: 1000,
        pit: Some(a.eye()),
        pit_ticks: 100,
        ..Default::default()
    };
    assert_eq!(v.tick(&a, 8).len(), 2);
    v.cancel();
    assert!(v.tick(&a, 8).is_empty());
    v.cloud_on = true;
    let mut dead = a;
    dead.eligible = false;
    assert!(v.tick(&dead, 8).is_empty());
}
#[test]
fn pit_collapses_when_owner_leaves_and_capacity_is_bounded() {
    let a = actor(1, [0., 1., 0.]);
    let mut v = Viper {
        pit: Some([100., 1., 100.]),
        pit_ticks: 600,
        ..Default::default()
    };
    for _ in 0..100 {
        v.tick(&a, 8);
    }
    assert!(v.pit.is_none());
    v.pit = Some(a.eye());
    v.pit_ticks = 600;
    assert!(v.tick(&a, 0).is_empty());
}
#[test]
fn snake_bite_is_real_damage_and_slow() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.projectiles.push(projectiles::Projectile {
        owner: 1,
        kind: projectiles::ProjectileKind::SnakeBite,
        position: [1., 2., 1.],
        velocity: [0., -0.4, 0.],
        ticks: 20,
        guided: false,
        victims: vec![],
    });
    for _ in 0..20 {
        s.tick(&mut w);
    }
    assert!(s.projectiles.is_empty());
    assert!(w.actors[1].health < 20.);
    assert!(w.effects.iter().any(|(_, n)| n == "minecraft:slowness"));
}
#[test]
fn fields_snapshot_fuel_and_offline_recovery() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 7,
            ..Default::default()
        },
    );
    let p = s.players.get_mut(&1).unwrap();
    p.viper.fuel = 40.;
    p.viper.cloud = Some(w.actors[0].eye());
    p.viper.placed_ticks = 100;
    p.viper.cloud_on = true;
    s.tick(&mut w);
    assert_eq!(s.snapshot(1).unwrap().toxins.len(), 1);
    assert!(s.snapshot(1).unwrap().fuel.unwrap() < 40.);
    w.actors.clear();
    for _ in 0..100 {
        s.tick(&mut w);
    }
    assert!(s.players[&1].viper.fuel > 40.);
}

#[test]
fn decay_ledger_recovers_after_session_reload() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 7,
            ..Default::default()
        },
    );
    let v = &mut s.players.get_mut(&1).unwrap().viper;
    v.cloud = Some(w.actors[1].eye());
    v.cloud_on = true;
    v.placed_ticks = 500;
    for _ in 0..20 {
        s.tick(&mut w);
    }
    assert!(w.actors[1].health < 20.);
    assert!(!w.decay.is_empty());
    let mut reloaded = Session::default();
    for _ in 0..200 {
        reloaded.tick(&mut w);
    }
    assert!((w.actors[1].health - 20.).abs() < 0.01);
    assert!(w.decay.is_empty());
}

#[test]
fn crossing_a_thin_screen_is_detected() {
    let f = Field {
        owner: 1,
        bounds: geometry::Box3 {
            min: [-3., 1., 2.],
            max: [3., 5., 2.4],
        },
        sphere: false,
    };
    let mut a = actor(2, [0., 1., 4.]);
    a.velocity = [0., 0., 4.];
    assert!(!f.contains(a.eye()));
    assert!(f.crossed(&a));
}

#[test]
fn outside_healing_does_not_bypass_active_decay_cap() {
    let mut w = TestWorld::default();
    let f = vec![Field::ball(1, w.actors[1].eye(), 3.)];
    let mut d = Default::default();
    for _ in 0..60 {
        tick(&mut w, &f, &mut d);
    }
    w.heal(Entity::Player(2), 20.);
    assert_eq!(w.actors[1].health, 20.);
    tick(&mut w, &f, &mut d);
    assert!(w.actors[1].health <= 2.01);
    for _ in 0..200 {
        tick(&mut w, &[], &mut d);
    }
    assert!((w.actors[1].health - 20.).abs() < 0.01);
}
#[test]
fn protected_targets_do_not_receive_decay() {
    let mut w = TestWorld::default();
    w.protected.push(Entity::Player(2));
    let f = vec![Field::ball(1, w.actors[1].eye(), 3.)];
    let mut d = Default::default();
    for _ in 0..20 {
        tick(&mut w, &f, &mut d);
    }
    assert_eq!(w.actors[1].health, 20.);
    assert!(d.is_empty());
}
