mod support;
use support::*;
use valorant_horizon_engine::{
    agents::reyna::*,
    gameplay::{geometry::*, objects::*, *},
    server::{Player, Session},
};
#[test]
fn cannot_heal_or_phase_without_kill() {
    let w = TestWorld::default();
    let mut r = Reyna::default();
    assert!(!r.activate(1, &w.actors[0], &w));
    assert!(!r.activate(2, &w.actors[0], &w));
    assert_eq!(r.slots[1].charges, 2);
}
#[test]
fn soul_lasts_three_seconds_and_shared_pool_is_not_duplicated() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let mut r = Reyna::default();
    r.kill(a.eye());
    assert!(r.activate(2, &a, &w));
    assert_eq!(r.slots[1].charges, 1);
    assert_eq!(r.slots[2].charges, 1);
    assert!(!r.activate(1, &a, &w));
    r.cancel();
    r.kill(a.eye());
    for _ in 0..60 {
        r.tick(&a, &mut w);
    }
    assert!(r.souls.is_empty());
}
#[test]
fn devour_heals_but_link_does_not_pass_through_wall() {
    let mut w = TestWorld::default();
    w.actors[0].health = 5.;
    let a = w.actors[0].clone();
    let mut r = Reyna::default();
    r.kill([0., 2., 3.]);
    assert!(r.activate(1, &a, &w));
    r.tick(&a, &mut w);
    assert!(w.actors[0].health > 5.);
    w.wall = Some(Box3 {
        min: [-1., 0., 1.],
        max: [1., 5., 2.],
    });
    r.tick(&a, &mut w);
    assert!(r.healing.is_none());
}
#[test]
fn empress_kill_renews_and_autoheal_does_not_consume_orb() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let mut r = Reyna::default();
    assert!(r.activate(3, &a, &w));
    r.tick(&a, &mut w);
    r.kill(a.eye());
    assert_eq!(r.frenzy, 600);
    assert!(r.healing.is_some());
    assert_eq!(r.souls.len(), 1);
    assert_eq!(r.slots[1].charges, 2);
    assert!(r.activate(2, &a, &w));
    assert_eq!(r.slots[1].charges, 2);
    r.tick(&a, &mut w);
    assert!(w.effects.iter().any(|(_, n)| n == "minecraft:invisibility"));
    r.cancel();
    assert_eq!(r.phase, 0);
    assert_eq!(r.frenzy, 0);
    assert_eq!(r.slots[3].charges, 0);
}
#[test]
fn leer_is_destructible_and_only_affects_enemies_looking_at_it() {
    let mut w = TestWorld::default();
    w.actors[1] = actor(2, [0., 1., 4.]);
    w.actors[1].direction = [0., 0., -1.];
    let a = w.actors[0].clone();
    let mut r = Reyna::default();
    let (mut o, mut n) = (vec![], 0);
    assert!(r.activate(0, &a, &w));
    assert!(r.fire(&a, &w, &mut o, &mut n));
    o[0].position = [0., 2., 2.];
    let mut s = Session {
        objects: o,
        ..Default::default()
    };
    s.tick(&mut w);
    assert!(s.vision[&2].nearsight > 0);
    s.vision.clear();
    w.actors[1].direction = [0., 0., 1.];
    s.tick(&mut w);
    assert!(!s.vision.contains_key(&2));
    s.objects[0].health = 4.;
    let mut attacker = a;
    attacker.direction = unit(sub(s.objects[0].position, attacker.eye()));
    assert!(strike(&mut s.objects, &attacker, &w));
    assert!(s.objects.is_empty());
}
#[test]
fn cancelled_death_yields_no_orb_and_duplicate_events_yield_one() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 5,
            ..Default::default()
        },
    );
    s.queue_death(1, Entity::Player(2), [1., 2., 1.], false);
    s.tick(&mut w);
    assert!(s.players[&1].reyna.souls.is_empty());
    w.actors[1].health = 0.;
    w.actors[1].eligible = false;
    for _ in 0..2 {
        s.queue_death(1, Entity::Player(2), [1., 2., 1.], false);
    }
    s.tick(&mut w);
    assert_eq!(s.players[&1].reyna.souls.len(), 1);
}
#[test]
fn overheal_decays_and_dismiss_expires_without_free_charge() {
    let mut w = TestWorld::default();
    let mut r = Reyna::default();
    r.kill(w.actors[0].eye());
    assert!(r.activate(1, &w.actors[0], &w));
    for _ in 0..60 {
        let a = w.actors[0].clone();
        r.tick(&a, &mut w);
    }
    assert!(w.shield > 0. && w.shield <= 10.);
    for _ in 0..400 {
        let a = w.actors[0].clone();
        r.tick(&a, &mut w);
    }
    assert!(w.shield < 0.001);
    r.kill(w.actors[0].eye());
    assert!(r.activate(2, &w.actors[0], &w));
    for _ in 0..40 {
        let a = w.actors[0].clone();
        r.tick(&a, &mut w);
    }
    assert_eq!(r.phase, 0);
    assert_eq!(r.slots[2].active, 0);
    assert_eq!(r.slots[1].charges, 0);
}
