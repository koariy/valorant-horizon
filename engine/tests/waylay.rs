mod support;
use support::{extra::Scene, *};
use valorant_horizon_engine::{
    advanced::{Agent, waylay::*},
    gameplay::{geometry::*, objects::*, *},
};
fn step(k: &mut Waylay, c: &mut Scene) {
    let a = c.w.actors[0].clone();
    c.step(k, &a);
    c.w.protect(1, k.protection(&a));
    for o in &mut c.objects {
        o.ticks = o.ticks.saturating_sub(1);
    }
    c.objects
        .retain(|o| o.ticks > 0 && o.health > 0. && k.owns(o.id));
}
fn activate(k: &mut Waylay, c: &mut Scene, slot: usize) -> bool {
    let a = c.w.actors[0].clone();
    k.activate(slot, &a, &mut c.ctx())
}
fn fire(k: &mut Waylay, c: &mut Scene, alt: bool) -> bool {
    let a = c.w.actors[0].clone();
    k.fire(alt, &a, &mut c.ctx())
}
fn walk(k: &mut Waylay, c: &mut Scene, positions: &[V3]) {
    for &p in positions {
        c.w.move_player(1, p);
        step(k, c);
    }
}
#[test]
fn saturate_is_immediate_ground_impact_hinder_without_damage_or_blindness() {
    let (mut k, mut c) = (Waylay::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 5.]);
    assert!(activate(&mut k, &mut c, 0));
    assert_eq!(k.core.slots[0].charges, 0);
    for _ in 0..25 {
        step(&mut k, &mut c);
    }
    assert!(
        c.w.effects
            .contains(&(Entity::Player(2), "minecraft:slowness".into()))
    );
    assert!(
        c.w.effects
            .contains(&(Entity::Player(2), "minecraft:mining_fatigue".into()))
    );
    assert!(!c.w.effects.iter().any(|(_, e)| e == "minecraft:blindness"));
    assert!(c.w.damage.is_empty());
    assert!(c.objects.is_empty());
}
#[test]
fn saturate_vertical_wall_bounces_without_early_airburst() {
    let (mut k, mut c) = (Waylay::default(), Scene::default());
    c.w.wall = Some(Box3 {
        min: [-3., 1., 2.],
        max: [3., 5., 2.2],
    });
    activate(&mut k, &mut c, 0);
    for _ in 0..4 {
        step(&mut k, &mut c);
    }
    assert!(c.w.effects.is_empty());
    assert!(c.objects[0].velocity[2] < 0.);
    assert!(c.objects[0].position[2] < 2.);
}
#[test]
fn saturate_does_not_hinder_allies_protected_phased_or_occluded_targets() {
    for mode in 0..4 {
        let (mut k, mut c) = (Waylay::default(), Scene::default());
        c.w.actors[1] = actor(2, [0., 1., 5.]);
        match mode {
            0 => {
                c.w.intel.insert(1, vec![1, 2]);
            }
            1 => c.w.protected.push(Entity::Player(2)),
            2 => {
                c.w.protect(
                    2,
                    effects::Protection {
                        invulnerable: true,
                        ..Default::default()
                    },
                );
            }
            _ => {
                c.w.actors[1] = actor(2, [2., 1., 5.]);
                c.w.wall = Some(Box3 {
                    min: [0.8, 1., 0.],
                    max: [1.2, 5., 10.],
                });
            }
        }
        activate(&mut k, &mut c, 0);
        for _ in 0..25 {
            step(&mut k, &mut c);
        }
        assert!(c.w.effects.is_empty(), "mode {mode}");
    }
}
#[test]
fn scarce_pool_does_not_spend_cluster_beacon_or_ultimate() {
    let (mut k, mut c) = (Waylay::default(), Scene::default());
    let a = c.w.actors[0].clone();
    for _ in 0..MAX_OBJECTS {
        c.ctx()
            .object(99, ObjectKind::Leer, [3., 2., 0.], 1., 100, [0.; 3]);
    }
    assert!(!activate(&mut k, &mut c, 0));
    assert!(!activate(&mut k, &mut c, 2));
    assert!(activate(&mut k, &mut c, 3));
    assert!(!k.fire(false, &a, &mut c.ctx()));
    assert_eq!(k.core.slots.map(|r| r.charges), [1; 4]);
}
#[test]
fn lightspeed_alt_one_dash_and_primary_two_dashes() {
    for (alt, expected) in [(true, 5.1), (false, 10.2)] {
        let (mut k, mut c) = (Waylay::default(), Scene::default());
        assert!(activate(&mut k, &mut c, 1));
        assert!(fire(&mut k, &mut c, alt));
        for _ in 0..20 {
            step(&mut k, &mut c);
        }
        assert!((c.w.actors[0].position[2] - expected).abs() < 0.01);
        assert!(!k.equipped());
        assert_eq!(k.core.slots[1].charges, 0);
    }
}
#[test]
fn only_first_dash_can_move_upward_second_uses_updated_horizontal_aim() {
    let (mut k, mut c) = (Waylay::default(), Scene::default());
    c.w.actors[0].direction = unit([0., 1., 1.]);
    activate(&mut k, &mut c, 1);
    fire(&mut k, &mut c, false);
    for _ in 0..6 {
        step(&mut k, &mut c);
    }
    let y = c.w.actors[0].position[1];
    c.w.actors[0].direction = unit([1., 1., 0.]);
    for _ in 0..12 {
        step(&mut k, &mut c);
    }
    assert!((c.w.actors[0].position[1] - y).abs() < 0.01);
    assert!(c.w.actors[0].position[0] > 5.);
}
#[test]
fn lightspeed_stops_at_wall_and_unknown_chunks() {
    for unknown in [false, true] {
        let (mut k, mut c) = (Waylay::default(), Scene::default());
        if unknown {
            c.w.unloaded_z = Some(3);
        } else {
            c.w.wall = Some(Box3 {
                min: [-3., 0., 3.],
                max: [3., 5., 4.],
            });
        }
        activate(&mut k, &mut c, 1);
        fire(&mut k, &mut c, true);
        for _ in 0..10 {
            step(&mut k, &mut c);
        }
        assert!(c.w.actors[0].position[2] < 2.71);
        assert!(clear_box(&c.w, c.w.actors[0].bounds));
    }
}
#[test]
fn body_sweep_does_not_tunnel_through_actor_and_budget_fails_closed() {
    let (mut k, mut c) = (Waylay::default(), Scene::default());
    let a = c.w.actors[0].clone();
    c.w.actors[1] = actor(2, [0., 1., 1.]);
    assert!(c.ctx().body_blocked(&a, [0., 0., 2.]));
    c.budget.actors = 0;
    assert!(c.ctx().body_blocked(&a, [0.1, 0., 0.]));
    activate(&mut k, &mut c, 1);
    fire(&mut k, &mut c, true);
    step(&mut k, &mut c);
    assert_eq!(c.w.actors[0].position, a.position);
}
#[test]
fn refract_requires_real_ground_and_refuses_airborne_or_unknown_anchor() {
    let (mut k, mut c) = (Waylay::default(), Scene::default());
    c.w.actors[0].ground = false;
    assert!(!activate(&mut k, &mut c, 2));
    c.w.actors[0].ground = true;
    c.w.move_player(1, [0., 3., 0.]);
    assert!(!activate(&mut k, &mut c, 2));
    c.w.move_player(1, [0., 1., 0.]);
    c.w.unloaded_z = Some(0);
    assert!(!activate(&mut k, &mut c, 2));
    assert_eq!(k.core.slots[2].charges, 1);
}
#[test]
fn refract_returns_real_body_along_recorded_path_with_temporary_protection() {
    let (mut k, mut c) = (Waylay::default(), Scene::default());
    assert!(activate(&mut k, &mut c, 2));
    walk(
        &mut k,
        &mut c,
        &[[0., 1., 1.], [0., 1., 2.], [1., 1., 2.], [2., 1., 2.]],
    );
    assert!(activate(&mut k, &mut c, 2));
    assert!(k.phased() && k.hidden());
    assert!(c.w.protections[&1].invulnerable && c.w.protections[&1].disarmed);
    for _ in 0..12 {
        step(&mut k, &mut c);
    }
    assert_eq!(c.w.actors[0].position, [0., 1., 0.]);
    assert!(!k.phased());
    assert!(!c.w.protections[&1].invulnerable);
    assert_eq!(k.core.slots[2].charges, 0);
    assert!(c.objects.is_empty());
}
#[test]
fn refract_records_dash_positions_and_can_return_after_two_dashes() {
    let (mut k, mut c) = (Waylay::default(), Scene::default());
    activate(&mut k, &mut c, 2);
    activate(&mut k, &mut c, 1);
    fire(&mut k, &mut c, false);
    for _ in 0..20 {
        step(&mut k, &mut c);
    }
    assert!(c.w.actors[0].position[2] > 10.);
    assert!(activate(&mut k, &mut c, 2));
    for _ in 0..30 {
        step(&mut k, &mut c);
    }
    assert!(distance(c.w.actors[0].position, [0., 1., 0.]) < 0.01);
}
#[test]
fn refract_new_obstruction_stops_without_clipping_or_infinite_invulnerability() {
    let (mut k, mut c) = (Waylay::default(), Scene::default());
    activate(&mut k, &mut c, 2);
    walk(&mut k, &mut c, &[[0., 1., 1.], [0., 1., 2.], [0., 1., 3.]]);
    activate(&mut k, &mut c, 2);
    c.w.wall = Some(Box3 {
        min: [-2., 1., 1.4],
        max: [2., 4., 1.6],
    });
    for _ in 0..8 {
        step(&mut k, &mut c);
    }
    assert!(c.w.actors[0].position[2] >= 1.9);
    assert!(!k.phased());
    assert!(clear_box(&c.w, c.w.actors[0].bounds));
}
#[test]
fn semantic_teleport_expiry_and_cancellation_do_not_refund_refract() {
    for mode in 0..3 {
        let (mut k, mut c) = (Waylay::default(), Scene::default());
        k.settings.beacon_ticks = 20;
        activate(&mut k, &mut c, 2);
        match mode {
            0 => {
                c.w.teleport_player(1, [0., 1., 1.]);
                step(&mut k, &mut c);
            }
            1 => {
                for _ in 0..21 {
                    step(&mut k, &mut c);
                }
            }
            _ => k.cancel(),
        }
        assert_eq!(k.stats()[1], 0);
        assert!(!k.phased());
        assert_eq!(k.core.slots[2].charges, 0);
    }
}
#[test]
fn convergent_paths_delays_then_expands_hinder_and_grants_real_speed() {
    let (mut k, mut c) = (Waylay::default(), Scene::default());
    c.w.actors[1] = actor(2, [2.5, 1., 5.]);
    activate(&mut k, &mut c, 3);
    assert!(fire(&mut k, &mut c, false));
    for _ in 0..19 {
        step(&mut k, &mut c);
    }
    assert!(c.w.effects.is_empty());
    step(&mut k, &mut c);
    assert!(
        c.w.effects
            .contains(&(Entity::Player(1), "minecraft:speed".into()))
    );
    assert!(!c.w.effects.iter().any(|(t, _)| *t == Entity::Player(2)));
    for _ in 0..15 {
        step(&mut k, &mut c);
    }
    assert!(
        c.w.effects
            .contains(&(Entity::Player(2), "minecraft:mining_fatigue".into()))
    );
    assert!(c.w.damage.is_empty());
    for _ in 0..20 {
        step(&mut k, &mut c);
    }
    assert!(c.objects.is_empty());
}
#[test]
fn convergent_paths_respects_terrain_and_phase_immunity() {
    for phase in [false, true] {
        let (mut k, mut c) = (Waylay::default(), Scene::default());
        c.w.actors[1] = actor(2, [0., 1., 5.]);
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
                min: [-3., 0., 3.],
                max: [3., 5., 4.],
            });
        }
        activate(&mut k, &mut c, 3);
        fire(&mut k, &mut c, false);
        for _ in 0..50 {
            step(&mut k, &mut c);
        }
        assert!(!c.w.effects.iter().any(|(t, _)| *t == Entity::Player(2)));
    }
}
#[test]
fn unavailable_actor_and_death_cleanup_have_no_free_charges_or_owned_objects() {
    let (mut k, mut c) = (Waylay::default(), Scene::default());
    c.w.actors[0].eligible = false;
    assert!(!activate(&mut k, &mut c, 0));
    c.w.actors[0].eligible = true;
    activate(&mut k, &mut c, 0);
    activate(&mut k, &mut c, 2);
    activate(&mut k, &mut c, 3);
    fire(&mut k, &mut c, false);
    let ids: Vec<_> = c.objects.iter().map(|o| o.id).collect();
    k.inactive_tick();
    assert!(ids.into_iter().all(|id| !k.owns(id)));
    assert_eq!(k.core.slots[2].charges, 0);
    assert!(k.busy());
}
#[test]
fn registry_menu_and_named_hud_include_waylay_without_changing_previous_indices() {
    let mut state = valorant_horizon_engine::State::default();
    assert_eq!(state.registry.index("cypher"), Some(12));
    assert_eq!(state.registry.index("iso"), Some(16));
    assert_eq!(state.registry.index("waylay"), Some(17));
    assert!(state.registry.agents().len() >= 18);
    assert!(state.select(17));
    let text = state
        .lines(&["C".into(), "Q".into(), "E".into(), "X".into()])
        .join("\n");
    for name in NAMES {
        assert!(text.contains(name));
    }
}
#[test]
fn cluster_expiry_and_unknown_chunk_leave_no_lingering_hinder() {
    for unknown in [false, true] {
        let (mut k, mut c) = (Waylay::default(), Scene::default());
        activate(&mut k, &mut c, 0);
        if unknown {
            c.w.unloaded_z = Some(1);
        } else {
            c.objects[0].ticks = 1;
        }
        for _ in 0..5 {
            step(&mut k, &mut c);
        }
        assert!(c.objects.is_empty());
        assert!(c.w.effects.is_empty());
    }
}
#[test]
fn beam_unknown_chunk_and_exhausted_ray_budget_fail_closed_without_charge_loss() {
    let (mut k, mut c) = (Waylay::default(), Scene::default());
    c.budget.rays = 0;
    activate(&mut k, &mut c, 3);
    assert!(!fire(&mut k, &mut c, false));
    assert_eq!(k.core.slots[3].charges, 1);
    c.budget = Default::default();
    c.w.unloaded_z = Some(4);
    assert!(fire(&mut k, &mut c, false));
    assert!(c.objects[0].velocity[2] <= 4.);
}
#[test]
fn conflicting_activations_cannot_break_return_protection_or_refund_dash() {
    let (mut k, mut c) = (Waylay::default(), Scene::default());
    activate(&mut k, &mut c, 2);
    walk(&mut k, &mut c, &[[0., 1., 1.], [0., 1., 2.]]);
    activate(&mut k, &mut c, 2);
    assert!(!activate(&mut k, &mut c, 0));
    assert!(!activate(&mut k, &mut c, 1));
    assert!(!activate(&mut k, &mut c, 3));
    assert!(k.phased());
    k.cancel();
    step(&mut k, &mut c);
    assert!(!k.phased());
    assert!(!c.w.protections[&1].invulnerable);
    assert!(c.objects.is_empty());
    assert_eq!(k.core.slots[2].charges, 0);
}
