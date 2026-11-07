mod support;
use support::*;
use valorant_horizon_engine::gameplay::projectiles::intercept;
use valorant_horizon_engine::{
    agents::sova::*,
    gameplay::{geometry::*, objects::*, projectiles::*, recon::*, shields::*, *},
    server::{Player, Session},
};
#[derive(Default)]
struct Scene {
    w: TestWorld,
    p: Vec<Projectile>,
    o: Vec<Object>,
    n: u64,
    intel: Intel,
    beams: Vec<Beam>,
    bursts: Vec<explosions::Explosion>,
}
impl Scene {
    fn ctx(&mut self) -> Context<'_, TestWorld> {
        Context {
            world: &mut self.w,
            projectiles: &mut self.p,
            objects: &mut self.o,
            next: &mut self.n,
            intel: &mut self.intel,
            beams: &mut self.beams,
            bursts: &mut self.bursts,
        }
    }
}
fn bolt(recon: bool, bounces: u8) -> Projectile {
    Projectile {
        owner: 1,
        kind: ProjectileKind::SovaBolt { recon, bounces },
        position: [0., 2., 0.],
        velocity: [2., 0., 0.],
        ticks: 100,
        guided: false,
        victims: vec![],
    }
}
fn utility(kind: ObjectKind, at: V3) -> Object {
    Object {
        id: 1,
        owner: 1,
        kind,
        position: at,
        health: 8.,
        ticks: 100,
        velocity: [0.; 3],
        target: None,
    }
}
#[test]
fn bow_charge_release_and_bounce_selection_spend_exactly_one_charge() {
    let (mut s, mut c) = (Sova::default(), Scene::default());
    let a = c.w.actors[0].clone();
    assert!(s.activate(1, &a));
    assert!(s.fire(true, &a, &mut c.ctx()));
    assert!(s.fire(true, &a, &mut c.ctx()));
    assert_eq!(s.bounces, 2);
    assert!(s.fire(false, &a, &mut c.ctx()));
    for _ in 0..15 {
        s.tick(&a, &mut c.ctx());
    }
    assert!(s.release(&a, &mut c.ctx()));
    assert_eq!(c.p.len(), 1);
    assert!((length(c.p[0].velocity) - 1.3).abs() < 0.01);
    assert_eq!(s.slots[1].charges, 1);
    assert!(matches!(
        c.p[0].kind,
        ProjectileKind::SovaBolt {
            recon: false,
            bounces: 2
        }
    ));
    assert!(s.release(&a, &mut c.ctx()));
    assert_eq!(c.p.len(), 1);
}
#[test]
fn arrow_pool_full_rejects_release_without_spending() {
    let (mut s, mut c) = (Sova::default(), Scene::default());
    let a = c.w.actors[0].clone();
    s.activate(2, &a);
    s.fire(false, &a, &mut c.ctx());
    c.p = vec![bolt(false, 0); MAX_PROJECTILES];
    assert!(!s.release(&a, &mut c.ctx()));
    assert_eq!(s.slots[2].charges, 1);
    assert!(c.o.is_empty());
}
#[test]
fn two_bounces_reflect_normals_then_explode_on_third_collision() {
    let mut c = Scene::default();
    c.w.actors.clear();
    let mut p = bolt(false, 2);
    c.w.wall = Some(Box3 {
        min: [1., 0., -2.],
        max: [1.2, 5., 2.],
    });
    assert!(arrow(&mut p, &mut c.ctx()));
    assert!(p.velocity[0] < 0.);
    c.w.wall = Some(Box3 {
        min: [-1., 0., -2.],
        max: [-0.8, 5., 2.],
    });
    assert!(arrow(&mut p, &mut c.ctx()));
    assert!(p.velocity[0] > 0.);
    c.w.wall = Some(Box3 {
        min: [1., 0., -2.],
        max: [1.2, 5., 2.],
    });
    assert!(!arrow(&mut p, &mut c.ctx()));
    assert_eq!(c.bursts.len(), 1);
    assert!(c.o.is_empty());
}
#[test]
fn recon_arrow_creates_destructible_emitter_and_unknown_chunk_fails_closed() {
    let mut c = Scene::default();
    c.w.actors.clear();
    c.w.wall = Some(Box3 {
        min: [1., 0., -2.],
        max: [1.2, 5., 2.],
    });
    assert!(!arrow(&mut bolt(true, 0), &mut c.ctx()));
    assert!(matches!(c.o[0].kind, ObjectKind::ReconBolt));
    assert_eq!(c.o[0].health, 1.);
    c.w.wall = None;
    c.w.unloaded_z = Some(1);
    let mut p = bolt(true, 2);
    p.velocity = [0., 0., 2.];
    assert!(!arrow(&mut p, &mut c.ctx()));
    assert_eq!(c.o.len(), 1);
}
#[test]
fn recon_los_and_private_team_contacts_do_not_leak_to_enemies() {
    let mut w = TestWorld::default();
    w.actors.push(actor(3, [-1., 1., 1.]));
    w.intel.insert(1, vec![1, 3]);
    let mut i = Intel::default();
    i.scan(&w, 1, [0., 2., -1.], 24.);
    assert_eq!(i.snapshot(1).len(), 1);
    assert_eq!(i.snapshot(3).len(), 1);
    assert_eq!(i.snapshot(1)[0].entity, 2);
    assert!(i.snapshot(2).is_empty());
    w.intel.insert(1, vec![1]);
    i.tick(&w);
    assert!(i.snapshot(3).is_empty());
    let mut blocked = Intel::default();
    w.wall = Some(Box3 {
        min: [-2., 0., 0.],
        max: [2., 5., 0.2],
    });
    blocked.scan(&w, 1, [0., 2., -1.], 24.);
    assert!(blocked.snapshot(1).is_empty());
}
#[test]
fn contacts_and_tags_expire_and_have_bounded_capacity() {
    let w = TestWorld {
        actors: (1..=100)
            .map(|id| actor(id, [id as f64 * 0.01, 1., 1.]))
            .collect(),
        ..Default::default()
    };
    let mut i = Intel::default();
    i.scan(&w, 1, [0., 2., 0.], 24.);
    assert_eq!(i.snapshot(1).len(), MAX_CONTACTS);
    for id in 2..2 + MAX_TAGS {
        assert!(i.tag(1, Entity::Player(id as u128)));
    }
    assert!(!i.tag(1, Entity::Player(99)));
    for _ in 0..68 {
        i.tick(&w);
    }
    assert!(i.tags.is_empty());
    assert!(i.snapshot(1).is_empty());
}
#[test]
fn destroyed_emitter_never_scans_and_active_one_pulses_then_expires() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    let mut o = utility(ObjectKind::ReconBolt, [0., 2., 0.]);
    o.health = 0.;
    o.ticks = 60;
    s.objects.push(o);
    s.tick(&mut w);
    assert!(s.intel.snapshot(1).is_empty());
    let mut o = utility(ObjectKind::ReconBolt, [0., 2., 0.]);
    o.ticks = 60;
    s.objects.push(o);
    s.tick(&mut w);
    assert!(!s.intel.snapshot(1).is_empty());
    for _ in 0..69 {
        s.tick(&mut w);
    }
    assert!(s.objects.is_empty());
    assert!(s.intel.snapshot(1).is_empty());
}
#[test]
fn drone_moves_camera_and_freezes_vulnerable_body_then_destruction_cancels() {
    let (mut s, mut c) = (Sova::default(), Scene::default());
    let mut a = c.w.actors[0].clone();
    let original = a.position;
    s.activate(0, &a);
    assert!(s.fire(false, &a, &mut c.ctx()));
    a.movement = [0., 0., 1.];
    for _ in 0..5 {
        s.tick(&a, &mut c.ctx());
    }
    assert!(s.view.as_ref().unwrap().position[2] > 2.);
    assert_eq!(c.w.actors[0].position, original);
    assert_eq!(s.slots[0].charges, 0);
    c.o[0].health = 0.;
    s.tick(&a, &mut c.ctx());
    assert!(s.view.is_none());
    assert!(s.anchor.is_none());
    assert_eq!(s.slots[0].charges, 0);
}
#[test]
fn drone_dart_uses_remote_origin_cooldown_and_marks_a_real_hit() {
    let (mut s, mut c) = (Sova::default(), Scene::default());
    let a = c.w.actors[0].clone();
    s.activate(0, &a);
    assert!(s.fire(false, &a, &mut c.ctx()));
    let v = s.view.as_mut().unwrap();
    v.position = [0., 2., 5.];
    v.look([0., 0., 1.]);
    c.w.actors[1] = actor(2, [0., 1., 7.]);
    assert!(s.fire(false, &a, &mut c.ctx()));
    assert!(!s.fire(false, &a, &mut c.ctx()));
    assert!(c.p[0].position[2] > 5.);
    let mut session = Session {
        projectiles: std::mem::take(&mut c.p),
        ..Default::default()
    };
    for _ in 0..3 {
        session.tick(&mut c.w);
    }
    assert!(!session.intel.snapshot(1).is_empty());
    assert!(c.w.damage.is_empty());
}
#[test]
fn fury_has_windup_three_shots_wall_penetration_and_cancel_no_refund() {
    let (mut s, mut c) = (Sova::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 8.]);
    c.w.wall = Some(Box3 {
        min: [-2., 0., 3.],
        max: [2., 5., 3.2],
    });
    let a = c.w.actors[0].clone();
    assert!(s.activate(3, &a));
    assert_eq!(s.slots[3].charges, 1);
    assert!(s.fire(false, &a, &mut c.ctx()));
    assert_eq!(s.slots[3].charges, 0);
    for _ in 0..9 {
        s.tick(&a, &mut c.ctx());
    }
    assert!(c.w.damage.is_empty());
    s.tick(&a, &mut c.ctx());
    assert_eq!(c.w.damage.len(), 1);
    assert_eq!(c.beams.len(), 1);
    assert_eq!(s.fury_shots, 2);
    for _ in 0..10 {
        s.tick(&a, &mut c.ctx());
    }
    assert!(s.activate(3, &a));
    assert_eq!(s.fury_shots, 2);
    s.cancel();
    assert_eq!(s.slots[3].charges, 0);
    assert_eq!(s.fury_shots, 0);
}
#[test]
fn fury_stops_before_unknown_chunks_and_can_fire_only_three_times() {
    let (mut s, mut c) = (Sova::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 12.]);
    c.w.unloaded_z = Some(8);
    let a = c.w.actors[0].clone();
    s.activate(3, &a);
    for _ in 0..3 {
        assert!(s.fire(false, &a, &mut c.ctx()));
        for _ in 0..20 {
            s.tick(&a, &mut c.ctx());
        }
    }
    assert!(c.w.damage.is_empty());
    assert_eq!(c.beams.len(), 3);
    assert!(c.beams.iter().all(|b| b.to[2] < 8.));
    assert!(!s.fire(false, &a, &mut c.ctx()));
}
#[test]
fn utility_raycast_respects_nearest_actor_shield_divide_and_object() {
    let mut w = TestWorld::default();
    w.actors.clear();
    let mut o = vec![utility(ObjectKind::Drone, [0., 1.85, 1.])];
    let mut shields = vec![Shield {
        owner: 1,
        center: [0., 2., 4.],
        radius: 2.,
        health: 10.,
        ticks: 100,
        active: true,
        spent: true,
    }];
    let shot = || Shot {
        from: [0., 2., 0.],
        to: [0., 2., 8.],
        damage: 4.,
        padding: CollisionPadding::default(),
    };
    assert!(intercept(&w, &mut shields, &[], &mut o, shot()));
    assert_eq!(o[0].health, 4.);
    assert_eq!(shields[0].health, 10.);
    o[0].position[2] = 3.;
    assert!(intercept(&w, &mut shields, &[], &mut o, shot()));
    assert_eq!(shields[0].health, 6.);
    assert_eq!(o[0].health, 4.);
    let d = divide::Divide::new([-1., 1., 1.], [1., 1., 1.]).unwrap();
    assert!(intercept(&w, &mut shields, &[d], &mut o, shot()));
    assert_eq!(shields[0].health, 6.);
    assert_eq!(o[0].health, 4.);
    w.actors = vec![actor(2, [0., 1., 0.5])];
    assert!(!intercept(&w, &mut shields, &[], &mut o, shot()));
    assert_eq!(o[0].health, 4.);
}
#[test]
fn snapshot_camera_cancel_and_dimension_change_preserve_cooldowns() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 11,
            ..Default::default()
        },
    );
    for (seq, action) in [Action::Activate(0), Action::Fire(false)]
        .into_iter()
        .enumerate()
    {
        s.enqueue(
            1,
            Request {
                sequence: seq as u64 + 1,
                expected_agent: 11,
                action,
            },
        );
        s.tick(&mut w);
    }
    assert!(s.snapshot(1).unwrap().camera.is_some());
    assert_eq!(s.snapshot(1).unwrap().charges[0], 0);
    s.players.get_mut(&1).unwrap().change_dimension();
    s.tick(&mut w);
    assert!(s.snapshot(1).unwrap().camera.is_none());
    assert!(s.objects.is_empty());
    assert_eq!(s.players[&1].sova.slots[0].charges, 0);
}
#[test]
fn shock_bolt_does_real_radial_damage_including_owner_and_respects_cover() {
    for covered in [false, true] {
        let mut w = TestWorld::default();
        if covered {
            w.wall = Some(Box3 {
                min: [0.45, 0., 0.45],
                max: [0.65, 4., 0.65],
            });
        }
        let mut s = Session::default();
        let mut p = bolt(false, 0);
        p.position = [0., 2., 0.];
        p.velocity = [0., -1., 0.];
        s.projectiles.push(p);
        s.tick(&mut w);
        assert!(
            w.damage
                .iter()
                .any(|(t, d, k)| *t == Entity::Player(1) && *d > 0. && k == "minecraft:explosion")
        );
        assert_eq!(
            w.damage.iter().any(|(t, _, _)| *t == Entity::Player(2)),
            !covered
        );
        assert!(s.projectiles.is_empty());
        assert!(w.blocks.is_empty());
    }
}
#[test]
fn drone_expiry_and_blocked_deployment_do_not_refill_charges() {
    let (mut s, mut c) = (Sova::default(), Scene::default());
    s.settings.drone_ticks = 2;
    let a = c.w.actors[0].clone();
    s.activate(0, &a);
    c.w.wall = Some(Box3 {
        min: [-2., 0., 0.3],
        max: [2., 5., 0.5],
    });
    assert!(!s.fire(false, &a, &mut c.ctx()));
    assert_eq!(s.slots[0].charges, 1);
    c.w.wall = None;
    assert!(s.fire(false, &a, &mut c.ctx()));
    s.tick(&a, &mut c.ctx());
    s.tick(&a, &mut c.ctx());
    assert!(s.view.is_none());
    assert_eq!(s.slots[0].charges, 0);
}
#[test]
fn fury_kills_are_credited_and_protected_targets_are_not_revealed() {
    let mut w = TestWorld::default();
    w.actors[1] = actor(2, [0., 1., 8.]);
    w.actors[1].health = 4.;
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 11,
            ..Default::default()
        },
    );
    for (seq, action) in [Action::Activate(3), Action::Fire(false)]
        .into_iter()
        .enumerate()
    {
        s.enqueue(
            1,
            Request {
                sequence: seq as u64 + 1,
                expected_agent: 11,
                action,
            },
        );
        s.tick(&mut w);
    }
    for _ in 0..11 {
        s.tick(&mut w);
    }
    assert!(s.credited.contains(&Entity::Player(2)));
    let (mut agent, mut c) = (Sova::default(), Scene::default());
    c.w.protected.push(Entity::Player(2));
    let a = c.w.actors[0].clone();
    agent.activate(3, &a);
    agent.fire(false, &a, &mut c.ctx());
    for _ in 0..10 {
        agent.tick(&a, &mut c.ctx());
    }
    assert!(c.w.damage.is_empty());
    assert!(c.intel.snapshot(1).is_empty());
}

#[test]
fn fury_cannot_start_a_windup_that_would_outlive_its_timer() {
    let (mut s, mut c) = (Sova::default(), Scene::default());
    let a = c.w.actors[0].clone();
    s.activate(3, &a);
    assert!(s.fire(false, &a, &mut c.ctx()));
    s.fury_time = 9;
    s.windup = 0;
    s.shot_delay = 0;
    let remaining = s.fury_shots;
    assert!(!s.fire(false, &a, &mut c.ctx()));
    assert_eq!(s.fury_shots, remaining);
    assert_eq!(s.windup, 0);
}
