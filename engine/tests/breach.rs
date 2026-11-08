mod support;
use support::{extra::Scene, *};
use valorant_horizon_engine::{
    advanced::{Agent, breach::*},
    gameplay::{geometry::*, objects::*, *},
    server::{Player, Session},
};
fn step(k: &mut Breach, c: &mut Scene) {
    let a = c.w.actors[0].clone();
    c.step(k, &a);
    for o in &mut c.objects {
        o.ticks = o.ticks.saturating_sub(1);
    }
    c.objects.retain(|o| o.ticks > 0 && k.owns(o.id));
}
fn activate(k: &mut Breach, c: &mut Scene, slot: usize) -> bool {
    let a = c.w.actors[0].clone();
    k.activate(slot, &a, &mut c.ctx())
}
fn fire(k: &mut Breach, c: &mut Scene, alt: bool) -> bool {
    let a = c.w.actors[0].clone();
    k.fire(alt, &a, &mut c.ctx())
}
fn release(k: &mut Breach, c: &mut Scene) -> bool {
    let a = c.w.actors[0].clone();
    k.release(&a, &mut c.ctx())
}
fn wall(c: &mut Scene) {
    c.w.wall = Some(Box3 {
        min: [-3., 0., 2.],
        max: [3., 5., 3.],
    });
    c.w.actors[1] = actor(2, [0., 1., 5.]);
}
#[test]
fn aftershock_is_two_delayed_real_explosion_bursts_behind_wall() {
    let (mut k, mut c) = (Breach::default(), Scene::default());
    wall(&mut c);
    activate(&mut k, &mut c, 0);
    assert!(fire(&mut k, &mut c, false));
    assert!(c.objects[0].position[2] > 3.);
    for _ in 0..29 {
        step(&mut k, &mut c);
    }
    assert!(c.w.damage.is_empty());
    step(&mut k, &mut c);
    assert_eq!(c.w.actors[1].health, 11.);
    for _ in 0..15 {
        step(&mut k, &mut c);
    }
    assert_eq!(c.w.actors[1].health, 2.);
    assert_eq!(c.w.damage.len(), 2);
    assert!(
        c.w.damage
            .iter()
            .all(|(_, _, k)| k == "minecraft:explosion")
    );
    assert!(c.objects.is_empty());
}
#[test]
fn wall_charge_refuses_air_thick_wall_unknown_chunks_and_zero_budget_without_spending() {
    for mode in 0..4 {
        let (mut k, mut c) = (Breach::default(), Scene::default());
        match mode {
            0 => {}
            1 => {
                c.w.wall = Some(Box3 {
                    min: [-3., 0., 2.],
                    max: [3., 5., 8.],
                })
            }
            2 => c.w.unloaded_z = Some(3),
            _ => {
                wall(&mut c);
                c.budget.rays = 0;
            }
        }
        activate(&mut k, &mut c, 0);
        assert!(!fire(&mut k, &mut c, false), "mode {mode}");
        assert_eq!(k.core.slots[0].charges, 1);
    }
}
#[test]
fn aftershock_later_obstruction_shields_targets_and_new_block_inside_charge_cancels_it() {
    for inside in [false, true] {
        let (mut k, mut c) = (Breach::default(), Scene::default());
        wall(&mut c);
        activate(&mut k, &mut c, 0);
        fire(&mut k, &mut c, false);
        c.w.blocks
            .insert([0, 2, if inside { 3 } else { 4 }], "minecraft:stone".into());
        for _ in 0..50 {
            step(&mut k, &mut c);
        }
        assert!(c.w.damage.is_empty());
        assert!(c.objects.is_empty());
    }
}
#[test]
fn flashpoint_appears_behind_wall_and_blinds_looking_targets_not_caster_through_wall() {
    let (mut k, mut c) = (Breach::default(), Scene::default());
    wall(&mut c);
    c.w.actors[1].direction = [0., 0., -1.];
    activate(&mut k, &mut c, 1);
    fire(&mut k, &mut c, false);
    for _ in 0..9 {
        step(&mut k, &mut c);
    }
    assert!(c.w.effects.is_empty());
    step(&mut k, &mut c);
    assert!(
        c.w.effects
            .contains(&(Entity::Player(2), "minecraft:blindness".into()))
    );
    assert!(!c.w.effects.iter().any(|(e, _)| *e == Entity::Player(1)));
    assert!(c.w.damage.is_empty());
    assert!(c.objects.is_empty());
}
#[test]
fn flashpoint_respects_facing_and_phase_and_can_affect_allies() {
    for mode in 0..3 {
        let (mut k, mut c) = (Breach::default(), Scene::default());
        wall(&mut c);
        if mode != 0 {
            c.w.actors[1].direction = [0., 0., -1.];
        }
        if mode == 1 {
            c.w.protect(
                2,
                effects::Protection {
                    invulnerable: true,
                    ..Default::default()
                },
            );
        } else if mode == 2 {
            c.w.intel.insert(1, vec![1, 2]);
        }
        activate(&mut k, &mut c, 1);
        fire(&mut k, &mut c, false);
        for _ in 0..10 {
            step(&mut k, &mut c);
        }
        assert_eq!(
            c.w.effects.iter().any(|(e, _)| *e == Entity::Player(2)),
            mode == 2
        );
    }
}
#[test]
fn fault_line_requires_release_and_hold_extends_range_without_consuming_preparation() {
    for charged in [false, true] {
        let (mut k, mut c) = (Breach::default(), Scene::default());
        c.w.actors[1] = actor(2, [0., 1., 8.]);
        activate(&mut k, &mut c, 2);
        assert!(fire(&mut k, &mut c, false));
        for _ in 0..if charged { 40 } else { 1 } {
            step(&mut k, &mut c);
        }
        assert_eq!(k.core.slots[2].charges, 1);
        assert!(c.w.effects.is_empty());
        assert!(release(&mut k, &mut c));
        assert_eq!(k.core.slots[2].charges, 0);
        for _ in 0..35 {
            step(&mut k, &mut c);
        }
        assert_eq!(
            c.w.effects.iter().any(|(e, _)| *e == Entity::Player(2)),
            charged
        );
    }
}
#[test]
fn fault_line_alt_quick_cast_through_known_wall_concusses_without_damage() {
    let (mut k, mut c) = (Breach::default(), Scene::default());
    wall(&mut c);
    c.w.actors[1] = actor(2, [0., 1., 3.8]);
    activate(&mut k, &mut c, 2);
    assert!(fire(&mut k, &mut c, true));
    for _ in 0..12 {
        step(&mut k, &mut c);
    }
    for name in [
        "minecraft:slowness",
        "minecraft:mining_fatigue",
        "minecraft:weakness",
    ] {
        assert!(c.w.effects.contains(&(Entity::Player(2), name.into())));
    }
    assert!(c.w.damage.is_empty());
}
#[test]
fn quake_handles_height_sides_unknown_chunks_and_allies_without_unbounded_search() {
    for mode in 0..4 {
        let (mut k, mut c) = (Breach::default(), Scene::default());
        c.w.actors[1] = actor(2, [0., 1., 7.]);
        match mode {
            0 => c.w.actors[1] = actor(2, [0., 6., 7.]),
            1 => c.w.actors[1] = actor(2, [4., 1., 7.]),
            2 => c.w.unloaded_z = Some(5),
            _ => {
                c.w.intel.insert(1, vec![1, 2]);
            }
        }
        activate(&mut k, &mut c, 2);
        fire(&mut k, &mut c, false);
        for _ in 0..40 {
            step(&mut k, &mut c);
        }
        release(&mut k, &mut c);
        for _ in 0..40 {
            step(&mut k, &mut c);
        }
        assert!(c.w.effects.is_empty(), "mode {mode}");
        assert!(c.objects.is_empty());
    }
}
#[test]
fn rolling_thunder_cascades_and_knocks_up_players_and_mobs_through_terrain() {
    let (mut k, mut c) = (Breach::default(), Scene::default());
    wall(&mut c);
    c.w.actors[1] = actor(2, [3., 1., 8.]);
    let mut mob = actor(3, [0., 1., 10.]);
    mob.entity = Entity::Mob(50);
    c.w.actors.push(mob);
    activate(&mut k, &mut c, 3);
    fire(&mut k, &mut c, false);
    for _ in 0..11 {
        step(&mut k, &mut c);
    }
    assert!(c.w.effects.is_empty());
    for _ in 0..40 {
        step(&mut k, &mut c);
    }
    assert!(c.w.actors[1].velocity[1] > 0.5);
    assert!(c.w.actors[2].velocity[1] > 0.5);
    assert!(c.w.damage.is_empty());
    for _ in 0..30 {
        step(&mut k, &mut c);
    }
    assert!(c.objects.is_empty());
    assert_eq!(
        c.w.effects
            .iter()
            .filter(|(e, n)| *e == Entity::Player(2) && n == "minecraft:slowness")
            .count(),
        1
    );
}
#[test]
fn rolling_thunder_skips_protected_phased_far_and_high_targets() {
    for mode in 0..4 {
        let (mut k, mut c) = (Breach::default(), Scene::default());
        c.w.actors[1] = actor(2, [0., 1., 8.]);
        match mode {
            0 => {
                c.w.protect(
                    2,
                    effects::Protection {
                        invulnerable: true,
                        ..Default::default()
                    },
                );
            }
            1 => c.w.protected.push(Entity::Player(2)),
            2 => c.w.actors[1] = actor(2, [10., 1., 8.]),
            _ => c.w.actors[1] = actor(2, [0., 7., 8.]),
        }
        activate(&mut k, &mut c, 3);
        fire(&mut k, &mut c, false);
        for _ in 0..80 {
            step(&mut k, &mut c);
        }
        assert!(c.w.effects.is_empty());
        assert_eq!(c.w.actors[1].velocity, [0.; 3]);
    }
}
#[test]
fn cancel_preparation_is_free_but_cancel_after_fire_keeps_paid_wave() {
    let (mut k, mut c) = (Breach::default(), Scene::default());
    activate(&mut k, &mut c, 2);
    fire(&mut k, &mut c, false);
    step(&mut k, &mut c);
    k.cancel();
    assert_eq!(k.core.slots[2].charges, 1);
    assert!(c.objects.is_empty());
    activate(&mut k, &mut c, 3);
    fire(&mut k, &mut c, false);
    let id = c.objects[0].id;
    k.cancel();
    assert!(k.owns(id));
    assert_eq!(k.core.slots[3].charges, 0);
    k.leave();
    assert!(!k.owns(id));
}
#[test]
fn filled_object_pool_refuses_casts_without_spending_charged_fault_line() {
    let (mut k, mut c) = (Breach::default(), Scene::default());
    wall(&mut c);
    for _ in 0..MAX_OBJECTS {
        c.ctx()
            .object(99, ObjectKind::Leer, [0., 2., 0.], 1., 100, [0.; 3]);
    }
    for slot in 0..4 {
        activate(&mut k, &mut c, slot);
        if slot == 2 {
            fire(&mut k, &mut c, false);
            assert!(!release(&mut k, &mut c));
        } else {
            assert!(!fire(&mut k, &mut c, false));
        }
    }
    assert_eq!(k.core.slots.map(|r| r.charges), [1, 2, 1, 1]);
}
#[test]
fn suppression_interrupts_charging_but_previously_fired_quake_still_resolves() {
    let mut c = Scene::default();
    let mut k = Breach::default();
    c.w.actors[1] = actor(2, [0., 1., 3.]);
    activate(&mut k, &mut c, 3);
    fire(&mut k, &mut c, false);
    activate(&mut k, &mut c, 2);
    fire(&mut k, &mut c, false);
    let mut s = Session {
        objects: c.objects,
        ..Default::default()
    };
    s.players.insert(
        1,
        Player {
            agent: 19,
            extra: valorant_horizon_engine::advanced::Expansion {
                breach: k,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    control::suppress(&mut s.controls, &c.w, Entity::Player(1), 100);
    for _ in 0..25 {
        s.tick(&mut c.w);
    }
    assert!(!s.players[&1].extra.breach.core.firing);
    assert_eq!(s.players[&1].extra.breach.core.slots[2].charges, 1);
    assert!(
        c.w.effects
            .iter()
            .any(|(e, n)| *e == Entity::Player(2) && n == "minecraft:slowness")
    );
}
#[test]
fn vertical_wall_charge_can_target_through_ceiling_without_horizontal_fake_hit() {
    let (mut k, mut c) = (Breach::default(), Scene::default());
    c.w.actors[0].direction = [0., 1., 0.];
    c.w.wall = Some(Box3 {
        min: [-3., 3., -3.],
        max: [3., 4., 3.],
    });
    c.w.actors[1] = actor(2, [0., 4.2, 0.]);
    activate(&mut k, &mut c, 0);
    assert!(fire(&mut k, &mut c, false));
    assert!(c.objects[0].position[1] > 4.);
    for _ in 0..40 {
        step(&mut k, &mut c);
    }
    assert!(c.w.damage.iter().any(|(e, _, _)| *e == Entity::Player(2)));
}
#[test]
fn registry_and_hud_keep_previous_agent_indices_and_add_breach() {
    let mut state = valorant_horizon_engine::State::default();
    assert_eq!(state.registry.index("kayo"), Some(18));
    assert_eq!(state.registry.index("breach"), Some(19));
    assert!(state.select(19));
    let text = state
        .lines(&["C".into(), "Q".into(), "E".into(), "X".into()])
        .join("\n");
    for name in NAMES {
        assert!(text.contains(name));
    }
}
