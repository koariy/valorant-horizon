use valorant_horizon_engine::{
    agents::jett::Jett,
    gameplay::{geometry::*, projectiles::*, *},
    server::Session,
};
struct TestWorld {
    actors: Vec<Actor>,
    wall: Option<Box3>,
    unloaded: bool,
    moves: Vec<V3>,
}
impl TestWorld {
    fn new() -> Self {
        Self {
            actors: vec![
                actor(Entity::Player(1), [0.0, 1.0, 0.0]),
                actor(Entity::Mob(9), [0.0, 1.0, 5.0]),
            ],
            wall: None,
            unloaded: false,
            moves: vec![],
        }
    }
}
fn actor(entity: Entity, p: V3) -> Actor {
    Actor {
        entity,
        position: p,
        bounds: Box3 {
            min: add(p, [-0.3, 0.0, -0.3]),
            max: add(p, [0.3, 1.8, 0.3]),
        },
        velocity: [0.0, -0.5, 0.0],
        direction: [0.0, 0.0, 1.0],
        movement: [0.0; 3],
        health: 20.0,
        max_health: 20.0,
        ground: false,
        jump: false,
        eligible: true,
    }
}
impl World for TestWorld {
    fn actors(&self) -> Vec<Actor> {
        self.actors.clone()
    }
    fn boxes(&self, _: [i32; 3]) -> Option<Vec<Box3>> {
        if self.unloaded {
            None
        } else {
            Some(self.wall.into_iter().collect())
        }
    }
    fn move_player(&mut self, _: u128, p: V3) -> bool {
        let a = &mut self.actors[0];
        a.bounds = a.bounds.translated(sub(p, a.position));
        a.position = p;
        self.moves.push(p);
        true
    }
    fn velocity(&mut self, _: u128, v: V3, _: bool) -> bool {
        self.actors[0].velocity = v;
        true
    }
    fn hurt(&mut self, _: u128, t: Entity, d: f32) -> Hit {
        let a = self.actors.iter_mut().find(|a| a.entity == t).unwrap();
        a.health = (a.health - d).max(0.0);
        Hit {
            accepted: true,
            killed: a.health == 0.0,
        }
    }
}
#[test]
fn updraft_consumes_only_on_valid_space_and_glide_clamps_fall() {
    let mut w = TestWorld::new();
    let mut j = Jett::default();
    let a = w.actors[0].clone();
    assert!(j.activate(1, &a, &mut w, &mut vec![]));
    assert_eq!(w.actors[0].velocity[1], 1.05);
    assert_eq!(j.slots[1].charges, 0);
    w.actors[0].velocity[1] = -0.7;
    w.actors[0].jump = true;
    let a = w.actors[0].clone();
    j.tick(&a, &mut w);
    assert_eq!(w.actors[0].velocity[1], -0.08);
}
#[test]
fn dash_is_two_stage_and_cannot_tunnel_or_enter_unloaded_chunks() {
    let mut w = TestWorld::new();
    w.wall = Some(Box3 {
        min: [-2.0, 0.0, 2.0],
        max: [2.0, 5.0, 2.1],
    });
    let mut j = Jett::default();
    let a = w.actors[0].clone();
    assert!(j.activate(2, &a, &mut w, &mut vec![]));
    assert!(w.moves.is_empty());
    assert!(j.activate(2, &a, &mut w, &mut vec![]));
    for _ in 0..4 {
        let a = w.actors[0].clone();
        j.tick(&a, &mut w);
    }
    assert!(w.actors[0].position[2] <= 1.7);
    let a = w.actors[0].clone();
    w.unloaded = true;
    assert_eq!(sweep(&w, a.bounds, [0.0, 0.0, 5.0]), [0.0; 3]);
}
#[test]
fn charge_recovery_is_sequential_and_cancel_does_not_refund() {
    let mut j = Jett::default();
    let mut w = TestWorld::new();
    let a = w.actors[0].clone();
    let mut p = vec![];
    assert!(j.activate(0, &a, &mut w, &mut p));
    assert!(j.activate(0, &a, &mut w, &mut p));
    assert!(!j.activate(0, &a, &mut w, &mut p));
    for _ in 0..400 {
        j.tick(&a, &mut w);
    }
    assert_eq!(j.slots[0].charges, 1);
    j.cancel();
    assert_eq!(j.slots[0].charges, 1);
    assert_eq!(j.slots[0].remaining, 400);
}
#[test]
fn knife_hits_nearest_entity_headshot_and_single_kill_refills() {
    let mut w = TestWorld::new();
    let mut s = Session::default();
    s.enqueue(
        1,
        Request {
            expected_agent: 0,
            sequence: 1,
            action: Action::Activate(3),
        },
    );
    s.tick(&mut w);
    s.enqueue(
        1,
        Request {
            expected_agent: 0,
            sequence: 2,
            action: Action::Fire(false),
        },
    );
    for _ in 0..3 {
        s.tick(&mut w);
    }
    assert_eq!(w.actors[1].health, 0.0);
    assert_eq!(s.players[&1].jett.knives, 5);
}
#[test]
fn wall_stops_knives_and_alt_kill_does_not_refill() {
    let mut w = TestWorld::new();
    w.wall = Some(Box3 {
        min: [-5.0, 0.0, 2.0],
        max: [5.0, 5.0, 2.1],
    });
    let mut p = Projectile {
        owner: 1,
        kind: ProjectileKind::Knife { alternate: false },
        position: [0.0, 2.6, 0.0],
        velocity: [0.0, 0.0, 12.0],
        ticks: 20,
        guided: false,
        victims: Vec::new(),
    };
    assert!(matches!(p.advance(&w), Some(Impact::Block(_))));
    w.wall = None;
    let mut s = Session::default();
    s.enqueue(
        1,
        Request {
            expected_agent: 0,
            sequence: 1,
            action: Action::Activate(3),
        },
    );
    s.tick(&mut w);
    s.enqueue(
        1,
        Request {
            expected_agent: 0,
            sequence: 2,
            action: Action::Fire(true),
        },
    );
    for _ in 0..3 {
        s.tick(&mut w);
    }
    assert_eq!(s.players[&1].jett.knives, 0);
}
#[test]
fn cloud_steers_and_expires_while_replay_and_bad_slot_do_nothing() {
    let mut w = TestWorld::new();
    let mut s = Session::default();
    s.enqueue(
        1,
        Request {
            expected_agent: 0,
            sequence: 1,
            action: Action::Activate(0),
        },
    );
    s.tick(&mut w);
    w.actors[0].direction = [1.0, 0.0, 0.0];
    s.tick(&mut w);
    assert!(s.projectiles[0].position[0] > 0.0);
    s.enqueue(
        1,
        Request {
            expected_agent: 0,
            sequence: 1,
            action: Action::Activate(0),
        },
    );
    s.enqueue(
        1,
        Request {
            expected_agent: 0,
            sequence: 2,
            action: Action::Activate(99),
        },
    );
    s.tick(&mut w);
    assert_eq!(s.projectiles.len(), 1);
    for _ in 0..35 {
        s.tick(&mut w);
    }
    assert_eq!(s.smokes.len(), 1);
    assert!(s.smokes[0].contains(s.smokes[0].center));
    for _ in 0..50 {
        s.tick(&mut w);
    }
    assert!(s.smokes.is_empty());
}

#[test]
fn kill_credit_deduplicates_and_recharges_tailwind_after_two_kills() {
    let mut w = TestWorld::new();
    let mut s = Session::default();
    s.enqueue(
        1,
        Request {
            expected_agent: 0,
            sequence: 1,
            action: Action::Activate(2),
        },
    );
    s.tick(&mut w);
    assert_eq!(s.players[&1].jett.slots[2].charges, 0);
    s.credit_kill(1, Entity::Mob(90));
    s.credit_kill(1, Entity::Mob(90));
    assert_eq!(s.players[&1].jett.slots[2].charges, 0);
    s.credit_kill(1, Entity::Mob(91));
    assert_eq!(s.players[&1].jett.slots[2].charges, 1);
}
#[test]
fn disconnected_player_keeps_spent_charge_and_active_knives_are_cancelled() {
    let mut w = TestWorld::new();
    let mut s = Session::default();
    s.enqueue(
        1,
        Request {
            expected_agent: 0,
            sequence: 1,
            action: Action::Activate(3),
        },
    );
    s.tick(&mut w);
    w.actors.remove(0);
    s.tick(&mut w);
    assert_eq!(s.players[&1].jett.knives, 0);
    assert_eq!(s.players[&1].jett.slots[3].charges, 0);
    w.actors
        .insert(0, actor(Entity::Player(1), [0.0, 1.0, 0.0]));
    s.enqueue(
        1,
        Request {
            expected_agent: 0,
            sequence: 2,
            action: Action::Activate(3),
        },
    );
    s.tick(&mut w);
    assert_eq!(s.players[&1].jett.knives, 0);
}
#[test]
fn snapshot_round_trips_large_uuid_and_forged_commands_are_rejected() {
    let w = TestWorld::new();
    let mut s = Session::default();
    let mut w = w;
    s.enqueue(
        1,
        Request {
            expected_agent: 0,
            sequence: 1,
            action: Action::Activate(0),
        },
    );
    s.tick(&mut w);
    let mut snapshot = s.snapshot(1).unwrap();
    snapshot.projectiles[0].owner = u128::MAX;
    let bytes = serde_json::to_vec(&snapshot).unwrap();
    let result: Snapshot = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(result.projectiles[0].owner, u128::MAX);
    assert!(
        serde_json::from_str::<Request>(
            r#"{"expected_agent":0,"sequence":2,"action":{"Damage":{"amount":99999}}}"#
        )
        .is_err()
    );
}
