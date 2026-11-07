mod support;
use support::*;
use valorant_horizon_engine::{
    agents::clove::*,
    gameplay::{projectiles::*, toxin::*, *},
    server::{Player, Session},
};
#[test]
fn pickup_requires_recent_kill_or_damaging_assist_and_expires() {
    let mut w = TestWorld {
        shield: 8.,
        ..Default::default()
    };
    let a = w.actors[0].clone();
    let mut c = Clove::default();
    assert!(!c.activate(0, &a, &mut w));
    c.damage(Entity::Player(2));
    c.fallen(Entity::Player(2), false);
    assert!(c.activate(0, &a, &mut w));
    assert!(w.effects.iter().any(|(_, n)| n == "minecraft:absorption"));
    assert_eq!(c.reward, 0);
    assert!(!c.activate(0, &a, &mut w));
    for _ in 0..160 {
        c.tick(&a, &mut w);
    }
    assert_eq!(c.buff, 0);
    assert_eq!(w.shield, 8.);
    assert!(w.effects.iter().any(|(_, n)| n == "minecraft:speed"));
}
#[test]
fn contribution_window_and_capacity_are_bounded() {
    let mut c = Clove::default();
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    for id in 0..100 {
        c.damage(Entity::Mob(id));
    }
    assert_eq!(c.contributions.len(), 32);
    for _ in 0..200 {
        c.tick(&a, &mut w);
    }
    c.fallen(Entity::Mob(1), false);
    assert_eq!(c.reward, 0);
    c.fallen(Entity::Mob(3), true);
    assert!(c.reward > 0);
    for _ in 0..160 {
        c.tick(&a, &mut w);
    }
    assert_eq!(c.reward, 0);
}
#[test]
fn ruse_after_death_retains_marks_and_confirms_real_smokes() {
    let mut c = Clove::default();
    let mut w = TestWorld::default();
    let mut a = w.actors[0].clone();
    c.tick(&a, &mut w);
    a.eligible = false;
    a.health = 0.;
    c.tick(&a, &mut w);
    let origin = c.death.unwrap();
    assert!(c.activate(2, &a, &mut w));
    assert!(c.nudge(1, 0, &a));
    assert!(!c.nudge(127, 0, &a));
    let (mut p, mut smoke) = (vec![], vec![]);
    let mut x = Context {
        world: &mut w,
        projectiles: &mut p,
        smokes: &mut smoke,
    };
    assert!(c.fire(false, &a, &mut x));
    assert_eq!(c.marks.len(), 1);
    c.tick(&a, x.world);
    assert_eq!(c.equipped, Some(2));
    assert!(c.nudge(1, 0, &a));
    assert!(c.fire(false, &a, &mut x));
    assert!(c.fire(true, &a, &mut x));
    assert_eq!(x.smokes.len(), 2);
    assert_eq!(c.slots[2].charges, 0);
    assert_eq!(c.death, Some(origin));
    assert!(!c.activate(1, &a, x.world));
    assert!(!c.activate(3, &a, x.world));
}
#[test]
fn ruse_capacity_cancel_and_respawn_do_not_refund_spent_charges() {
    let mut c = Clove::default();
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let (mut p, mut smoke) = (vec![], vec![]);
    c.activate(2, &a, &mut w);
    let mut x = Context {
        world: &mut w,
        projectiles: &mut p,
        smokes: &mut smoke,
    };
    assert!(c.fire(false, &a, &mut x));
    c.cancel();
    assert_eq!(c.slots[2].charges, 2);
    c.activate(2, &a, x.world);
    assert!(c.fire(false, &a, &mut x));
    assert!(c.fire(true, &a, &mut x));
    let charge = c.slots[2].charges;
    c.cancel();
    c.leave();
    assert_eq!(c.slots[2].charges, charge);
    assert!(c.death.is_none());
}
#[test]
fn meddle_pulse_is_nonlethal_and_repaid_via_shared_ledger() {
    let mut w = TestWorld::default();
    let a = w.actors[1].clone();
    let mut debts = Default::default();
    assert!(pulse(&mut w, &a, &mut debts, 80., 3));
    assert_eq!(w.actors[1].health, 1.);
    for _ in 0..100 {
        valorant_horizon_engine::gameplay::toxin::tick(&mut w, &[], &mut debts);
    }
    assert!((w.actors[1].health - 20.).abs() < 0.001);
    assert!(debts.is_empty());
}
#[test]
fn meddle_has_fuse_decays_without_damage_and_cleans_up() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.projectiles.push(Projectile {
        owner: 1,
        kind: ProjectileKind::Meddle,
        position: [0., 2., 0.],
        velocity: [0.; 3],
        ticks: 2,
        guided: false,
        victims: vec![],
    });
    s.tick(&mut w);
    assert_eq!(w.actors[1].health, 20.);
    s.tick(&mut w);
    assert_eq!(w.actors[1].health, 12.);
    assert!(w.damage.is_empty());
    assert!(!s.decay.is_empty());
    for _ in 0..110 {
        s.tick(&mut w);
    }
    assert!(s.projectiles.is_empty());
    assert!(w.actors[1].health > 12.);
}
#[test]
fn dead_player_can_only_issue_clove_utility_commands() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 10,
            ..Default::default()
        },
    );
    s.tick(&mut w);
    w.actors[0].eligible = false;
    w.actors[0].health = 0.;
    s.tick(&mut w);
    assert!(s.snapshot(1).unwrap().after_death);
    for (seq, action) in [
        Action::Select(0),
        Action::Activate(1),
        Action::Activate(2),
        Action::Aim(1, 0),
        Action::Fire(false),
        Action::Fire(true),
    ]
    .into_iter()
    .enumerate()
    {
        s.enqueue(
            1,
            Request {
                sequence: seq as u64 + 1,
                expected_agent: 10,
                action,
            },
        );
        s.tick(&mut w);
    }
    assert_eq!(s.players[&1].agent, 10);
    assert_eq!(s.smokes.len(), 1);
    w.actors[0].eligible = true;
    w.actors[0].health = 20.;
    s.tick(&mut w);
    assert!(!s.snapshot(1).unwrap().after_death);
}
#[test]
fn resurrection_is_explicitly_disabled_and_selection_is_registered() {
    let c = Clove::default();
    assert_eq!(c.slots[3].charges, 0);
    let mut s = valorant_horizon_engine::State::default();
    s.restore("clove");
    assert_eq!(s.selected, 10);
    assert!(
        s.lines(&Default::default())
            .iter()
            .any(|l| l.contains("Not Dead Yet (unavailable)"))
    );
}
#[test]
fn kill_credit_deduplicates_pickup_and_removes_assist() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 10,
            ..Default::default()
        },
    );
    s.players
        .get_mut(&1)
        .unwrap()
        .clove
        .damage(Entity::Player(2));
    w.actors[1].health = 0.;
    w.actors[1].eligible = false;
    s.queue_death(9, Entity::Player(2), [0.; 3], false);
    s.queue_death(9, Entity::Player(2), [0.; 3], false);
    s.tick(&mut w);
    assert!(s.players[&1].clove.reward > 0);
    assert!(s.players[&1].clove.contributions.is_empty());
}

#[test]
fn spectator_transition_clears_death_targeting() {
    let mut w = TestWorld::default();
    let mut c = Clove::default();
    let mut a = w.actors[0].clone();
    c.tick(&a, &mut w);
    a.eligible = false;
    a.health = 0.;
    c.tick(&a, &mut w);
    assert!(c.death.is_some());
    a.health = 20.;
    c.tick(&a, &mut w);
    assert!(c.death.is_none());
}
#[test]
fn meddle_respects_protected_targets_and_includes_owner() {
    let mut w = TestWorld::default();
    w.protected.push(Entity::Player(2));
    let mut s = Session::default();
    s.projectiles.push(Projectile {
        owner: 1,
        kind: ProjectileKind::Meddle,
        position: [0., 2., 0.],
        velocity: [0.; 3],
        ticks: 1,
        guided: false,
        victims: vec![],
    });
    s.tick(&mut w);
    assert_eq!(w.actors[1].health, 20.);
    assert_eq!(w.actors[0].health, 12.);
}
