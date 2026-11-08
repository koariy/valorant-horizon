mod support;
use support::*;
use valorant_horizon_engine::{
    agents::raze::{Context, Raze, pack},
    gameplay::{explosions::*, geometry::*, objects::*, projectiles::*},
    server::Session,
};
#[test]
fn blast_pack_launches_owner_without_own_damage() {
    let mut w = TestWorld::default();
    apply(&mut w, &mut vec![], pack(1, [0.0, 1.05, 0.0]));
    assert_eq!(w.actors[0].health, 20.0);
    assert!(w.actors[0].velocity[1] > 0.5);
    assert!(w.actors[1].health < 20.0);
}
#[test]
fn satchel_reactivation_detonates_without_spending_second_charge() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let mut r = Raze::default();
    let (mut p, mut o, mut b, mut next) = (vec![], vec![], vec![], 0);
    let mut c = Context {
        world: &mut w,
        projectiles: &mut p,
        objects: &mut o,
        bursts: &mut b,
        next: &mut next,
    };
    assert!(r.activate(1, &a, &mut c));
    assert_eq!(r.slots[1].charges, 1);
    assert!(r.activate(1, &a, &mut c));
    assert_eq!(r.slots[1].charges, 1);
    assert_eq!(c.bursts.len(), 1);
    assert!(c.projectiles.is_empty());
}
#[test]
fn paint_shells_split_into_four_real_submunitions() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.projectiles.push(Projectile {
        owner: 1,
        kind: ProjectileKind::Grenade { cluster: false },
        position: [0.0, 3.0, 8.0],
        velocity: [0.0; 3],
        ticks: 1,
        guided: false,
        victims: vec![],
    });
    s.tick(&mut w);
    assert_eq!(s.projectiles.len(), 4);
    assert!(
        s.projectiles
            .iter()
            .all(|p| matches!(p.kind, ProjectileKind::Grenade { cluster: true }))
    );
    for _ in 0..12 {
        s.tick(&mut w);
    }
    assert!(s.projectiles.is_empty());
}
#[test]
fn rocket_uses_swept_collision_and_explosion_is_blocked_by_wall() {
    let mut w = TestWorld::default();
    w.actors[1] = actor(2, [0.0, 1.0, 5.0]);
    w.wall = Some(Box3 {
        min: [-3.0, 0.0, 3.0],
        max: [3.0, 5.0, 3.2],
    });
    let mut s = Session::default();
    s.projectiles.push(Projectile {
        owner: 1,
        kind: ProjectileKind::Rocket,
        position: [0.0, 2.6, 1.0],
        velocity: [0.0, 0.0, 8.0],
        ticks: 10,
        guided: false,
        victims: vec![],
    });
    s.tick(&mut w);
    assert!(s.projectiles.is_empty());
    assert_eq!(w.actors[1].health, 20.0);
}
#[test]
fn boom_bot_acquires_visible_target_moves_and_explodes_on_contact() {
    let mut w = TestWorld::default();
    w.actors[1] = actor(2, [0.0, 1.0, 5.0]);
    let mut s = Session::default();
    s.objects.push(Object {
        last_hit: None,
        id: 1,
        owner: 1,
        kind: ObjectKind::BoomBot,
        position: [0.0, 1.02, 1.0],
        velocity: [0.0, 0.0, 0.18],
        health: 6.0,
        ticks: 200,
        target: None,
    });
    for _ in 0..25 {
        s.tick(&mut w);
    }
    assert!(s.objects.is_empty());
    assert!(w.actors[1].health < 20.0);
}
#[test]
fn kills_restore_paint_shells_and_launcher_cancellation_does_not_refund() {
    let mut r = Raze::default();
    r.slots[2].charges = 0;
    r.slots[2].remaining = 700;
    r.kill();
    assert_eq!(r.slots[2].charges, 0);
    r.kill();
    assert_eq!(r.slots[2].charges, 1);
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let (mut p, mut o, mut b, mut next) = (vec![], vec![], vec![], 0);
    let mut c = Context {
        world: &mut w,
        projectiles: &mut p,
        objects: &mut o,
        bursts: &mut b,
        next: &mut next,
    };
    assert!(r.activate(3, &a, &mut c));
    r.cancel();
    assert!(!r.rocket);
    assert_eq!(r.slots[3].charges, 0);
}
