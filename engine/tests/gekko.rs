mod support;
use support::{extra::Scene, *};
use valorant_horizon_engine::{
    advanced::{Agent, Expansion, gekko::*},
    gameplay::{geometry::*, objects::*, *},
    server::{Player, Session},
};
fn step(k: &mut Gekko, c: &mut Scene) {
    control::reset_interactions(&c.controls, &mut c.w);
    control::tick(&mut c.controls);
    let a = c.w.actors[0].clone();
    c.step(k, &a);
    c.w.protect(1, k.protection(&a));
    let actors = c.w.actors();
    control::apply(&c.controls, &mut c.w, &actors);
    for o in &mut c.objects {
        o.ticks = o.ticks.saturating_sub(1);
    }
    c.objects
        .retain(|o| o.health > 0. && o.ticks > 0 && (o.owner != 1 || k.owns(o.id)));
}
fn activate(k: &mut Gekko, c: &mut Scene, s: usize) -> bool {
    let a = c.w.actors[0].clone();
    k.activate(s, &a, &mut c.ctx())
}
fn fire(k: &mut Gekko, c: &mut Scene, alt: bool) -> bool {
    let a = c.w.actors[0].clone();
    k.fire(alt, &a, &mut c.ctx())
}
fn recall(k: &mut Gekko, c: &mut Scene) -> bool {
    let a = c.w.actors[0].clone();
    k.recall(&a, &mut c.ctx())
}
fn cast(k: &mut Gekko, c: &mut Scene, slot: usize) {
    assert!(activate(k, c, slot));
    assert!(fire(k, c, false));
}
fn wing_globule(k: &mut Gekko, c: &mut Scene) {
    c.w.actors[1] = actor(2, [0., 1., 3.]);
    cast(k, c, 1);
    step(k, c);
    assert!(
        c.objects
            .iter()
            .any(|o| matches!(o.kind, ObjectKind::WingmanGlobule))
    );
}
fn collect(k: &mut Gekko, c: &mut Scene) {
    for t in 0..k.settings.reclaim_ticks.clamp(10, 40) {
        if t % 5 == 0 {
            assert!(recall(k, c));
        }
        step(k, c);
    }
}
#[test]
fn wingman_seeks_visible_forward_enemy_concusses_and_becomes_real_globule() {
    let (mut k, mut c) = (Gekko::default(), Scene::default());
    wing_globule(&mut k, &mut c);
    assert!(
        c.w.effects
            .contains(&(Entity::Player(2), "minecraft:slowness".into()))
    );
    assert_eq!(c.w.actors[1].health, 20.);
    assert_eq!(k.stats()[0], 1);
    assert_eq!(k.core.slots[1].charges, 0);
}
#[test]
fn wingman_does_not_stun_through_wall_or_seek_ally_or_passive_creature() {
    for mode in 0..3 {
        let (mut k, mut c) = (Gekko::default(), Scene::default());
        c.w.actors[1] = actor(2, [0., 1., 3.]);
        match mode {
            0 => {
                c.w.wall = Some(Box3 {
                    min: [-3., 0., 2.],
                    max: [3., 5., 2.3],
                })
            }
            1 => {
                c.w.intel.insert(1, vec![1, 2]);
            }
            _ => {
                c.w.actors[1].entity = Entity::Mob(50);
                c.w.non_hostile.push(Entity::Mob(50));
            }
        }
        cast(&mut k, &mut c, 1);
        for _ in 0..5 {
            step(&mut k, &mut c);
        }
        assert!(c.w.effects.is_empty(), "mode {mode}");
    }
}
#[test]
fn wingman_expiry_or_destruction_still_creates_pickup_without_free_charge() {
    for destroyed in [false, true] {
        let (mut k, mut c) = (Gekko::default(), Scene::default());
        k.settings.wing_ticks = 40;
        c.w.move_player(2, [0., 1., 100.]);
        cast(&mut k, &mut c, 1);
        if destroyed {
            c.objects[0].damage(5., Some(Entity::Player(2)), [0., 2., 3.]);
        }
        for _ in 0..42 {
            step(&mut k, &mut c);
        }
        assert!(
            c.objects
                .iter()
                .any(|o| matches!(o.kind, ObjectKind::WingmanGlobule))
        );
        assert_eq!(k.core.slots[1].charges, 0);
    }
}
#[test]
fn spike_alt_action_is_not_faked_in_ordinary_minecraft() {
    let (mut k, mut c) = (Gekko::default(), Scene::default());
    activate(&mut k, &mut c, 1);
    assert!(!fire(&mut k, &mut c, true));
    assert_eq!(k.core.slots[1].charges, 1);
    assert!(c.objects.is_empty());
}
#[test]
fn dizzy_launches_actual_plasma_and_blinds_on_hit_even_when_target_looks_away() {
    let (mut k, mut c) = (Gekko::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 5.]);
    cast(&mut k, &mut c, 2);
    for _ in 0..14 {
        step(&mut k, &mut c);
    }
    assert!(c.vision[&2].splash > 0);
    assert!(
        c.w.effects
            .contains(&(Entity::Player(2), "minecraft:blindness".into()))
    );
    assert!(c.w.damage.is_empty());
    assert!(
        c.objects
            .iter()
            .any(|o| matches!(o.kind, ObjectKind::Dizzy) && o.health == 2.)
    );
}
#[test]
fn plasma_in_flight_collides_with_new_terrain_before_target() {
    let (mut k, mut c) = (Gekko::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 5.]);
    k.settings.dizzy_ticks = 20;
    cast(&mut k, &mut c, 2);
    for _ in 0..8 {
        step(&mut k, &mut c);
    }
    assert!(
        c.objects
            .iter()
            .any(|o| matches!(o.kind, ObjectKind::Plasma))
    );
    c.w.wall = Some(Box3 {
        min: [-3., 0., 3.2],
        max: [3., 5., 3.4],
    });
    for _ in 0..15 {
        step(&mut k, &mut c);
    }
    assert!(c.vision.is_empty());
}
#[test]
fn plasma_respects_projectile_shield_and_does_not_damage_it() {
    let (mut k, mut c) = (Gekko::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 5.]);
    k.settings.dizzy_ticks = 20;
    c.shields.push(shields::Shield {
        owner: 99,
        center: [0., 2.5, 3.2],
        radius: 0.6,
        ticks: 100,
        health: 10.,
        active: true,
        spent: false,
    });
    cast(&mut k, &mut c, 2);
    for _ in 0..21 {
        step(&mut k, &mut c);
    }
    assert!(c.vision.is_empty());
    assert_eq!(c.shields[0].health, 10.);
}
#[test]
fn plasma_cannot_blind_phased_allied_or_unknown_chunk_targets() {
    for mode in 0..3 {
        let (mut k, mut c) = (Gekko::default(), Scene::default());
        c.w.actors[1] = actor(2, [0., 1., 5.]);
        match mode {
            0 => c.w.protect(
                2,
                effects::Protection {
                    invulnerable: true,
                    ..Default::default()
                },
            ),
            1 => {
                c.w.intel.insert(1, vec![1, 2]);
            }
            _ => c.w.unloaded_z = Some(2),
        }
        cast(&mut k, &mut c, 2);
        for _ in 0..25 {
            step(&mut k, &mut c);
        }
        assert!(c.vision.is_empty());
    }
}
#[test]
fn dizzy_mob_blind_has_explicit_real_physical_fallback() {
    let (mut k, mut c) = (Gekko::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 5.]);
    c.w.actors[1].entity = Entity::Mob(50);
    cast(&mut k, &mut c, 2);
    for _ in 0..14 {
        step(&mut k, &mut c);
    }
    assert!(
        c.w.effects
            .contains(&(Entity::Mob(50), "minecraft:weakness".into()))
    );
    assert!(
        c.w.effects
            .contains(&(Entity::Mob(50), "minecraft:slowness".into()))
    );
}
#[test]
fn reclaim_requires_hold_then_short_cooldown_and_cannot_duplicate_charge() {
    let (mut k, mut c) = (Gekko::default(), Scene::default());
    wing_globule(&mut k, &mut c);
    collect(&mut k, &mut c);
    assert_eq!(k.stats()[0], 0);
    assert_eq!(k.core.slots[1].charges, 0);
    assert_eq!(k.core.slots[1].remaining, 80);
    assert!(!recall(&mut k, &mut c));
    for _ in 0..80 {
        step(&mut k, &mut c);
    }
    assert_eq!(k.core.slots[1].charges, 1);
    assert!(!recall(&mut k, &mut c));
}
#[test]
fn reclaim_interrupts_on_release_motion_damage_obstruction_range_and_lost_lease() {
    for mode in 0..6 {
        let (mut k, mut c) = (Gekko::default(), Scene::default());
        wing_globule(&mut k, &mut c);
        assert!(recall(&mut k, &mut c));
        step(&mut k, &mut c);
        match mode {
            0 => k.recall_release(),
            1 => {
                c.w.move_player(1, [1., 1., 0.]);
            }
            2 => c.w.protections.get_mut(&1).unwrap().hurt_epoch += 1,
            3 => {
                let p = c.objects[0].position;
                c.w.wall = Some(Box3 {
                    min: [-2., 1., p[2] - 0.2],
                    max: [2., 4., p[2] + 0.2],
                });
            }
            4 => {
                c.w.move_player(1, [0., 1., -5.]);
            }
            _ => {
                for _ in 0..13 {
                    step(&mut k, &mut c);
                }
            }
        }
        step(&mut k, &mut c);
        assert_eq!(k.stats()[1], 0, "mode {mode}");
        assert!(k.core.slots[1].remaining > 100);
        assert_eq!(k.core.slots[1].charges, 0);
    }
}
#[test]
fn foreign_globules_are_not_reclaimed_and_expired_ones_leave_no_owned_refs() {
    let (mut owner, mut source) = (Gekko::default(), Scene::default());
    owner.settings.globule_ticks = 40;
    wing_globule(&mut owner, &mut source);
    let (mut other, mut scene) = (Gekko::default(), Scene::default());
    scene.objects = source.objects.clone();
    other.core.slots[1].charges = 0;
    assert!(!recall(&mut other, &mut scene));
    for _ in 0..41 {
        step(&mut owner, &mut source);
    }
    assert_eq!(owner.stats()[0], 0);
    assert!(source.objects.is_empty());
}
#[test]
fn pending_globule_waits_for_reaped_destroyed_utility_instead_of_being_lost() {
    let (mut k, mut c) = (Gekko::default(), Scene::default());
    c.w.move_player(2, [0., 1., 100.]);
    cast(&mut k, &mut c, 1);
    c.objects[0].health = 0.;
    for _ in 0..MAX_OBJECTS - 1 {
        c.ctx()
            .object(99, ObjectKind::Leer, [10., 2., 0.], 1., 100, [0.; 3]);
    }
    step(&mut k, &mut c);
    assert_eq!(k.stats()[0], 0);
    step(&mut k, &mut c);
    assert_eq!(k.stats()[0], 1);
    assert_eq!(c.objects.len(), MAX_OBJECTS);
}
#[test]
fn mosh_has_overhand_underhand_physics_ground_damage_phase_and_real_explosion() {
    let (mut k, mut c) = (Gekko::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 0.]);
    cast(&mut k, &mut c, 0);
    let normal = length(c.objects[0].velocity);
    let (mut short, mut scene) = (Gekko::default(), Scene::default());
    activate(&mut short, &mut scene, 0);
    assert!(fire(&mut short, &mut scene, true));
    assert!(length(scene.objects[0].velocity) < normal);
    c.objects[0].position = [0., 1.3, 0.];
    c.objects[0].velocity = [0.; 3];
    for _ in 0..15 {
        step(&mut k, &mut c);
    }
    assert!(
        c.w.damage
            .iter()
            .any(|(_, _, kind)| kind == "minecraft:magic")
    );
    assert!(
        !c.w.damage
            .iter()
            .any(|(_, _, kind)| kind == "minecraft:explosion")
    );
    for _ in 0..40 {
        step(&mut k, &mut c);
    }
    assert!(
        c.w.damage
            .iter()
            .any(|(target, _, kind)| *target == Entity::Player(1) && kind == "minecraft:explosion")
    );
    assert!(c.objects.is_empty());
    assert_eq!(k.stats()[0], 0);
}
#[test]
fn mosh_never_damages_through_wall_or_across_unknown_chunk() {
    for unknown in [false, true] {
        let (mut k, mut c) = (Gekko::default(), Scene::default());
        c.w.actors[1] = actor(2, [2., 1., 1.]);
        cast(&mut k, &mut c, 0);
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
        for _ in 0..60 {
            step(&mut k, &mut c);
        }
        assert!(!c.w.damage.iter().any(|(e, _, _)| *e == Entity::Player(2)));
        assert!(c.objects.is_empty());
    }
}
#[test]
fn thrash_remote_body_is_vulnerable_and_reactivation_lunges_into_aoe_detain() {
    let (mut k, mut c) = (Gekko::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 3.]);
    let origin = c.w.actors[0].position;
    cast(&mut k, &mut c, 3);
    step(&mut k, &mut c);
    assert!(k.camera().is_some());
    assert!(c.w.protections[&1].disarmed && !c.w.protections[&1].invulnerable);
    assert!(activate(&mut k, &mut c, 3));
    for _ in 0..8 {
        step(&mut k, &mut c);
    }
    assert!(k.camera().is_none());
    assert!(c.controls[&Entity::Player(2)].detained > 0);
    assert!(c.w.protections[&2].disarmed);
    assert_eq!(c.w.actors[0].position, origin);
    assert!(c.w.damage.is_empty());
    assert_eq!(k.core.slots[3].charges, 0);
}
#[test]
fn thrash_can_be_reclaimed_once_then_natural_recharge_opens_new_cycle() {
    let (mut k, mut c) = (Gekko::default(), Scene::default());
    cast(&mut k, &mut c, 3);
    k.cancel();
    step(&mut k, &mut c);
    collect(&mut k, &mut c);
    assert_eq!(k.thrash_reclaims, 1);
    for _ in 0..80 {
        step(&mut k, &mut c);
    }
    assert_eq!(k.core.slots[3].charges, 1);
    cast(&mut k, &mut c, 3);
    k.cancel();
    step(&mut k, &mut c);
    assert_eq!(k.stats()[0], 0);
    assert!(c.objects.is_empty());
    k.core.slots[3].remaining = 2;
    for _ in 0..2 {
        step(&mut k, &mut c);
    }
    assert_eq!(k.core.slots[3].charges, 1);
    assert_eq!(k.thrash_reclaims, 0);
}
#[test]
fn thrash_wall_or_phased_target_does_not_cause_ghost_detain() {
    for phase in [false, true] {
        let (mut k, mut c) = (Gekko::default(), Scene::default());
        c.w.actors[1] = actor(2, [0., 1., 3.]);
        if phase {
            c.w.protect(
                2,
                effects::Protection {
                    invulnerable: true,
                    ..Default::default()
                },
            );
        } else {
            c.w.wall = Some(Box3 {
                min: [-3., 0., 2.],
                max: [3., 5., 2.3],
            });
        }
        cast(&mut k, &mut c, 3);
        assert!(fire(&mut k, &mut c, false));
        for _ in 0..14 {
            step(&mut k, &mut c);
        }
        assert!(c.controls.is_empty());
    }
}
#[test]
fn controlled_creature_damage_destroy_and_expiry_revert_to_globule() {
    for mode in 0..3 {
        let (mut k, mut c) = (Gekko::default(), Scene::default());
        k.settings.thrash_ticks = 40;
        c.w.move_player(2, [0., 1., 100.]);
        cast(&mut k, &mut c, 3);
        step(&mut k, &mut c);
        match mode {
            0 => c.w.protections.get_mut(&1).unwrap().hurt_epoch += 1,
            1 => c.objects[0].damage(8., Some(Entity::Player(2)), [0., 2., 2.]),
            _ => {
                for _ in 0..40 {
                    step(&mut k, &mut c);
                }
            }
        }
        step(&mut k, &mut c);
        assert!(k.camera().is_none());
        assert!(
            c.objects
                .iter()
                .any(|o| matches!(o.kind, ObjectKind::ThrashGlobule))
        );
        assert!(!c.w.protections[&1].disarmed);
    }
}
#[test]
fn detain_gates_abilities_and_interactions_without_freezing_gravity_and_expires() {
    let mut w = TestWorld::default();
    w.actors[0].velocity = [0.8, -0.4, 0.];
    let mut s = Session::default();
    s.players.insert(1, Player::default());
    control::detain(&mut s.controls, &w, Entity::Player(1), 4);
    s.enqueue(
        1,
        Request {
            expected_agent: 0,
            sequence: 1,
            action: Action::Activate(0),
        },
    );
    s.tick(&mut w);
    assert_eq!(s.players[&1].jett.slots[0].charges, 2);
    assert!(w.protections[&1].disarmed);
    assert_eq!(w.actors[0].velocity[1], -0.4);
    assert!(w.actors[0].velocity[0] < 0.8);
    assert!(s.snapshot(1).unwrap().detained > 0);
    for _ in 0..5 {
        s.tick(&mut w);
    }
    assert_eq!(s.snapshot(1).unwrap().detained, 0);
    assert!(!w.protections[&1].disarmed);
}
#[test]
fn resource_limits_preserve_charges_and_death_removes_all_owned_objects() {
    let (mut k, mut c) = (Gekko::default(), Scene::default());
    for _ in 0..MAX_OBJECTS {
        c.ctx()
            .object(99, ObjectKind::Leer, [3., 2., 0.], 1., 100, [0.; 3]);
    }
    for slot in 0..4 {
        activate(&mut k, &mut c, slot);
        assert!(!fire(&mut k, &mut c, false));
    }
    assert_eq!(k.core.slots.map(|s| s.charges), [1; 4]);
    c.objects.clear();
    cast(&mut k, &mut c, 0);
    cast(&mut k, &mut c, 2);
    let ids = c.objects.iter().map(|o| o.id).collect::<Vec<_>>();
    k.leave();
    assert!(ids.into_iter().all(|id| !k.owns(id)));
    assert_eq!(k.core.slots[0].charges, 0);
    assert_eq!(k.core.slots[2].charges, 0);
}
#[test]
fn physical_contact_fails_closed_under_zero_budget() {
    let mut c = Scene::default();
    c.budget.rays = 0;
    let result = valorant_horizon_engine::advanced::companions::contact(
        &mut c.ctx(),
        1,
        [0., 2., 0.],
        [0., 2., 1.],
        0.1,
        true,
        None,
    );
    assert!(result.blocked);
    assert_eq!(result.at, [0., 2., 0.]);
    assert!(result.target.is_none());
}
#[test]
fn server_release_r_cancels_reclaim_channel_and_restores_interactions() {
    let (mut k, mut c) = (Gekko::default(), Scene::default());
    wing_globule(&mut k, &mut c);
    let mut s = Session {
        objects: c.objects,
        ..Default::default()
    };
    s.players.insert(
        1,
        Player {
            agent: 21,
            extra: Expansion {
                gekko: k,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    s.enqueue(
        1,
        Request {
            expected_agent: 21,
            sequence: 1,
            action: Action::Recall,
        },
    );
    s.tick(&mut c.w);
    assert!(s.snapshot(1).unwrap().kit[1] > 0);
    assert!(c.w.protections[&1].disarmed);
    s.enqueue(
        1,
        Request {
            expected_agent: 21,
            sequence: 2,
            action: Action::Release(5),
        },
    );
    s.tick(&mut c.w);
    assert_eq!(s.snapshot(1).unwrap().kit[1], 0);
    assert!(!c.w.protections[&1].disarmed);
    assert_eq!(s.snapshot(1).unwrap().charges[1], 0);
}
#[test]
fn all_requested_agents_are_registered_and_third_menu_page_names_gekko() {
    let mut state = valorant_horizon_engine::State::default();
    assert_eq!(state.registry.index("skye"), Some(20));
    assert_eq!(state.registry.index("gekko"), Some(21));
    assert!(state.registry.agents().len() >= 22);
    assert!(state.select(21));
    let text = state
        .lines(&["C".into(), "Q".into(), "E".into(), "X".into()])
        .join("\n");
    for name in NAMES {
        assert!(text.contains(name));
    }
}
mod departure {
    include!("../../helpers/player_departed.rs");
    pub fn run(id: u128) {
        valorant_player_departed(id)
    }
}
mod hears {
    include!("../../helpers/phase_hears.rs");
    pub fn run() -> bool {
        let mut value = false;
        valorant_phase_hears(&mut value);
        value
    }
}
#[test]
fn actual_departure_hook_captures_unobserved_native_knockout_and_clears_ghost_phase() {
    let (owner, ghost) = (950001, 950002);
    let key = (950001, "departure-test".into());
    let mut c = Scene::default();
    c.w.actors[0] = actor(owner, [0., 1., 0.]);
    let mut kayo = valorant_horizon_engine::advanced::kayo::Kayo::default();
    let a = c.w.actors[0].clone();
    assert!(kayo.activate(3, &a, &mut c.ctx()));
    let mut downed = kayo.protection(&a);
    downed.downed = true;
    downed.downable = false;
    downed.downed_pool = 40.;
    downed.health = 1.;
    effects::protections()
        .lock()
        .unwrap()
        .insert(owner as i32, downed);
    effects::protections().lock().unwrap().insert(
        ghost as i32,
        effects::Protection {
            invulnerable: true,
            ..Default::default()
        },
    );
    let mut phase = Player {
        agent: 15,
        ..Default::default()
    };
    phase.extra.yoru.phase = 10;
    phase.extra.yoru.core.slots[0].charges = 0;
    phase.extra.yoru.core.slots[0].remaining = 100;
    let mut session = Session::default();
    session.players.insert(
        owner,
        Player {
            agent: 18,
            extra: Expansion {
                kayo,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    session.players.insert(ghost, phase);
    combat::register(
        key.clone(),
        vec![
            (owner as i32, Entity::Player(owner)),
            (ghost as i32, Entity::Player(ghost)),
        ],
    );
    valorant_horizon_engine::server::sessions()
        .lock()
        .unwrap()
        .insert(key.clone(), session);
    assert!(hears::run());
    departure::run(owner);
    departure::run(ghost);
    let all = valorant_horizon_engine::server::sessions().lock().unwrap();
    let s = &all[&key];
    assert!(matches!(
        s.players[&owner].extra.kayo.overload,
        valorant_horizon_engine::advanced::kayo::Overload::PendingFatal
    ));
    assert_eq!(s.players[&owner].extra.kayo.core.slots[3].charges, 0);
    assert_eq!(s.players[&ghost].extra.yoru.phase, 0);
    assert_eq!(s.players[&ghost].extra.yoru.core.slots[0].remaining, 100);
    drop(all);
    assert!(!hears::run());
    assert!(combat::raw_ids(owner).is_empty());
    assert!(
        !effects::protections()
            .lock()
            .unwrap()
            .contains_key(&(owner as i32))
    );
    valorant_horizon_engine::server::sessions()
        .lock()
        .unwrap()
        .remove(&key);
}
fn channel_fixture() -> (Session, TestWorld, u128, u128) {
    let (owner, reviver) = (960001, 960002);
    let mut c = Scene::default();
    c.w.actors = vec![actor(owner, [0., 1., 2.]), actor(reviver, [0., 1., 0.])];
    c.w.intel.insert(owner, vec![owner, reviver]);
    c.w.intel.insert(reviver, vec![owner, reviver]);
    let mut kayo = valorant_horizon_engine::advanced::kayo::Kayo::default();
    let a = c.w.actors[0].clone();
    assert!(kayo.activate(3, &a, &mut c.ctx()));
    c.w.actors[0].health = 1.;
    let p = c.w.protections.get_mut(&owner).unwrap();
    p.downed = true;
    p.downable = false;
    p.downed_pool = 40.;
    p.health = 1.;
    let a = c.w.actors[0].clone();
    c.step(&mut kayo, &a);
    c.w.protect(owner, kayo.protection(&a));
    let mut s = Session {
        objects: c.objects,
        ..Default::default()
    };
    s.players.insert(
        owner,
        Player {
            agent: 18,
            extra: Expansion {
                kayo,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    s.players.insert(reviver, Player::default());
    s.enqueue(
        reviver,
        Request {
            expected_agent: 0,
            sequence: 1,
            action: Action::Stabilize(true),
        },
    );
    s.tick(&mut c.w);
    assert_eq!(s.stabilizers.len(), 1);
    (s, c.w, owner, reviver)
}
#[test]
fn departing_target_cancels_channel_without_sticking_old_reviver_disarm() {
    let (s, w, owner, reviver) = channel_fixture();
    let key = (960001, "departure-channel-test".into());
    effects::protections()
        .lock()
        .unwrap()
        .insert(owner as i32, w.protection_state(owner));
    effects::protections()
        .lock()
        .unwrap()
        .insert(reviver as i32, w.protection_state(reviver));
    assert!(effects::protections().lock().unwrap()[&(reviver as i32)].disarmed);
    combat::register(
        key.clone(),
        vec![
            (owner as i32, Entity::Player(owner)),
            (reviver as i32, Entity::Player(reviver)),
        ],
    );
    valorant_horizon_engine::server::sessions()
        .lock()
        .unwrap()
        .insert(key.clone(), s);
    departure::run(owner);
    assert!(!effects::protections().lock().unwrap()[&(reviver as i32)].disarmed);
    assert!(
        valorant_horizon_engine::server::sessions().lock().unwrap()[&key]
            .stabilizers
            .is_empty()
    );
    valorant_horizon_engine::server::sessions()
        .lock()
        .unwrap()
        .remove(&key);
    combat::detach(reviver);
    effects::protections()
        .lock()
        .unwrap()
        .remove(&(reviver as i32));
}
#[test]
fn detain_interrupts_existing_stabilization_but_suppression_remains_an_ability_gate() {
    let (mut s, mut w, owner, reviver) = channel_fixture();
    control::detain(&mut s.controls, &w, Entity::Player(reviver), 40);
    s.tick(&mut w);
    assert!(s.stabilizers.is_empty());
    assert_eq!(w.actors[0].entity, Entity::Player(owner));
    assert_eq!(w.actors[0].health, 1.);
    let (mut s, mut w, _owner, reviver) = channel_fixture();
    control::suppress(&mut s.controls, &w, Entity::Player(reviver), 40);
    s.tick(&mut w);
    assert_eq!(s.stabilizers.len(), 1);
}
#[test]
fn all_automatic_companions_use_hostile_target_policy_including_previous_skye() {
    let mut c = Scene::default();
    c.w.actors[1].entity = Entity::Mob(50);
    c.w.non_hostile.push(Entity::Mob(50));
    let mut skye = valorant_horizon_engine::advanced::skye::Skye::default();
    let a = c.w.actors[0].clone();
    assert!(skye.activate(3, &a, &mut c.ctx()));
    assert!(!skye.fire(false, &a, &mut c.ctx()));
    assert_eq!(skye.core.slots[3].charges, 1);
    let mut gekko = Gekko::default();
    cast(&mut gekko, &mut c, 2);
    for _ in 0..15 {
        step(&mut gekko, &mut c);
    }
    assert!(
        !c.objects
            .iter()
            .any(|o| matches!(o.kind, ObjectKind::Plasma))
    );
}
#[test]
fn phase_activated_in_same_frame_as_kayo_pulse_is_published_before_target_scan() {
    for reverse in [false, true] {
        let mut w = TestWorld::default();
        if reverse {
            w.actors.reverse();
        }
        let mut s = Session::default();
        s.players.insert(
            1,
            Player {
                agent: 18,
                ..Default::default()
            },
        );
        s.players.insert(
            2,
            Player {
                agent: 15,
                ..Default::default()
            },
        );
        s.enqueue(
            1,
            Request {
                expected_agent: 18,
                sequence: 1,
                action: Action::Activate(3),
            },
        );
        s.enqueue(
            2,
            Request {
                expected_agent: 15,
                sequence: 1,
                action: Action::Activate(3),
            },
        );
        s.enqueue(
            2,
            Request {
                expected_agent: 15,
                sequence: 2,
                action: Action::Fire(false),
            },
        );
        s.tick(&mut w);
        assert!(s.players[&2].extra.yoru.phase > 0);
        assert!(!s.controls.contains_key(&Entity::Player(2)));
        assert!(w.protections[&2].invulnerable);
    }
}
#[test]
fn same_tick_suppression_interrupts_later_companion_control_without_join_order_delay() {
    let mut w = TestWorld::default();
    w.actors[1] = actor(2, [3., 1., 0.]);
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 18,
            ..Default::default()
        },
    );
    s.players.insert(
        2,
        Player {
            agent: 20,
            ..Default::default()
        },
    );
    for (seq, action) in [(1, Action::Activate(1)), (2, Action::Fire(false))] {
        s.enqueue(
            2,
            Request {
                expected_agent: 20,
                sequence: seq,
                action,
            },
        );
    }
    s.enqueue(
        1,
        Request {
            expected_agent: 18,
            sequence: 1,
            action: Action::Activate(3),
        },
    );
    s.tick(&mut w);
    assert!(s.controls[&Entity::Player(2)].suppressed > 0);
    assert!(s.players[&2].extra.skye.camera().is_none());
    assert_eq!(s.players[&2].extra.skye.core.slots[1].charges, 0);
}
