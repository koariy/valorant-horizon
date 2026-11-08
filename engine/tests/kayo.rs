extern crate self as horizon_client;
mod support;
use support::{extra::Scene, *};
use valorant_horizon_engine::{
    advanced::{Agent, Expansion, kayo::*},
    gameplay::{geometry::*, objects::*, *},
    server::{Player, Session},
};
fn step(k: &mut Kayo, c: &mut Scene) {
    let a = c.w.actors[0].clone();
    c.step(k, &a);
    c.w.protect(1, k.protection(&a));
    for o in &mut c.objects {
        o.ticks = o.ticks.saturating_sub(1);
    }
    c.objects
        .retain(|o| o.health > 0. && o.ticks > 0 && k.owns(o.id));
}
fn activate(k: &mut Kayo, c: &mut Scene, slot: usize) -> bool {
    let a = c.w.actors[0].clone();
    k.activate(slot, &a, &mut c.ctx())
}
fn fire(k: &mut Kayo, c: &mut Scene, alt: bool) -> bool {
    let a = c.w.actors[0].clone();
    k.fire(alt, &a, &mut c.ctx())
}
fn down(k: &mut Kayo, c: &mut Scene) {
    assert!(activate(k, c, 3));
    c.w.actors[0].health = 1.;
    let p = c.w.protections.get_mut(&1).unwrap();
    p.health = 1.;
    p.downed = true;
    p.downable = false;
    p.downed_pool = 40.;
    step(k, c);
    assert!(k.downed());
}
#[test]
fn zero_point_sticks_winds_up_and_suppresses_through_wall() {
    let (mut k, mut c) = (Kayo::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 4.]);
    c.w.wall = Some(Box3 {
        min: [-3., 1., 2.],
        max: [3., 5., 2.2],
    });
    assert!(activate(&mut k, &mut c, 2));
    assert!(fire(&mut k, &mut c, false));
    for _ in 0..12 {
        step(&mut k, &mut c);
    }
    assert!(c.controls.is_empty());
    for _ in 0..20 {
        step(&mut k, &mut c);
    }
    assert_eq!(c.controls[&Entity::Player(2)].suppressed, 120);
    assert!(c.objects.is_empty());
    assert!(c.w.damage.is_empty());
}
#[test]
fn destroyed_blade_cannot_pulse_or_reveal_targets() {
    let (mut k, mut c) = (Kayo::default(), Scene::default());
    activate(&mut k, &mut c, 2);
    fire(&mut k, &mut c, false);
    c.objects[0].damage_with(
        2.,
        Some(Entity::Player(2)),
        [1., 2., 0.],
        UtilityCause::Shot,
    );
    for _ in 0..60 {
        step(&mut k, &mut c);
    }
    assert!(c.controls.is_empty());
    assert!(c.intel.snapshot(1).is_empty());
}
#[test]
fn flash_fuse_strength_and_friendly_fire_are_real_status_effects() {
    for (alt, fuse, strength) in [(false, 30, 60), (true, 15, 25)] {
        let (mut k, mut c) = (Kayo::default(), Scene::default());
        c.w.actors[1] = actor(2, [0., 1., 6.]);
        c.w.actors[1].direction = [0., 0., -1.];
        c.w.intel.insert(1, vec![1, 2]);
        activate(&mut k, &mut c, 1);
        fire(&mut k, &mut c, alt);
        c.objects[0].position = [0., 2.5, 3.];
        c.objects[0].velocity = [0.; 3];
        for _ in 0..fuse - 1 {
            step(&mut k, &mut c);
        }
        assert!(c.w.effects.is_empty());
        step(&mut k, &mut c);
        for id in [1, 2] {
            assert!(c.w.effect_details.contains(&(
                Entity::Player(id),
                "minecraft:blindness".into(),
                strength,
                0
            )));
        }
    }
}
#[test]
fn flash_respects_facing_and_new_obstacle() {
    for wall in [false, true] {
        let (mut k, mut c) = (Kayo::default(), Scene::default());
        c.w.actors[1] = actor(2, [0., 1., 6.]);
        if wall {
            c.w.actors[1].direction = [0., 0., -1.];
            c.w.wall = Some(Box3 {
                min: [-3., 1., 4.],
                max: [3., 5., 4.2],
            });
        }
        activate(&mut k, &mut c, 1);
        fire(&mut k, &mut c, true);
        c.objects[0].position = [0., 2.5, 3.];
        c.objects[0].velocity = [0.; 3];
        for _ in 0..15 {
            step(&mut k, &mut c);
        }
        assert!(!c.w.effects.iter().any(|(e, _)| *e == Entity::Player(2)));
    }
}
#[test]
fn fragment_pulses_on_ground_with_falloff_and_self_damage_policy() {
    let (mut k, mut c) = (Kayo::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 0.]);
    c.w.actors.push(actor(3, [2.8, 1., 0.]));
    activate(&mut k, &mut c, 0);
    fire(&mut k, &mut c, false);
    c.objects[0].position = [0., 1.3, 0.];
    c.objects[0].velocity = [0.; 3];
    for _ in 0..4 {
        step(&mut k, &mut c);
    }
    let center =
        c.w.damage
            .iter()
            .find(|(e, _, _)| *e == Entity::Player(2))
            .unwrap()
            .1;
    let edge =
        c.w.damage
            .iter()
            .find(|(e, _, _)| *e == Entity::Player(3))
            .unwrap()
            .1;
    assert!(center > edge);
    assert!(
        c.w.damage
            .iter()
            .all(|(_, _, kind)| kind == "minecraft:explosion")
    );
    assert!(c.w.damage.iter().any(|(e, _, _)| *e == Entity::Player(1)));
    for _ in 0..50 {
        step(&mut k, &mut c);
    }
    assert!(c.objects.is_empty());
    assert!(
        c.w.damage
            .iter()
            .filter(|(e, _, _)| *e == Entity::Player(3))
            .count()
            <= 4
    );
}
#[test]
fn fragment_does_not_damage_through_wall_or_in_unknown_chunks() {
    for unknown in [false, true] {
        let (mut k, mut c) = (Kayo::default(), Scene::default());
        c.w.actors[1] = actor(2, [2., 1., 1.]);
        activate(&mut k, &mut c, 0);
        fire(&mut k, &mut c, false);
        c.objects[0].position = [0., 1.3, 1.];
        c.objects[0].velocity = [0.; 3];
        if unknown {
            c.w.unloaded_z = Some(1);
        } else {
            c.w.wall = Some(Box3 {
                min: [1., 1., -2.],
                max: [1.2, 5., 5.],
            });
        }
        for _ in 0..50 {
            step(&mut k, &mut c);
        }
        assert!(!c.w.damage.iter().any(|(e, _, _)| *e == Entity::Player(2)));
    }
}
#[test]
fn overload_pulses_repeat_and_grant_native_stim_without_invulnerability() {
    let (mut k, mut c) = (Kayo::default(), Scene::default());
    k.settings.pulse_interval = 10;
    activate(&mut k, &mut c, 3);
    step(&mut k, &mut c);
    assert!(c.controls.contains_key(&Entity::Player(2)));
    assert!(c.w.effects.iter().any(|(_, e)| e == "minecraft:haste"));
    assert!(!k.protection(&c.w.actors[0]).invulnerable);
    assert!(k.protection(&c.w.actors[0]).downable);
    c.controls.clear();
    for _ in 0..10 {
        step(&mut k, &mut c);
    }
    assert!(!c.controls.is_empty());
}
#[test]
fn downed_pins_real_body_disarms_and_rejects_self_abilities() {
    let (mut k, mut c) = (Kayo::default(), Scene::default());
    down(&mut k, &mut c);
    let anchor = c.w.actors[0].position;
    c.w.move_player(1, [2., 1., 2.]);
    step(&mut k, &mut c);
    assert_eq!(c.w.actors[0].position, anchor);
    assert!(c.w.protections[&1].disarmed);
    assert!(!c.w.protections[&1].invulnerable);
    assert!(!activate(&mut k, &mut c, 0));
    assert!(!activate(&mut k, &mut c, 3));
    k.cancel();
    assert!(k.downed());
}
#[test]
fn downed_timeout_and_disconnect_cannot_silently_restore_normal_state() {
    for disconnect in [false, true] {
        let (mut k, mut c) = (Kayo::default(), Scene::default());
        k.settings.downed_ticks = 40;
        down(&mut k, &mut c);
        if disconnect {
            k.leave();
            assert!(matches!(k.overload, Overload::PendingFatal));
        }
        for _ in 0..45 {
            step(&mut k, &mut c);
        }
        assert_eq!(c.w.actors[0].health, 0.);
        assert!(!k.downed());
    }
}
#[test]
fn confirmed_self_death_clears_pending_fatal_without_killing_respawn_again() {
    let (mut k, mut c) = (Kayo::default(), Scene::default());
    down(&mut k, &mut c);
    k.leave();
    k.fallen(Entity::Player(1), [0.; 3], false);
    step(&mut k, &mut c);
    assert_eq!(c.w.actors[0].health, 1.);
    assert!(matches!(k.overload, Overload::Off));
}
#[test]
fn suppression_is_bounded_and_phase_immune() {
    let mut c = Scene::default();
    let t = Entity::Player(2);
    c.w.protect(
        2,
        effects::Protection {
            invulnerable: true,
            ..Default::default()
        },
    );
    control::suppress(&mut c.controls, &c.w, t, 100);
    assert!(c.controls.is_empty());
    for id in 3..10000 {
        control::suppress(&mut c.controls, &c.w, Entity::Player(id), u32::MAX);
    }
    assert_eq!(c.controls.len(), control::MAX_CONTROLLED);
    for _ in 0..400 {
        control::tick(&mut c.controls);
    }
    assert!(c.controls.is_empty());
}
#[test]
fn resource_refusal_keeps_all_charges_and_inactive_cleanup_removes_utilities() {
    let (mut k, mut c) = (Kayo::default(), Scene::default());
    for _ in 0..MAX_OBJECTS {
        c.ctx()
            .object(99, ObjectKind::Leer, [3., 2., 0.], 1., 100, [0.; 3]);
    }
    for slot in 0..3 {
        activate(&mut k, &mut c, slot);
        assert!(!fire(&mut k, &mut c, false));
    }
    assert!(!activate(&mut k, &mut c, 3));
    assert_eq!(k.core.slots.map(|r| r.charges), [1, 2, 1, 1]);
    c.objects.clear();
    activate(&mut k, &mut c, 0);
    fire(&mut k, &mut c, false);
    let id = c.objects[0].id;
    k.inactive_tick();
    assert!(!k.owns(id));
    assert_eq!(k.core.slots[0].charges, 0);
}
fn setup_channel() -> (Session, TestWorld) {
    let (mut k, mut c) = (Kayo::default(), Scene::default());
    k.settings.stabilize_ticks = 20;
    k.settings.downed_ticks = 400;
    c.w.actors = vec![actor(2, [0., 1., 2.]), actor(1, [0., 1., 0.])];
    let a = c.w.actors[0].clone();
    assert!(k.activate(3, &a, &mut c.ctx()));
    c.w.actors[0].health = 1.;
    let native = c.w.protections.get_mut(&2).unwrap();
    native.health = 1.;
    native.downable = false;
    native.downed = true;
    native.downed_pool = 40.;
    let a = c.w.actors[0].clone();
    c.step(&mut k, &a);
    c.w.protect(2, k.protection(&a));
    let mut w = c.w;
    w.actors.swap(0, 1);
    w.intel.insert(1, vec![1, 2]);
    w.intel.insert(2, vec![1, 2]);
    let mut s = Session {
        objects: c.objects,
        ..Default::default()
    };
    s.players.insert(1, Player::default());
    s.players.insert(
        2,
        Player {
            agent: 18,
            extra: Expansion {
                kayo: k,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    (s, w)
}
fn hold(s: &mut Session, w: &mut TestWorld, seq: u64, held: bool) {
    s.enqueue(
        1,
        Request {
            expected_agent: 0,
            sequence: seq,
            action: Action::Stabilize(held),
        },
    );
    s.tick(w);
}
#[test]
fn teammate_hold_channel_restores_health_without_respawn_or_free_ult() {
    let (mut s, mut w) = setup_channel();
    let mut sequence = 1;
    for tick in 0..20 {
        if tick % 5 == 0 {
            hold(&mut s, &mut w, sequence, true);
            sequence += 1;
        } else {
            s.tick(&mut w);
        }
    }
    assert_eq!(w.actors[1].health, 10.);
    assert!(!s.players[&2].extra.kayo.downed());
    assert!(!w.protections[&2].downed);
    assert_eq!(s.snapshot(2).unwrap().charges[3], 0);
    assert!(s.stabilizers.is_empty());
}
#[test]
fn channel_interrupts_on_release_motion_damage_target_damage_obstacle_and_lost_lease() {
    for mode in 0..6 {
        let (mut s, mut w) = setup_channel();
        hold(&mut s, &mut w, 1, true);
        assert!(!s.stabilizers.is_empty());
        match mode {
            0 => hold(&mut s, &mut w, 2, false),
            1 => {
                w.move_player(1, [1., 1., 0.]);
                s.tick(&mut w);
            }
            2 => {
                w.protections.get_mut(&1).unwrap().hurt_epoch += 1;
                s.tick(&mut w);
            }
            3 => {
                w.protections.get_mut(&2).unwrap().downed_pool -= 1.;
                s.tick(&mut w);
            }
            4 => {
                w.wall = Some(Box3 {
                    min: [-3., 1., 1.],
                    max: [3., 5., 1.2],
                });
                s.tick(&mut w);
            }
            _ => {
                for _ in 0..13 {
                    s.tick(&mut w);
                }
            }
        }
        assert!(s.stabilizers.is_empty(), "mode {mode}");
        assert_eq!(w.actors[1].health, 1.);
    }
}
#[test]
fn enemies_remote_and_stale_requests_cannot_stabilize() {
    for mode in 0..3 {
        let (mut s, mut w) = setup_channel();
        if mode == 0 {
            w.intel.clear();
        } else if mode == 1 {
            w.move_player(1, [0., 1., -5.]);
        }
        s.enqueue(
            1,
            Request {
                expected_agent: if mode == 2 { 18 } else { 0 },
                sequence: 1,
                action: Action::Stabilize(true),
            },
        );
        s.tick(&mut w);
        assert!(s.stabilizers.is_empty());
        assert_eq!(w.actors[1].health, 1.);
    }
}
#[test]
fn all_existing_agents_reject_suppressed_activations_and_keep_placed_utilities() {
    for agent in 0..19 {
        let mut w = TestWorld::default();
        let mut s = Session::default();
        s.players.insert(
            1,
            Player {
                agent,
                ..Default::default()
            },
        );
        control::suppress(&mut s.controls, &w, Entity::Player(1), 20);
        s.enqueue(
            1,
            Request {
                expected_agent: agent,
                sequence: 1,
                action: Action::Activate(0),
            },
        );
        s.tick(&mut w);
        assert_eq!(
            s.snapshot(1).unwrap().error.as_deref(),
            Some("Abilities suppressed"),
            "agent {agent}"
        );
        assert!(s.snapshot(1).unwrap().suppressed > 0);
    }
    let (mut s, mut w) = setup_channel();
    s.players.get_mut(&2).unwrap().extra.kayo.stabilize();
    s.players.get_mut(&1).unwrap().agent = 12;
    let p = s.players.get_mut(&1).unwrap();
    p.extra.cypher.core.equipped = Some(1);
    control::suppress(&mut s.controls, &w, Entity::Player(1), 20);
    s.tick(&mut w);
    assert_eq!(s.players[&1].extra.cypher.core.equipped, None);
}
pub mod valorant_native_state {
    include!("../../native-state/src/lib.rs");
}
pub mod health {
    pub fn knockback_attacker() -> i32 {
        0
    }
}
pub mod game {
    pub struct Game<'a> {
        pub entity: i32,
        pub marker: std::marker::PhantomData<&'a ()>,
    }
    impl Game<'_> {
        pub fn vh_control_locked(&self) -> bool {
            valorant_horizon_engine::gameplay::effects::actions_locked(self.entity)
        }
    }
}
mod incoming {
    include!("../../helpers/incoming.rs");
    pub fn run(e: i32, k: &str, causing: i32, direct: i32, amount: f32, cancel: &mut bool) {
        valorant_protection(e, k, causing, direct, amount, cancel)
    }
}
mod pre {
    include!("../../helpers/damage_pre.rs");
    pub fn run(e: i32, k: &str, amount: &mut f32) {
        valorant_damage_pre(e, k, amount)
    }
}
mod post {
    include!("../../helpers/damage_post.rs");
    pub fn run(e: i32, amount: f32) {
        valorant_damage_post(0, e, amount)
    }
}
mod heal {
    include!("../../helpers/downed_healing.rs");
    pub fn run(id: u32, cancel: &mut bool) {
        valorant_downed_healing((), id, cancel)
    }
}
mod locked {
    include!("../../helpers/locked_interaction.rs");
    pub fn run(g: &crate::game::Game<'_>, c: &mut bool) {
        valorant_locked_interaction(g, c)
    }
}
#[test]
fn actual_pre_post_helpers_down_once_and_pool_is_consumed_without_sync_refill() {
    let id = 710001;
    let initial = effects::Protection {
        health: 20.,
        down_epoch: 7,
        downable: true,
        ..Default::default()
    };
    effects::protections().lock().unwrap().insert(id, initial);
    let mut amount = 100.;
    pre::run(id, "minecraft:arrow", &mut amount);
    assert_eq!(amount, 19.);
    post::run(id, amount);
    let mut amount = 7.;
    pre::run(id, "minecraft:arrow", &mut amount);
    assert_eq!(amount, 0.);
    post::run(id, amount);
    let p = effects::protections().lock().unwrap()[&id];
    assert!(p.downed);
    assert_eq!(p.health, 1.);
    assert_eq!(p.downed_pool, 33.);
    assert_eq!(p.hurt_epoch, 1);
    let next = effects::merge_protection(Some(p), initial);
    assert!(next.downed);
    assert!(!next.downable);
    assert_eq!(next.downed_pool, 33.);
    effects::protections().lock().unwrap().insert(id, next);
    let mut amount = 33.;
    pre::run(id, "minecraft:arrow", &mut amount);
    assert!(amount >= 1.);
    effects::protections().lock().unwrap().remove(&id);
}
#[test]
fn void_remains_lethal_downed_attacks_are_refused_and_real_helpers_never_unlock_cancellation() {
    let id = 710002;
    effects::protections().lock().unwrap().insert(
        id,
        effects::Protection {
            health: 1.,
            downed: true,
            downed_pool: 40.,
            ..Default::default()
        },
    );
    let mut amount = 100.;
    pre::run(id, "minecraft:fell_out_of_world", &mut amount);
    assert_eq!(amount, 100.);
    let mut cancel = false;
    incoming::run(710003, "minecraft:player_attack", id, id, 4., &mut cancel);
    assert!(cancel);
    let game = game::Game {
        entity: id,
        marker: Default::default(),
    };
    let mut cancel = false;
    locked::run(&game, &mut cancel);
    assert!(cancel);
    let unlocked = game::Game {
        entity: 710003,
        marker: Default::default(),
    };
    let mut cancel = true;
    locked::run(&unlocked, &mut cancel);
    assert!(cancel);
    effects::protections().lock().unwrap().remove(&id);
}
#[test]
fn healing_guard_preserves_existing_cancellation_and_has_no_healing_side_effects() {
    valorant_native_state::register_downed(effects::is_downed);
    let id = 710004;
    effects::protections().lock().unwrap().insert(
        id,
        effects::Protection {
            downed: true,
            ..Default::default()
        },
    );
    let mut cancel = false;
    heal::run(id as u32, &mut cancel);
    assert!(cancel);
    let mut cancel = false;
    heal::run(710005, &mut cancel);
    assert!(!cancel);
    let mut cancel = true;
    heal::run(710005, &mut cancel);
    assert!(cancel);
    effects::protections().lock().unwrap().remove(&id);
}

#[test]
fn releasing_or_finishing_channel_restores_old_kit_interactions() {
    for finish in [false, true] {
        let (mut s, mut w) = setup_channel();
        hold(&mut s, &mut w, 1, true);
        assert!(w.protections[&1].disarmed);
        if finish {
            for t in 1..20 {
                if t % 5 == 0 {
                    hold(&mut s, &mut w, t as u64 + 1, true);
                } else {
                    s.tick(&mut w);
                }
            }
        } else {
            hold(&mut s, &mut w, 2, false);
        }
        assert!(s.stabilizers.is_empty());
        assert!(!w.protections[&1].disarmed);
    }
}
#[test]
fn only_one_ally_can_channel_and_suppressed_players_can_use_normal_interaction() {
    let (mut s, mut w) = setup_channel();
    hold(&mut s, &mut w, 1, true);
    let mut third = actor(3, [0.8, 1., 0.]);
    third.direction = unit([-0.8, 0., 2.]);
    w.actors.push(third);
    w.intel.insert(3, vec![2, 3]);
    s.players.insert(3, Player::default());
    s.enqueue(
        3,
        Request {
            expected_agent: 0,
            sequence: 1,
            action: Action::Stabilize(true),
        },
    );
    s.tick(&mut w);
    assert_eq!(s.stabilizers.len(), 1);
    assert!(s.stabilizers.contains_key(&1));
    let (mut s, mut w) = setup_channel();
    control::suppress(&mut s.controls, &w, Entity::Player(1), 100);
    hold(&mut s, &mut w, 1, true);
    assert_eq!(s.stabilizers.len(), 1);
}
#[test]
fn suppression_keeps_previously_deployed_cage_and_consumed_iso_shield() {
    let mut c = Scene::default();
    let mut kit = valorant_horizon_engine::advanced::cypher::Cypher::default();
    let a = c.w.actors[0].clone();
    assert!(kit.activate(1, &a, &mut c.ctx()));
    let object = c.objects[0].id;
    let mut s = Session {
        objects: c.objects,
        ..Default::default()
    };
    s.players.insert(
        1,
        Player {
            agent: 12,
            extra: Expansion {
                cypher: kit,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    control::suppress(&mut s.controls, &c.w, Entity::Player(1), 100);
    s.tick(&mut c.w);
    assert!(s.objects.iter().any(|o| o.id == object));
    let mut iso = valorant_horizon_engine::advanced::iso::Iso::default();
    iso.shield = true;
    iso.focus = 5;
    iso.suppress();
    assert!(iso.shield);
    assert_eq!(iso.focus, 5);
}
#[test]
fn invalid_stabilization_cannot_start_without_line_of_sight_or_for_self() {
    let (mut s, mut w) = setup_channel();
    w.wall = Some(Box3 {
        min: [-3., 1., 1.],
        max: [3., 5., 1.2],
    });
    hold(&mut s, &mut w, 1, true);
    assert!(s.stabilizers.is_empty());
    s.enqueue(
        2,
        Request {
            expected_agent: 18,
            sequence: 1,
            action: Action::Stabilize(true),
        },
    );
    s.tick(&mut w);
    assert!(s.stabilizers.is_empty());
}
#[test]
fn suppression_disables_jett_floating_passive_without_freezing_cooldown() {
    let mut w = TestWorld::default();
    w.actors[0].ground = false;
    w.actors[0].jump = true;
    w.actors[0].velocity = [0., -0.7, 0.];
    let mut s = Session::default();
    let mut p = Player::default();
    p.jett.slots[0].remaining = 100;
    s.players.insert(1, p);
    control::suppress(&mut s.controls, &w, Entity::Player(1), 20);
    s.tick(&mut w);
    assert_eq!(w.actors[0].velocity[1], -0.7);
    assert_eq!(s.players[&1].jett.slots[0].remaining, 99);
}
#[test]
fn lethal_hit_immediately_followed_by_disconnect_is_not_lost_before_kit_tick() {
    let (mut k, mut c) = (Kayo::default(), Scene::default());
    activate(&mut k, &mut c, 3);
    let mut native = k.protection(&c.w.actors[0]);
    native.downed = true;
    native.downable = false;
    native.downed_pool = 40.;
    effects::protections().lock().unwrap().insert(1, native);
    k.leave();
    assert!(matches!(k.overload, Overload::PendingFatal));
    let (mut other, mut scene) = (Kayo::default(), Scene::default());
    activate(&mut other, &mut scene, 3);
    other.leave();
    assert!(matches!(other.overload, Overload::Off));
    k.fallen(Entity::Player(1), [0.; 3], false);
    k.leave();
    assert!(matches!(k.overload, Overload::Off));
    effects::protections().lock().unwrap().remove(&1);
}
