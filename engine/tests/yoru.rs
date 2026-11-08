mod support;
use support::{extra::Scene, *};
use valorant_horizon_engine::{
    advanced::{Agent, Expansion, yoru::*},
    gameplay::{geometry::*, objects::*, projectiles::*, *},
    server::{Player, Session},
};
fn cast(k: &mut Yoru, c: &mut Scene, slot: usize, alt: bool) {
    let a = c.w.actors[0].clone();
    assert!(k.activate(slot, &a, &mut c.ctx()));
    assert!(k.fire(alt, &a, &mut c.ctx()));
}
#[test]
fn fakeout_moves_with_real_collision_and_dormant_echo_activates_by_use() {
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    cast(&mut k, &mut c, 0, false);
    let a = c.w.actors[0].clone();
    let before = c.objects[0].position;
    c.step(&mut k, &a);
    assert!(c.objects[0].position[2] > before[2]);
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    cast(&mut k, &mut c, 0, true);
    let mut a = c.w.actors[0].clone();
    a.direction = unit(sub(c.objects[0].position, a.eye()));
    assert!(matches!(c.objects[0].kind, ObjectKind::DormantEcho));
    assert!(k.recall(&a, &mut c.ctx()));
    assert!(matches!(c.objects[0].kind, ObjectKind::Echo));
    assert_eq!(k.core.slots[0].charges, 0);
}
#[test]
fn enemy_destroyed_echo_flashes_attacker_after_delay_not_teammates_or_back_cone() {
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., -2.]);
    c.w.actors[1].direction = [0., 0., 1.];
    c.w.actors.push(actor(3, [0., 1., -1.]));
    let mut behind = actor(4, [0., 1., 4.]);
    behind.direction = [0., 0., -1.];
    c.w.actors.push(behind);
    c.w.intel.insert(1, vec![1, 3]);
    cast(&mut k, &mut c, 0, false);
    let from = c.w.actors[1].eye();
    c.objects[0].damage(4., Some(Entity::Player(2)), from);
    let a = c.w.actors[0].clone();
    c.step(&mut k, &a);
    assert!(c.vision.is_empty());
    for _ in 0..12 {
        c.step(&mut k, &a);
    }
    assert!(c.vision.get(&2).is_some_and(|v| v.flash > 0));
    assert!(!c.vision.contains_key(&1));
    assert!(!c.vision.contains_key(&3));
    assert!(!c.vision.contains_key(&4));
}
#[test]
fn own_or_allied_damage_and_lifetime_expiration_do_not_flash() {
    for mode in 0..3 {
        let (mut k, mut c) = (Yoru::default(), Scene::default());
        c.w.intel.insert(1, vec![1, 2]);
        cast(&mut k, &mut c, 0, false);
        let a = c.w.actors[0].clone();
        if mode == 2 {
            c.objects[0].ticks = 0;
        } else {
            c.objects[0].damage(
                10.,
                Some(Entity::Player(if mode == 0 { 1 } else { 2 })),
                a.eye(),
            );
        }
        for _ in 0..20 {
            c.step(&mut k, &a);
        }
        assert!(c.vision.is_empty());
    }
}
#[test]
fn utility_damage_journal_is_private_and_preserves_ai_target() {
    let mut c = Scene::default();
    let mut k = Yoru::default();
    cast(&mut k, &mut c, 0, false);
    c.objects[0].target = Some(Entity::Mob(99));
    c.objects[0].damage(1., Some(Entity::Player(2)), [3., 2., 1.]);
    assert_eq!(c.objects[0].target, Some(Entity::Mob(99)));
    assert_eq!(
        c.objects[0].last_hit.as_ref().unwrap().attacker,
        Some(Entity::Player(2))
    );
    let json = serde_json::to_string(&c.objects[0]).unwrap();
    assert!(!json.contains("last_hit"));
    let decoded: Object = serde_json::from_str(&json).unwrap();
    assert!(decoded.last_hit.is_none());
}
#[test]
fn melee_and_projectile_record_author_and_preserve_dead_echo_until_owner_tick() {
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    cast(&mut k, &mut c, 0, false);
    c.w.actors[1] = actor(2, [2., 1., 0.8]);
    c.w.actors[1].direction = [-1., 0., 0.];
    let shooter = c.w.actors[1].clone();
    assert!(objects::strike(&mut c.objects, &shooter, &c.w));
    assert_eq!(c.objects.len(), 1);
    assert_eq!(c.objects[0].health, 0.);
    assert_eq!(
        c.objects[0].last_hit.as_ref().unwrap().attacker,
        Some(shooter.entity)
    );
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    cast(&mut k, &mut c, 0, false);
    c.w.actors[1] = shooter.clone();
    assert!(projectiles::intercept(
        &c.w,
        &mut [],
        &[],
        &mut c.objects,
        Shot {
            attacker: Some(shooter.entity),
            from: shooter.eye(),
            to: add(shooter.eye(), [-4., 0., 0.]),
            damage: 4.,
            padding: Default::default()
        }
    ));
    assert_eq!(c.objects[0].last_hit.as_ref().unwrap().from, shooter.eye());
    assert_eq!(c.objects[0].health, 0.);
}
#[test]
fn jett_knife_uses_common_author_and_no_duplicate_box_hit() {
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    cast(&mut k, &mut c, 0, false);
    c.w.actors[1] = actor(2, [2., 1., 0.8]);
    let from = c.w.actors[1].eye();
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 15,
            extra: Expansion {
                yoru: k,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    s.objects = c.objects;
    s.projectiles.push(Projectile {
        owner: 2,
        kind: ProjectileKind::Knife { alternate: false },
        position: from,
        velocity: [-3., 0., 0.],
        ticks: 10,
        guided: false,
        victims: vec![],
    });
    s.tick(&mut c.w);
    let echo = s
        .objects
        .iter()
        .find(|o| matches!(o.kind, ObjectKind::Echo))
        .unwrap();
    assert_eq!(echo.health, 0.);
    assert_eq!(
        echo.last_hit.as_ref().unwrap().attacker,
        Some(Entity::Player(2))
    );
}
#[test]
fn explosion_credits_echo_destroyer_and_keeps_event_for_next_tick() {
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    cast(&mut k, &mut c, 0, false);
    let at = c.objects[0].position;
    explosions::apply(
        &mut c.w,
        &mut c.objects,
        explosions::Explosion {
            owner: 2,
            position: add(at, [0., 0.5, 0.]),
            radius: 3.,
            damage: 10.,
            push: 0.,
            self_damage: false,
        },
    );
    assert_eq!(c.objects.len(), 1);
    assert_eq!(c.objects[0].health, 0.);
    assert_eq!(
        c.objects[0].last_hit.as_ref().unwrap().attacker,
        Some(Entity::Player(2))
    );
}
#[test]
fn blindside_requires_hard_surface_then_windup_and_line_of_sight() {
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    c.w.actors[1] = actor(2, [1., 1., 2.]);
    c.w.actors[1].direction = [-1., 0., -1.];
    cast(&mut k, &mut c, 1, false);
    let a = c.w.actors[0].clone();
    for _ in 0..10 {
        c.step(&mut k, &a);
    }
    assert!(c.vision.is_empty());
    // A fresh throw aimed at a hard surface, not an invented air detonation.
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., -6.]);
    c.w.actors[1].direction = [0., 0., 1.];
    c.w.wall = Some(Box3 {
        min: [-2., 0., 1.5],
        max: [2., 5., 2.],
    });
    cast(&mut k, &mut c, 1, false);
    let a = c.w.actors[0].clone();
    c.step(&mut k, &a);
    assert!(c.vision.is_empty());
    for _ in 0..15 {
        c.step(&mut k, &a);
    }
    assert!(c.vision.get(&2).is_some_and(|v| v.flash > 0));
}
#[test]
fn stationary_gate_teleports_body_and_records_semantic_event_once() {
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    cast(&mut k, &mut c, 2, true);
    c.objects[0].position = [0., 1., 6.];
    let a = c.w.actors[0].clone();
    assert!(k.activate(2, &a, &mut c.ctx()));
    for _ in 0..8 {
        c.step(&mut k, &a);
    }
    assert_eq!(c.w.actors[0].position, [0., 1., 6.]);
    assert_eq!(c.w.teleport_stamp(a.entity), 1);
    assert_eq!(k.core.slots[2].charges, 1);
    assert_eq!(k.stats()[1], 0);
}
#[test]
fn moving_gate_advances_on_loaded_ground_and_fake_use_never_teleports() {
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    cast(&mut k, &mut c, 2, false);
    let a = c.w.actors[0].clone();
    let before = c.objects[0].position;
    c.step(&mut k, &a);
    assert!(c.objects[0].position[2] > before[2]);
    assert!(k.recall(&a, &mut c.ctx()));
    assert_eq!(c.w.actors[0].position, a.position);
    assert_eq!(c.w.teleport_stamp(a.entity), 0);
    assert_eq!(k.stats()[1], 0);
    assert!(!c.beams.is_empty());
}
#[test]
fn destroyed_gate_or_blocked_destination_cannot_teleport() {
    for destroy in [false, true] {
        let (mut k, mut c) = (Yoru::default(), Scene::default());
        cast(&mut k, &mut c, 2, true);
        c.objects[0].position = [0., 1., 6.];
        let a = c.w.actors[0].clone();
        k.activate(2, &a, &mut c.ctx());
        if destroy {
            c.objects[0].damage(10., Some(Entity::Player(2)), [1., 2., 6.]);
        } else {
            c.w.wall = Some(Box3 {
                min: [-1., 0., 5.8],
                max: [1., 4., 6.8],
            });
        }
        for _ in 0..10 {
            c.step(&mut k, &a);
        }
        assert_eq!(c.w.actors[0].position, a.position);
        assert_eq!(c.w.teleport_stamp(a.entity), 0);
        assert_eq!(k.core.slots[2].charges, 1);
    }
}
#[test]
fn removing_dead_gate_during_cast_does_not_leave_ghost_teleport() {
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    cast(&mut k, &mut c, 2, true);
    c.objects[0].position = [0., 1., 6.];
    let a = c.w.actors[0].clone();
    k.activate(2, &a, &mut c.ctx());
    c.objects[0].health = 0.;
    assert!(!k.recall(&a, &mut c.ctx()));
    for _ in 0..10 {
        c.step(&mut k, &a);
    }
    assert_eq!(c.w.actors[0].position, a.position);
}
#[test]
fn dimensional_drift_is_protected_hidden_unarmed_and_exits_without_refund() {
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    cast(&mut k, &mut c, 3, false);
    let a = c.w.actors[0].clone();
    assert!(k.hidden() && k.phased() && k.protection(&a).invulnerable && k.protection(&a).disarmed);
    assert!(c.w.status_immune(a.entity));
    c.ctx().flash([0., 2.62, 2.], 16., 50);
    assert!(!c.vision.contains_key(&1));
    assert!(k.activate(3, &a, &mut c.ctx()));
    assert!(!k.hidden());
    assert!(!c.w.status_immune(a.entity));
    assert_eq!(k.core.slots[3].charges, 0);
    assert!(k.core.slots[3].remaining > 0);
}
#[test]
fn old_phoenix_flash_and_astra_nova_do_not_affect_drift() {
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    cast(&mut k, &mut c, 3, false);
    let a = c.w.actors[0].clone();
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 15,
            extra: Expansion {
                yoru: k,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    s.projectiles.push(Projectile {
        owner: 2,
        kind: ProjectileKind::Curveball { right: false },
        position: add(a.eye(), [0., 0., 2.]),
        velocity: [0.; 3],
        ticks: 1,
        guided: false,
        victims: vec![],
    });
    s.zones.push(zones::Zone {
        owner: 2,
        kind: zones::ZoneKind::Nova,
        bounds: Box3 {
            min: [-3., 0., -3.],
            max: [3., 4., 3.],
        },
        ticks: 1,
    });
    s.tick(&mut c.w);
    let v = s.snapshot(1).unwrap().vision;
    assert_eq!(v.flash, 0);
    assert_eq!(v.nearsight, 0);
    assert!(
        !c.w.effects
            .iter()
            .any(|(entity, n)| *entity == a.entity && effects::hostile_status(n))
    );
}
#[test]
fn ability_hitscan_and_temporary_projectiles_pass_drift_body() {
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    cast(&mut k, &mut c, 3, false);
    c.w.actors[1] = actor(2, [0., 1., -3.]);
    c.w.actors.push(actor(3, [0., 1., 3.]));
    let from = c.w.actors[1].eye();
    c.ctx().hitscan(2, from, add(from, [0., 0., 8.]), 4.);
    assert_eq!(c.w.actors[0].health, 20.);
    assert_eq!(c.w.actors[2].health, 16.);
    let mut p = Projectile {
        owner: 2,
        kind: ProjectileKind::Knife { alternate: false },
        position: from,
        velocity: [0., 0., 8.],
        ticks: 10,
        guided: false,
        victims: vec![],
    };
    assert!(matches!(
        p.advance(&c.w),
        Some(Impact::Entity(Entity::Player(3), _, _))
    ));
}
#[test]
fn hidden_entity_list_is_per_enemy_and_includes_only_active_phase() {
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    cast(&mut k, &mut c, 3, false);
    c.w.actors.push(actor(3, [4., 1., 0.]));
    c.w.intel.insert(1, vec![1, 3]);
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 15,
            extra: Expansion {
                yoru: k,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    s.players.insert(2, Player::default());
    s.players.insert(3, Player::default());
    s.tick(&mut c.w);
    assert!(s.snapshot(1).unwrap().hidden_entities.is_empty());
    assert_eq!(s.snapshot(2).unwrap().hidden_entities, vec![1]);
    assert!(s.snapshot(3).unwrap().hidden_entities.is_empty());
}
#[test]
fn cancellation_and_death_remove_devices_while_cooldowns_keep_running() {
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    cast(&mut k, &mut c, 0, false);
    cast(&mut k, &mut c, 2, true);
    cast(&mut k, &mut c, 3, false);
    k.cancel();
    assert_eq!(k.phase, 0);
    assert_eq!(k.stats()[0], 1);
    assert_eq!(k.core.slots[3].charges, 0);
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 15,
            extra: Expansion {
                yoru: k,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    s.objects = c.objects;
    c.w.actors[0].eligible = false;
    c.w.actors[0].health = 0.;
    s.tick(&mut c.w);
    assert!(s.objects.is_empty());
    assert!(s.hidden.is_empty());
    assert!(s.players[&1].extra.yoru.core.slots[3].remaining > 0);
}
#[test]
fn unknown_spawn_or_capacity_refusal_cannot_spend_charge() {
    let (mut k, mut c) = (Yoru::default(), Scene::default());
    c.w.unloaded_z = Some(0);
    let a = c.w.actors[0].clone();
    k.activate(2, &a, &mut c.ctx());
    assert!(!k.fire(false, &a, &mut c.ctx()));
    assert_eq!(k.core.slots[2].charges, 2);
}
mod native_helpers {
    include!("../../helpers/phase_hide.rs");
    include!("../../helpers/phase_names_enable.rs");
    include!("../../helpers/incoming.rs");
    include!("../../helpers/phase_name.rs");
    include!("../../helpers/phase_hears.rs");
    include!("../../helpers/phase_impact.rs");
    include!("../../helpers/recon_outline.rs");
    include!("../../helpers/astra_hand.rs");
    pub fn enable(v: bool) -> bool {
        valorant_phase_names_enable(v)
    }
    pub fn damage(entity: i32, ty: &str, causing: i32, direct: i32, c: &mut bool) {
        valorant_protection(entity, ty, causing, direct, c)
    }
    pub fn hidden(v: bool, id: u32) -> bool {
        valorant_phase_hide(v, id)
    }
    pub fn name(id: u32, v: &mut bool) {
        valorant_phase_name(id, v)
    }
    pub fn outline(id: u32, v: &mut Option<u32>) {
        recon_outline(id, v)
    }
    pub fn hand(v: &mut bool) {
        valorant_hand(v)
    }
    pub fn hears(v: &mut bool) {
        valorant_phase_hears(v)
    }
    pub fn impact(id: Option<u32>, v: &mut bool) {
        valorant_phase_impact(id, v)
    }
}
#[test]
fn actual_native_helpers_hide_equipment_name_outline_hand_and_cancel_entity_impact() {
    let mut s = Session::default();
    s.players.insert(1, Player::default());
    let mut snapshot = s.snapshot(1).unwrap();
    snapshot.hidden_entities = vec![98765];
    snapshot.vision.phased = true;
    let previous = valorant_horizon_engine::state()
        .lock()
        .unwrap()
        .snapshot
        .replace(snapshot);
    assert!(native_helpers::enable(false));
    assert!(native_helpers::hidden(false, 98765));
    assert!(native_helpers::hidden(true, 2));
    assert!(!native_helpers::hidden(false, 2));
    let mut name = true;
    native_helpers::name(98765, &mut name);
    assert!(!name);
    let mut outline = Some(0xffffff);
    native_helpers::outline(98765, &mut outline);
    assert!(outline.is_none());
    let mut hand = false;
    native_helpers::hand(&mut hand);
    assert!(hand);
    effects::protections().lock().unwrap().insert(
        98765,
        effects::Protection {
            invulnerable: true,
            ..Default::default()
        },
    );
    let mut hears = false;
    native_helpers::hears(&mut hears);
    assert!(hears);
    let mut canceled = false;
    native_helpers::impact(Some(98765), &mut canceled);
    assert!(canceled);
    let mut block = false;
    native_helpers::impact(None, &mut block);
    assert!(!block);
    let mut incoming = false;
    native_helpers::damage(98765, "minecraft:arrow", 0, 0, &mut incoming);
    assert!(incoming);
    let mut void = false;
    native_helpers::damage(98765, "minecraft:fell_out_of_world", 0, 0, &mut void);
    assert!(!void);
    effects::protections().lock().unwrap().insert(
        0,
        effects::Protection {
            disarmed: true,
            ..Default::default()
        },
    );
    let mut environment = false;
    native_helpers::damage(45678, "minecraft:fall", 0, 0, &mut environment);
    assert!(!environment);
    effects::protections().lock().unwrap().remove(&0);
    effects::protections().lock().unwrap().remove(&98765);
    valorant_horizon_engine::state().lock().unwrap().snapshot = previous;
}
