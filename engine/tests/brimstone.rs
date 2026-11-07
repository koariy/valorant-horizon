mod support;
use support::*;
use valorant_horizon_engine::{
    agents::brimstone::*,
    gameplay::{geometry::*, zones::*, *},
    server::Session,
};
#[test]
fn smoke_marks_are_confirmed_atomically_and_cancelled_without_spending() {
    let mut w = TestWorld::default();
    let mut a = w.actors[0].clone();
    a.direction = unit([0., -0.2, 1.]);
    let mut b = Brimstone::default();
    let (mut p, mut z, mut s) = (vec![], vec![], vec![]);
    let mut c = Context {
        world: &mut w,
        projectiles: &mut p,
        zones: &mut z,
        smokes: &mut s,
    };
    assert!(b.activate(2, &a));
    assert!(b.fire(false, &a, &mut c));
    assert_eq!(b.slots[2].charges, 3);
    b.cancel();
    assert!(b.marks.is_empty());
    b.activate(2, &a);
    b.fire(false, &a, &mut c);
    assert!(b.fire(true, &a, &mut c));
    assert_eq!(c.smokes.len(), 1);
    assert_eq!(b.slots[2].charges, 2);
}
#[test]
fn orbital_deals_real_damage_and_expires() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.zones.push(Zone {
        owner: 1,
        kind: ZoneKind::Orbital,
        bounds: Box3 {
            min: [-4., 0., -4.],
            max: [4., 64., 4.],
        },
        ticks: 20,
    });
    s.tick(&mut w);
    for _ in 0..19 {
        s.tick(&mut w);
    }
    assert!(w.actors[1].health < 20.);
    assert!(s.zones.is_empty());
    assert!(w.actors[0].health < 20.);
}
#[test]
fn stim_buff_is_not_damage_and_is_removed_with_zone() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.zones.push(Zone {
        owner: 1,
        kind: ZoneKind::Stim,
        bounds: Box3 {
            min: [-4., 0., -4.],
            max: [4., 4., 4.],
        },
        ticks: 11,
    });
    s.tick(&mut w);
    assert!(w.effects.iter().any(|(_, e)| e == "minecraft:haste"));
    assert!(w.damage.is_empty());
    for _ in 0..10 {
        s.tick(&mut w);
    }
    assert!(s.zones.is_empty());
}
#[test]
fn incendiary_lands_and_does_not_heal_owner() {
    let mut w = TestWorld::default();
    w.actors[0].health = 10.;
    let mut s = Session::default();
    s.projectiles.push(projectiles::Projectile {
        owner: 1,
        kind: projectiles::ProjectileKind::Incendiary { duration: 30 },
        position: [0., 2., 0.],
        velocity: [0., -0.2, 0.],
        ticks: 80,
        guided: false,
        victims: vec![],
    });
    for _ in 0..20 {
        s.tick(&mut w);
    }
    assert!(s.projectiles.is_empty());
    assert!(w.actors[0].health < 10.);
}
#[test]
fn selection_snapshot_and_offline_cooldown_are_integrated() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.enqueue(
        1,
        Request {
            sequence: 1,
            expected_agent: 0,
            action: Action::Select(6),
        },
    );
    s.tick(&mut w);
    assert_eq!(s.snapshot(1).unwrap().agent, 6);
    let p = s.players.get_mut(&1).unwrap();
    p.brimstone.slots[0].remaining = 2;
    p.brimstone.slots[0].charges = 0;
    w.actors.clear();
    s.tick(&mut w);
    s.tick(&mut w);
    assert_eq!(s.players[&1].brimstone.slots[0].charges, 1);
}
#[test]
fn capacity_failure_does_not_spend_charges() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let mut b = Brimstone {
        equipped: Some(2),
        marks: vec![a.position],
        ..Default::default()
    };
    let (mut p, mut z) = (vec![], vec![]);
    let mut smoke = vec![
        smoke::Smoke {
            owner: 1,
            center: a.position,
            radius: 3.,
            ticks: 30,
            color: [255; 4]
        };
        MAX_SMOKES
    ];
    assert!(!b.fire(
        true,
        &a,
        &mut Context {
            world: &mut w,
            projectiles: &mut p,
            zones: &mut z,
            smokes: &mut smoke
        }
    ));
    assert_eq!(b.slots[2].charges, 3);
}
