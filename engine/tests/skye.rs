mod support;
use support::{extra::Scene, *};
use valorant_horizon_engine::{
    advanced::{Agent, skye::*},
    gameplay::{geometry::*, objects::*, *},
};
fn step(k: &mut Skye, c: &mut Scene) {
    let a = c.w.actors[0].clone();
    c.step(k, &a);
    c.w.protect(1, k.protection(&a));
    for o in &mut c.objects {
        o.ticks = o.ticks.saturating_sub(1);
    }
    c.objects
        .retain(|o| o.health > 0. && o.ticks > 0 && (o.owner != 1 || k.owns(o.id)));
}
fn activate(k: &mut Skye, c: &mut Scene, s: usize) -> bool {
    let a = c.w.actors[0].clone();
    k.activate(s, &a, &mut c.ctx())
}
fn fire(k: &mut Skye, c: &mut Scene) -> bool {
    let a = c.w.actors[0].clone();
    k.fire(false, &a, &mut c.ctx())
}
fn release(k: &mut Skye, c: &mut Scene) {
    let a = c.w.actors[0].clone();
    k.release(&a, &mut c.ctx());
}
#[test]
fn regrowth_heals_scoreboard_allies_without_self_heal_and_spends_real_pool() {
    let (mut k, mut c) = (Skye::default(), Scene::default());
    c.w.actors[0].health = 10.;
    c.w.actors[1].health = 10.;
    c.w.intel.insert(1, vec![1, 2]);
    assert!(activate(&mut k, &mut c, 0));
    assert!(fire(&mut k, &mut c));
    for _ in 0..4 {
        step(&mut k, &mut c);
    }
    assert_eq!(c.w.actors[0].health, 10.);
    assert_eq!(c.w.actors[1].health, 11.);
    assert_eq!(k.pool, 76);
    assert!(c.w.protections[&1].disarmed);
    release(&mut k, &mut c);
    step(&mut k, &mut c);
    assert_eq!(c.w.actors[1].health, 11.);
    assert!(!c.w.protections[&1].disarmed);
}
#[test]
fn regrowth_group_channel_spends_once_per_tick_and_never_heals_enemy() {
    let (mut k, mut c) = (Skye::default(), Scene::default());
    c.w.actors[1].health = 10.;
    let mut third = actor(3, [2., 1., 2.]);
    third.health = 10.;
    c.w.actors.push(third);
    let mut enemy = actor(4, [0., 1., 3.]);
    enemy.health = 10.;
    c.w.actors.push(enemy);
    c.w.intel.insert(1, vec![1, 2, 3]);
    activate(&mut k, &mut c, 0);
    fire(&mut k, &mut c);
    step(&mut k, &mut c);
    assert_eq!(k.pool, 79);
    assert_eq!(c.w.actors[1].health, 10.25);
    assert_eq!(c.w.actors[2].health, 10.25);
    assert_eq!(c.w.actors[3].health, 10.);
}
#[test]
fn regrowth_requires_injured_alive_ally_range_los_and_not_downed() {
    for mode in 0..5 {
        let (mut k, mut c) = (Skye::default(), Scene::default());
        c.w.actors[1] = actor(2, [0., 1., 5.]);
        c.w.actors[1].health = 10.;
        c.w.intel.insert(1, vec![1, 2]);
        match mode {
            0 => c.w.actors[1].health = 20.,
            1 => c.w.actors[1].health = 0.,
            2 => {
                c.w.move_player(2, [0., 1., 10.]);
            }
            3 => {
                c.w.wall = Some(Box3 {
                    min: [-3., 1., 2.],
                    max: [3., 5., 3.],
                })
            }
            _ => c.w.protect(
                2,
                effects::Protection {
                    downed: true,
                    ..Default::default()
                },
            ),
        }
        let hp = c.w.actors[1].health;
        activate(&mut k, &mut c, 0);
        fire(&mut k, &mut c);
        for _ in 0..4 {
            step(&mut k, &mut c);
        }
        assert_eq!(c.w.actors[1].health, hp, "mode {mode}");
        assert_eq!(k.pool, 80);
    }
}
#[test]
fn empty_heal_pool_cannot_be_reused_until_delayed_regeneration() {
    let (mut k, mut c) = (Skye::default(), Scene::default());
    k.pool = 1;
    c.w.actors[1].health = 10.;
    c.w.intel.insert(1, vec![1, 2]);
    activate(&mut k, &mut c, 0);
    fire(&mut k, &mut c);
    step(&mut k, &mut c);
    assert_eq!(k.pool, 0);
    assert_eq!(k.core.slots[0].charges, 0);
    assert!(!activate(&mut k, &mut c, 0));
    for _ in 0..80 {
        step(&mut k, &mut c);
    }
    assert_eq!(k.pool, 0);
    for _ in 0..30 {
        step(&mut k, &mut c);
    }
    assert!(k.pool > 0);
    assert_eq!(k.core.slots[0].charges, 1);
    assert!(k.busy());
}
#[test]
fn trailblazer_moves_camera_and_companion_but_keeps_real_body_vulnerable() {
    let (mut k, mut c) = (Skye::default(), Scene::default());
    let body = c.w.actors[0].position;
    activate(&mut k, &mut c, 1);
    assert!(fire(&mut k, &mut c));
    c.w.actors[0].movement = [1., 0., 0.];
    for _ in 0..8 {
        step(&mut k, &mut c);
    }
    assert_eq!(c.w.actors[0].position, body);
    assert!(k.camera().unwrap()[0] > 1.);
    assert!(c.w.protections[&1].disarmed);
    assert!(!c.w.protections[&1].invulnerable);
    assert_eq!(k.core.slots[1].charges, 0);
}
#[test]
fn trailblazer_leap_hits_real_target_and_concusses_nearby_enemies() {
    let (mut k, mut c) = (Skye::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 3.]);
    c.w.actors.push(actor(3, [1., 1., 3.]));
    activate(&mut k, &mut c, 1);
    fire(&mut k, &mut c);
    assert!(fire(&mut k, &mut c));
    for _ in 0..8 {
        step(&mut k, &mut c);
    }
    assert_eq!(c.w.actors[1].health, 14.);
    assert_eq!(c.w.actors[2].health, 20.);
    assert!(
        c.w.effects
            .contains(&(Entity::Player(3), "minecraft:slowness".into()))
    );
    assert!(k.camera().is_none());
    assert!(c.objects.is_empty());
}
#[test]
fn trailblazer_wall_collision_prevents_damage_or_stun_through_wall() {
    let (mut k, mut c) = (Skye::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 3.]);
    c.w.wall = Some(Box3 {
        min: [-3., 0., 2.],
        max: [3., 5., 2.3],
    });
    activate(&mut k, &mut c, 1);
    fire(&mut k, &mut c);
    fire(&mut k, &mut c);
    for _ in 0..8 {
        step(&mut k, &mut c);
    }
    assert!(c.w.damage.is_empty());
    assert!(c.w.effects.is_empty());
    assert!(k.camera().is_none());
}
#[test]
fn trailblazer_damage_destroy_cancel_and_expiry_restore_body_control_without_refunds() {
    for mode in 0..4 {
        let (mut k, mut c) = (Skye::default(), Scene::default());
        k.settings.tiger_ticks = 40;
        activate(&mut k, &mut c, 1);
        fire(&mut k, &mut c);
        step(&mut k, &mut c);
        match mode {
            0 => c.w.protections.get_mut(&1).unwrap().hurt_epoch += 1,
            1 => c.objects[0].damage(10., Some(Entity::Player(2)), [0., 2., 2.]),
            2 => k.cancel(),
            _ => {
                for _ in 0..40 {
                    step(&mut k, &mut c);
                }
            }
        }
        step(&mut k, &mut c);
        assert!(k.camera().is_none());
        assert!(!c.w.protections[&1].disarmed);
        assert_eq!(k.core.slots[1].charges, 0);
        assert!(c.objects.is_empty());
    }
}
#[test]
fn trailblazer_requires_ground_and_does_not_move_into_unknown_chunks() {
    let (mut k, mut c) = (Skye::default(), Scene::default());
    c.w.actors[0].ground = false;
    activate(&mut k, &mut c, 1);
    assert!(!fire(&mut k, &mut c));
    assert_eq!(k.core.slots[1].charges, 1);
    c.w.actors[0].ground = true;
    assert!(fire(&mut k, &mut c));
    c.w.unloaded_z = Some(3);
    c.w.actors[0].movement = [0., 0., 1.];
    for _ in 0..20 {
        step(&mut k, &mut c);
    }
    assert!(k.camera().unwrap()[2] < 3.);
}
#[test]
fn hawk_hold_guides_release_keeps_heading_and_reuse_flashes_with_private_confirm() {
    let (mut k, mut c) = (Skye::default(), Scene::default());
    c.w.actors[1] = actor(2, [8., 1., 0.]);
    c.w.actors[1].direction = [-1., 0., 0.];
    activate(&mut k, &mut c, 2);
    fire(&mut k, &mut c);
    c.w.actors[0].direction = [1., 0., 0.];
    for _ in 0..10 {
        step(&mut k, &mut c);
    }
    assert!(c.objects[0].position[0] > 3.9);
    release(&mut k, &mut c);
    let x = c.objects[0].position[0];
    c.w.actors[0].direction = [-1., 0., 0.];
    step(&mut k, &mut c);
    assert!(c.objects[0].position[0] > x);
    assert!(activate(&mut k, &mut c, 2));
    step(&mut k, &mut c);
    assert!(c.objects.is_empty());
    assert_eq!(k.stats()[2], 30);
    assert!(
        c.w.effects
            .contains(&(Entity::Player(2), "minecraft:blindness".into()))
    );
    assert!(!k.equipped());
}
#[test]
fn hawk_auto_flashes_at_lifetime_but_not_if_destroyed_or_unknown() {
    for mode in 0..3 {
        let (mut k, mut c) = (Skye::default(), Scene::default());
        k.settings.hawk_ticks = 20;
        c.w.actors[1] = actor(2, [0., 1., 10.]);
        c.w.actors[1].direction = [0., 0., -1.];
        activate(&mut k, &mut c, 2);
        fire(&mut k, &mut c);
        if mode == 1 {
            c.objects[0].damage(2., Some(Entity::Player(2)), [0., 2., 3.]);
        } else if mode == 2 {
            c.w.unloaded_z = Some(2);
        }
        for _ in 0..22 {
            step(&mut k, &mut c);
        }
        assert!(c.objects.is_empty());
        assert_eq!(
            c.w.effects.iter().any(|(e, _)| *e == Entity::Player(2)),
            mode == 0
        );
        assert!(!k.equipped());
    }
}
#[test]
fn healing_hold_does_not_steer_previously_launched_hawk() {
    let (mut k, mut c) = (Skye::default(), Scene::default());
    activate(&mut k, &mut c, 2);
    fire(&mut k, &mut c);
    release(&mut k, &mut c);
    activate(&mut k, &mut c, 0);
    fire(&mut k, &mut c);
    c.w.actors[0].direction = [1., 0., 0.];
    step(&mut k, &mut c);
    let hawk = c
        .objects
        .iter()
        .find(|o| matches!(o.kind, ObjectKind::SkyeHawk))
        .unwrap();
    assert_eq!(hawk.position[0], 0.);
    assert!(hawk.position[2] > 0.);
}
#[test]
fn seekers_choose_three_nearest_unique_targets_and_apply_real_nearsight() {
    let (mut k, mut c) = (Skye::default(), Scene::default());
    for (id, z) in [(3, 4.), (4, 8.), (5, 20.)] {
        c.w.actors.push(actor(id, [0., 1., z]));
    }
    activate(&mut k, &mut c, 3);
    assert!(fire(&mut k, &mut c));
    let mut targets = c
        .objects
        .iter()
        .map(|o| o.target.unwrap())
        .collect::<Vec<_>>();
    targets.sort_by_key(|e| format!("{e:?}"));
    targets.dedup();
    assert_eq!(targets.len(), 3);
    assert!(!targets.contains(&Entity::Player(5)));
    for _ in 0..10 {
        step(&mut k, &mut c);
    }
    assert!(c.vision[&2].nearsight > 0);
    assert!(
        c.w.effects
            .contains(&(Entity::Player(2), "minecraft:blindness".into()))
    );
    assert!(c.w.damage.is_empty());
}
#[test]
fn seeker_mob_fallback_changes_movement_and_damage_without_claiming_ai_blindness() {
    let (mut k, mut c) = (Skye::default(), Scene::default());
    c.w.actors[1].entity = Entity::Mob(50);
    activate(&mut k, &mut c, 3);
    fire(&mut k, &mut c);
    for _ in 0..10 {
        step(&mut k, &mut c);
    }
    assert!(
        c.w.effects
            .contains(&(Entity::Mob(50), "minecraft:slowness".into()))
    );
    assert!(
        c.w.effects
            .contains(&(Entity::Mob(50), "minecraft:weakness".into()))
    );
}
#[test]
fn destroyed_seeker_and_phased_target_do_not_receive_hidden_nearsight() {
    for phase in [false, true] {
        let (mut k, mut c) = (Skye::default(), Scene::default());
        activate(&mut k, &mut c, 3);
        fire(&mut k, &mut c);
        if phase {
            c.w.protect(
                2,
                effects::Protection {
                    invulnerable: true,
                    ..Default::default()
                },
            );
        } else {
            c.objects[0].damage(5., Some(Entity::Player(2)), [1., 2., 1.]);
        }
        for _ in 0..10 {
            step(&mut k, &mut c);
        }
        assert!(c.vision.is_empty());
        assert!(c.objects.is_empty());
    }
}
#[test]
fn unknown_seeker_path_never_clips_or_runs_unbounded_and_expires() {
    let (mut k, mut c) = (Skye::default(), Scene::default());
    k.settings.seeker_ticks = 40;
    c.w.actors[1] = actor(2, [0., 1., 8.]);
    activate(&mut k, &mut c, 3);
    fire(&mut k, &mut c);
    c.w.unloaded_z = Some(3);
    for _ in 0..45 {
        step(&mut k, &mut c);
    }
    assert!(c.objects.is_empty());
    assert!(c.vision.is_empty());
}
#[test]
fn pool_limits_refuse_companions_atomically_without_spending() {
    let (mut k, mut c) = (Skye::default(), Scene::default());
    c.w.actors.push(actor(3, [0., 1., 4.]));
    c.w.actors.push(actor(4, [0., 1., 5.]));
    for _ in 0..MAX_OBJECTS - 1 {
        c.ctx()
            .object(99, ObjectKind::Leer, [3., 2., 0.], 1., 100, [0.; 3]);
    }
    activate(&mut k, &mut c, 3);
    assert!(!fire(&mut k, &mut c));
    assert_eq!(k.core.slots[3].charges, 1);
    assert_eq!(c.objects.len(), MAX_OBJECTS - 1);
    c.ctx()
        .object(99, ObjectKind::Leer, [3., 2., 0.], 1., 100, [0.; 3]);
    for slot in [1, 2] {
        activate(&mut k, &mut c, slot);
        assert!(!fire(&mut k, &mut c));
        assert_eq!(k.core.slots[slot].charges, DEFINITIONS[slot].charges);
    }
}
#[test]
fn death_and_suppression_cleanup_preserve_spent_resources() {
    let (mut k, mut c) = (Skye::default(), Scene::default());
    activate(&mut k, &mut c, 2);
    fire(&mut k, &mut c);
    let hawk = c.objects[0].id;
    k.suppress();
    assert!(k.owns(hawk));
    assert!(!k.core.firing);
    k.pool = 40;
    k.leave();
    assert!(!k.owns(hawk));
    assert_eq!(k.pool, 40);
    assert_eq!(k.core.slots[2].charges, 1);
}
#[test]
fn registry_pages_and_hud_include_skye_without_changing_breach() {
    let mut state = valorant_horizon_engine::State::default();
    assert_eq!(state.registry.index("breach"), Some(19));
    assert_eq!(state.registry.index("skye"), Some(20));
    assert!(state.select(20));
    let text = state
        .lines(&["C".into(), "Q".into(), "E".into(), "X".into()])
        .join("\n");
    for name in NAMES {
        assert!(text.contains(name));
    }
}
#[test]
fn refused_heal_does_not_consume_pool_and_preparation_survives_a_server_tick() {
    let (mut k, mut c) = (Skye::default(), Scene::default());
    c.w.actors[1].health = 10.;
    c.w.intel.insert(1, vec![1, 2]);
    c.w.heal_denied.push(Entity::Player(2));
    activate(&mut k, &mut c, 0);
    step(&mut k, &mut c);
    assert_eq!(k.core.equipped, Some(0));
    fire(&mut k, &mut c);
    step(&mut k, &mut c);
    assert_eq!(k.pool, 80);
    assert_eq!(c.w.actors[1].health, 10.);
    k.cancel();
    activate(&mut k, &mut c, 2);
    step(&mut k, &mut c);
    assert_eq!(k.core.equipped, Some(2));
    assert!(fire(&mut k, &mut c));
}
#[test]
fn flash_confirmation_stays_in_caster_snapshot_not_other_players_intel() {
    let (mut k, mut c) = (Skye::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 5.]);
    c.w.actors[1].direction = [0., 0., -1.];
    activate(&mut k, &mut c, 2);
    fire(&mut k, &mut c);
    activate(&mut k, &mut c, 2);
    step(&mut k, &mut c);
    let mut s = valorant_horizon_engine::server::Session::default();
    s.players.insert(
        1,
        valorant_horizon_engine::server::Player {
            agent: 20,
            extra: valorant_horizon_engine::advanced::Expansion {
                skye: k,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    s.players.insert(2, Default::default());
    assert_eq!(s.snapshot(1).unwrap().kit[2], 30);
    assert_eq!(s.snapshot(2).unwrap().kit, [0; 4]);
    assert!(s.snapshot(2).unwrap().contacts.is_empty());
}
#[test]
fn undefined_alt_heal_or_guidance_cannot_start_an_unreleased_hold_channel() {
    let (mut k, mut c) = (Skye::default(), Scene::default());
    c.w.actors[1].health = 10.;
    c.w.intel.insert(1, vec![1, 2]);
    for slot in [0, 2] {
        activate(&mut k, &mut c, slot);
        let a = c.w.actors[0].clone();
        assert!(!k.fire(true, &a, &mut c.ctx()));
        step(&mut k, &mut c);
        assert!(!k.core.firing);
    }
    assert_eq!(k.pool, 80);
    assert!(c.objects.is_empty());
    assert_eq!(c.w.actors[1].health, 10.);
}
