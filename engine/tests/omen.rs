mod support;
use support::*;
use valorant_horizon_engine::{
    agents::omen::Omen,
    gameplay::{geometry::*, objects::*, *},
    server::Session,
};
#[test]
fn shrouded_step_channels_then_moves_and_stays_in_loaded_clear_space() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let mut omen = Omen::default();
    omen.activate(0, &a, &w, &mut vec![]);
    assert!(omen.fire(false, &a, &w, &mut vec![], &mut 0));
    assert_eq!(w.actors[0].position, [0.0, 1.0, 0.0]);
    for _ in 0..8 {
        let a = w.actors[0].clone();
        omen.tick(&a, &mut w, &[]);
    }
    assert!(w.actors[0].position[2] > 7.0);
    assert_eq!(omen.slots[0].charges, 1);
}
#[test]
fn teleport_destination_is_stopped_before_obstructing_wall() {
    let w = TestWorld {
        wall: Some(Box3 {
            min: [-2.0, 0.0, 4.0],
            max: [2.0, 5.0, 4.1],
        }),
        ..Default::default()
    };
    let a = w.actors[0].clone();
    let target = teleport::target(&w, &a, 8.0).unwrap();
    assert!(target[2] < 4.0);
    assert!(clear_box(&w, a.bounds.translated(sub(target, a.position))));
}
#[test]
fn destroyed_shade_cancels_ultimate_and_manual_cancel_never_teleports() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let mut omen = Omen::default();
    let mut objects = vec![];
    omen.activate(3, &a, &w, &mut vec![]);
    assert!(omen.fire(false, &a, &w, &mut objects, &mut 0));
    let at = objects[0].position;
    let enemy = actor(2, add(at, [0.0, 0.0, -2.0]));
    assert!(strike(&mut objects, &enemy, &w));
    omen.tick(&a, &mut w, &objects);
    assert!(omen.cast.is_none());
    assert_eq!(w.actors[0].position, a.position);
    assert_eq!(omen.slots[3].charges, 0);
}
#[test]
fn dark_cover_moves_marker_and_passes_wall_before_becoming_smoke() {
    let mut w = TestWorld {
        wall: Some(Box3 {
            min: [-2.0, 0.0, 4.0],
            max: [2.0, 5.0, 4.1],
        }),
        ..Default::default()
    };
    let mut s = Session::default();
    for (sequence, action) in [
        (1, Action::Select(3)),
        (2, Action::Activate(2)),
        (3, Action::Fire(false)),
        (4, Action::Activate(2)),
    ] {
        s.enqueue(
            1,
            Request {
                expected_agent: 3,
                sequence,
                action,
            },
        );
    }
    for _ in 0..12 {
        s.tick(&mut w);
    }
    assert_eq!(s.smokes.len(), 1);
    assert!(s.smokes[0].center[2] > 16.0);
    assert_eq!(s.smokes[0].color, [65, 45, 100, 255]);
}
#[test]
fn paranoia_hits_once_through_wall_and_restricts_real_player_vision() {
    let mut w = TestWorld::default();
    w.actors[1] = actor(2, [0.0, 1.0, 6.0]);
    w.wall = Some(Box3 {
        min: [-2.0, 0.0, 3.0],
        max: [2.0, 5.0, 3.2],
    });
    let mut s = Session::default();
    s.enqueue(
        1,
        Request {
            expected_agent: 3,
            sequence: 1,
            action: Action::Select(3),
        },
    );
    s.enqueue(
        1,
        Request {
            expected_agent: 3,
            sequence: 2,
            action: Action::Activate(1),
        },
    );
    for _ in 0..12 {
        s.tick(&mut w);
    }
    assert!(s.vision[&2].nearsight > 0);
    assert_eq!(
        w.effects
            .iter()
            .filter(|(target, _)| *target == Entity::Player(2))
            .count(),
        1
    );
}
#[test]
fn stale_agent_request_cannot_trigger_different_agents_ability() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.enqueue(
        1,
        Request {
            expected_agent: 3,
            sequence: 1,
            action: Action::Select(3),
        },
    );
    s.enqueue(
        1,
        Request {
            expected_agent: 0,
            sequence: 2,
            action: Action::Activate(1),
        },
    );
    s.tick(&mut w);
    assert!(s.projectiles.is_empty());
    assert_eq!(s.players[&1].omen.slots[1].charges, 1);
}
