mod support;
use support::*;
use valorant_horizon_engine::{
    agents::phoenix::Phoenix,
    gameplay::{effects::*, geometry::*, zones::*, *},
    server::Session,
};
#[test]
fn run_it_back_returns_and_restores_full_health() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let mut p = Phoenix::default();
    assert!(p.activate(3, &a));
    w.move_player(1, [10.0, 1.0, 0.0]);
    w.actors[0].health = 4.0;
    w.requested = true;
    let a = w.actors[0].clone();
    p.tick(&a, &mut w, &mut vec![]);
    assert_eq!(w.actors[0].position, [0.0, 1.0, 0.0]);
    assert_eq!(w.actors[0].health, 20.0);
    assert!(p.anchor.is_none());
    assert_eq!(p.slots[3].charges, 0);
}
#[test]
fn blocked_anchor_never_places_player_inside_wall() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let mut p = Phoenix::default();
    p.activate(3, &a);
    w.move_player(1, [8.0, 1.0, 0.0]);
    w.wall = Some(Box3 {
        min: [-1.0, 1.0, -1.0],
        max: [1.0, 4.0, 1.0],
    });
    w.requested = true;
    let a = w.actors[0].clone();
    p.tick(&a, &mut w, &mut vec![]);
    assert_eq!(w.actors[0].position, [8.0, 1.0, 0.0]);
}
#[test]
fn final_damage_interception_requests_return_without_death() {
    protections().lock().unwrap().insert(
        98765,
        Protection {
            health: 7.0,
            returning: true,
            ..Default::default()
        },
    );
    let mut damage = 12.0;
    health_damage(98765, &mut damage);
    assert_eq!(damage, 6.0);
    assert!(protections().lock().unwrap()[&98765].requested);
    protections().lock().unwrap().remove(&98765);
}
#[test]
fn own_fire_heals_phoenix_and_damages_other_player() {
    let mut w = TestWorld::default();
    w.actors[0].health = 10.0;
    let mut s = Session::default();
    s.enqueue(
        1,
        Request {
            expected_agent: 2,
            sequence: 1,
            action: Action::Select(2),
        },
    );
    s.tick(&mut w);
    s.zones.push(Zone {
        owner: 1,
        kind: ZoneKind::FirePool,
        bounds: Box3 {
            min: [-2.0, 1.0, -2.0],
            max: [2.0, 2.0, 2.0],
        },
        ticks: 11,
    });
    s.tick(&mut w);
    assert_eq!(w.actors[0].health, 11.0);
    assert_eq!(w.actors[1].health, 18.0);
    assert_eq!(w.damage[0].2, "minecraft:in_fire");
}
#[test]
fn blaze_creates_damaging_wall_and_flash_orb_curves() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let mut p = Phoenix::default();
    let mut projectiles = vec![];
    let mut zones = vec![];
    assert!(p.activate(0, &a));
    assert!(p.fire(false, &a, &mut projectiles));
    p.tick(&a, &mut w, &mut zones);
    assert!(!zones.is_empty());
    assert_eq!(p.slots[0].charges, 0);
    let mut s = Session::default();
    s.enqueue(
        1,
        Request {
            expected_agent: 2,
            sequence: 1,
            action: Action::Select(2),
        },
    );
    s.enqueue(
        1,
        Request {
            expected_agent: 2,
            sequence: 2,
            action: Action::Activate(2),
        },
    );
    s.enqueue(
        1,
        Request {
            expected_agent: 2,
            sequence: 3,
            action: Action::Fire(false),
        },
    );
    s.tick(&mut w);
    assert!(s.projectiles[0].velocity[0] > 0.0);
    for _ in 0..12 {
        s.tick(&mut w);
    }
    assert!(!w.effects.is_empty());
}
