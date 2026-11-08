mod support;
use support::{extra::Scene, *};
use valorant_horizon_engine::{
    advanced::{Agent, Expansion, iso::*},
    gameplay::{geometry::*, objects::*, projectiles::*, *},
    server::{Player, Session},
};
fn activate(k: &mut Iso, c: &mut Scene, slot: usize) {
    let a = c.w.actors[0].clone();
    assert!(k.activate(slot, &a, &mut c.ctx()));
}
fn flow(k: &mut Iso, c: &mut Scene) {
    activate(k, c, 2);
    let a = c.w.actors[0].clone();
    for _ in 0..20 {
        c.step(k, &a);
    }
    assert!(k.flow > 0);
}
fn orb(k: &mut Iso, c: &mut Scene) {
    k.fallen(Entity::Mob(30), [0., 2., 4.], true);
    let a = c.w.actors[0].clone();
    c.step(k, &a);
}
fn grant(k: &mut Iso, c: &mut Scene) {
    flow(k, c);
    orb(k, c);
    let a = c.w.actors[0].clone();
    c.objects
        .iter_mut()
        .find(|o| matches!(o.kind, ObjectKind::IsoOrb))
        .unwrap()
        .damage_with(1., Some(a.entity), a.eye(), UtilityCause::Shot);
    c.step(k, &a);
    assert!(k.shield);
}
#[test]
fn focus_precedes_flow_without_free_shield_and_cancel_keeps_spent_charge() {
    let (mut k, mut c) = (Iso::default(), Scene::default());
    activate(&mut k, &mut c, 2);
    assert_eq!(k.focus, 20);
    assert_eq!(k.flow, 0);
    assert!(!k.shield);
    let a = c.w.actors[0].clone();
    for _ in 0..19 {
        c.step(&mut k, &a);
    }
    assert_eq!(k.flow, 0);
    c.step(&mut k, &a);
    assert_eq!(k.flow, 300);
    k.cancel();
    assert_eq!(k.flow, 0);
    assert_eq!(k.core.slots[2].charges, 0);
    assert!(k.core.slots[2].remaining > 0);
}
#[test]
fn only_flow_kills_or_confirmed_assists_generate_orbs() {
    let (mut k, mut c) = (Iso::default(), Scene::default());
    k.fallen(Entity::Mob(30), [0., 2., 4.], true);
    let a = c.w.actors[0].clone();
    c.step(&mut k, &a);
    assert!(c.objects.is_empty());
    flow(&mut k, &mut c);
    k.damage(Entity::Mob(31));
    k.fallen(Entity::Mob(31), [0., 2., 4.], false);
    k.fallen(Entity::Mob(32), [0., 2., 4.], false);
    c.step(&mut k, &a);
    assert_eq!(k.stats()[3], 1);
}
#[test]
fn server_routes_positive_hits_and_confirmed_death_into_private_orb() {
    let mut w = TestWorld::default();
    w.actors[1].health = 0.;
    w.actors[1].eligible = false;
    w.credits.push((1, Entity::Player(2)));
    let mut kit = Iso::default();
    kit.flow = 100;
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 16,
            extra: Expansion {
                iso: kit,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    s.players.insert(2, Player::default());
    s.queue_death(3, Entity::Player(2), [0., 1., 3.], false);
    s.tick(&mut w);
    assert_eq!(s.players[&1].extra.iso.stats()[3], 1);
    assert!(
        s.snapshot(1)
            .unwrap()
            .objects
            .iter()
            .any(|o| matches!(o.kind, ObjectKind::IsoOrb))
    );
    assert!(
        !s.snapshot(2)
            .unwrap()
            .objects
            .iter()
            .any(|o| matches!(o.kind, ObjectKind::IsoOrb))
    );
}
#[test]
fn only_owner_shot_can_damage_orb_melee_explosion_and_enemy_shot_cannot() {
    let (mut k, mut c) = (Iso::default(), Scene::default());
    flow(&mut k, &mut c);
    orb(&mut k, &mut c);
    let index = c
        .objects
        .iter()
        .position(|o| matches!(o.kind, ObjectKind::IsoOrb))
        .unwrap();
    let a = c.w.actors[0].clone();
    for (who, cause) in [
        (Entity::Player(2), UtilityCause::Shot),
        (a.entity, UtilityCause::Melee),
        (a.entity, UtilityCause::Explosion),
    ] {
        c.objects[index].damage_with(100., Some(who), a.eye(), cause);
        assert_eq!(c.objects[index].health, 1.);
    }
    assert!(
        c.objects[index]
            .hit_by(a.eye(), [0., 2.5, 8.], 0.1, Some(Entity::Player(2)))
            .is_none()
    );
    c.objects[index].damage_with(1., Some(a.entity), a.eye(), UtilityCause::Shot);
    c.step(&mut k, &a);
    assert!(k.shield);
    assert!(c.w.single_hit_shield(1));
}
#[test]
fn real_utility_shot_path_grants_shield_and_orb_is_not_melee_target() {
    let (mut k, mut c) = (Iso::default(), Scene::default());
    flow(&mut k, &mut c);
    orb(&mut k, &mut c);
    let a = c.w.actors[0].clone();
    let at = c
        .objects
        .iter()
        .find(|o| matches!(o.kind, ObjectKind::IsoOrb))
        .unwrap()
        .position;
    let to = add(at, [0., 0.2, 0.]);
    assert!(projectiles::intercept(
        &c.w,
        &mut [],
        &[],
        &mut c.objects,
        Shot {
            attacker: Some(a.entity),
            from: a.eye(),
            to,
            damage: 4.,
            padding: Default::default()
        }
    ));
    c.step(&mut k, &a);
    assert!(k.shield);
}
#[test]
fn expired_orb_and_expired_assist_cannot_grant_reward() {
    let (mut k, mut c) = (Iso::default(), Scene::default());
    flow(&mut k, &mut c);
    k.damage(Entity::Mob(31));
    let a = c.w.actors[0].clone();
    for _ in 0..201 {
        c.step(&mut k, &a);
    }
    k.fallen(Entity::Mob(31), [0., 2., 4.], false);
    c.step(&mut k, &a);
    assert_eq!(k.stats()[3], 0);
    orb(&mut k, &mut c);
    let o = c
        .objects
        .iter_mut()
        .find(|o| matches!(o.kind, ObjectKind::IsoOrb))
        .unwrap();
    o.ticks = 0;
    c.step(&mut k, &a);
    assert!(!k.shield);
}
#[test]
fn shield_consumption_is_not_restored_by_duplicate_native_sync() {
    let (mut k, mut c) = (Iso::default(), Scene::default());
    grant(&mut k, &mut c);
    let a = c.w.actors[0].clone();
    let stale = k.protection(&a);
    c.w.protections.get_mut(&1).unwrap().one_hit = false;
    c.w.protect(1, stale);
    assert!(!c.w.single_hit_shield(1));
    c.step(&mut k, &a);
    assert!(!k.shield);
    assert!(!c.w.single_hit_shield(1));
}
#[test]
fn a_new_orb_grants_a_new_shield_epoch_after_consumption() {
    let (mut k, mut c) = (Iso::default(), Scene::default());
    grant(&mut k, &mut c);
    let epoch = c.w.protections[&1].shield_epoch;
    c.w.protections.get_mut(&1).unwrap().one_hit = false;
    let a = c.w.actors[0].clone();
    c.step(&mut k, &a);
    orb(&mut k, &mut c);
    c.objects
        .iter_mut()
        .filter(|o| matches!(o.kind, ObjectKind::IsoOrb) && o.health > 0.)
        .for_each(|o| o.damage_with(1., Some(a.entity), a.eye(), UtilityCause::Shot));
    c.step(&mut k, &a);
    assert!(k.shield);
    assert!(c.w.protections[&1].shield_epoch > epoch);
}
#[test]
fn shield_has_finite_duration_and_does_not_make_actor_globally_invulnerable() {
    let (mut k, mut c) = (Iso::default(), Scene::default());
    grant(&mut k, &mut c);
    let a = c.w.actors[0].clone();
    assert!(!c.w.status_immune(a.entity));
    for _ in 0..200 {
        c.step(&mut k, &a);
    }
    assert!(!k.shield);
    assert!(!c.w.single_hit_shield(1));
}
#[test]
fn undercut_passes_known_walls_and_applies_fragile_to_allies_too() {
    let (mut k, mut c) = (Iso::default(), Scene::default());
    c.w.actors[1] = actor(2, [0., 1., 5.]);
    c.w.intel.insert(1, vec![1, 2]);
    c.w.wall = Some(Box3 {
        min: [-2., 0., 2.],
        max: [2., 5., 3.],
    });
    activate(&mut k, &mut c, 1);
    let a = c.w.actors[0].clone();
    assert!(k.fire(false, &a, &mut c.ctx()));
    for _ in 0..8 {
        c.step(&mut k, &a);
    }
    assert!(c.vulnerable.contains_key(&Entity::Player(2)));
    assert!(c.w.vulnerable_targets.contains(&Entity::Player(2)));
    assert!(!c.vulnerable.contains_key(&a.entity));
}
#[test]
fn undercut_stops_at_unknown_chunks_and_skips_phased_actors() {
    for phase in [false, true] {
        let (mut k, mut c) = (Iso::default(), Scene::default());
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
            c.w.unloaded_z = Some(3);
        }
        activate(&mut k, &mut c, 1);
        let a = c.w.actors[0].clone();
        k.fire(false, &a, &mut c.ctx());
        for _ in 0..8 {
            c.step(&mut k, &a);
        }
        assert!(c.vulnerable.is_empty());
    }
}
#[test]
fn contingency_moves_and_blocks_shots_indestructibly_with_finite_bounds() {
    let (mut k, mut c) = (Iso::default(), Scene::default());
    activate(&mut k, &mut c, 0);
    let a = c.w.actors[0].clone();
    assert!(k.fire(false, &a, &mut c.ctx()));
    let before = c.objects[0].position;
    c.step(&mut k, &a);
    assert!(c.objects[0].position[2] > before[2]);
    let wall = c.objects[0].clone();
    assert!(wall.hit([-1., 2., -2.], [-1., 2., 5.], 0.).is_some());
    assert!(wall.hit([3., 2., -2.], [3., 2., 5.], 0.).is_none());
    assert!(wall.hit([0., 5., -2.], [0., 5., 5.], 0.).is_none());
    assert!(projectiles::intercept(
        &c.w,
        &mut [],
        &[],
        &mut c.objects,
        Shot {
            attacker: Some(Entity::Player(2)),
            from: [-1., 2., -2.],
            to: [-1., 2., 5.],
            damage: 100000.,
            padding: Default::default()
        }
    ));
    assert_eq!(c.objects[0].health, 1.);
}
#[test]
fn finite_wall_blocks_hitscan_but_allows_melee_and_stops_before_terrain() {
    let (mut k, mut c) = (Iso::default(), Scene::default());
    activate(&mut k, &mut c, 0);
    let a = c.w.actors[0].clone();
    k.fire(false, &a, &mut c.ctx());
    c.w.actors[1] = actor(2, [0., 1., 5.]);
    c.ctx()
        .hitscan(1, a.eye(), add(a.eye(), [0., 0., 8.]), 100.);
    assert_eq!(c.w.actors[1].health, 20.);
    assert!(!objects::strike(&mut c.objects, &a, &c.w));
    c.w.wall = Some(Box3 {
        min: [-3., 0., 2.],
        max: [3., 5., 3.],
    });
    for _ in 0..15 {
        c.step(&mut k, &a);
    }
    assert!(c.objects[0].bounds().max[2] <= 2.);
}
#[test]
fn kill_contract_is_explicitly_unavailable_without_fake_arena_or_spending() {
    let (mut k, mut c) = (Iso::default(), Scene::default());
    let a = c.w.actors[0].clone();
    assert_eq!(DEFINITIONS[3].charges, 0);
    assert!(!k.activate(3, &a, &mut c.ctx()));
    assert!(!k.fire(false, &a, &mut c.ctx()));
    assert!(c.objects.is_empty());
    assert_eq!(c.w.actors[0].position, a.position);
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 16,
            ..Default::default()
        },
    );
    s.enqueue(
        1,
        Request {
            sequence: 1,
            expected_agent: 16,
            action: Action::Activate(3),
        },
    );
    s.tick(&mut c.w);
    assert!(
        s.snapshot(1)
            .unwrap()
            .error
            .unwrap()
            .contains("isolated arena")
    );
}
#[test]
fn death_and_dimension_leave_cleanup_all_utilities_and_shield_not_cooldown() {
    let (mut k, mut c) = (Iso::default(), Scene::default());
    grant(&mut k, &mut c);
    activate(&mut k, &mut c, 0);
    let a = c.w.actors[0].clone();
    k.fire(false, &a, &mut c.ctx());
    let mut s = Session::default();
    s.players.insert(
        1,
        Player {
            agent: 16,
            extra: Expansion {
                iso: k,
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
    assert!(!s.players[&1].extra.iso.shield);
    assert!(s.players[&1].extra.iso.core.slots[2].remaining > 0);
}
#[test]
fn scarce_resources_never_refill_charges_or_create_unbounded_orbs() {
    let (mut k, mut c) = (Iso::default(), Scene::default());
    flow(&mut k, &mut c);
    for n in 0..10000 {
        k.fallen(Entity::Mob(n), [0., 2., 4.], true);
    }
    let a = c.w.actors[0].clone();
    c.step(&mut k, &a);
    assert!(k.stats()[3] <= 4);
    assert!(c.objects.len() <= MAX_OBJECTS);
    c.w.wall = Some(Box3 {
        min: [-3., 0., 0.5],
        max: [3., 5., 3.],
    });
    activate(&mut k, &mut c, 0);
    assert!(!k.fire(false, &a, &mut c.ctx()));
    assert_eq!(k.core.slots[0].charges, 1);
}
mod native {
    include!("../../helpers/incoming.rs");
    pub fn damage(entity: i32, ty: &str, causing: i32, direct: i32, amount: f32, c: &mut bool) {
        valorant_protection(entity, ty, causing, direct, amount, c)
    }
}
#[test]
fn actual_native_shield_blocks_exactly_one_positive_instance_from_any_source() {
    const ID: i32 = 654321;
    for source in [
        "minecraft:arrow",
        "minecraft:player_attack",
        "minecraft:explosion",
        "minecraft:in_fire",
        "minecraft:fall",
        "minecraft:fell_out_of_world",
    ] {
        effects::protections().lock().unwrap().insert(
            ID,
            effects::Protection {
                one_hit: true,
                shield_epoch: 1,
                ..Default::default()
            },
        );
        let mut zero = false;
        native::damage(ID, source, 0, 0, 0., &mut zero);
        assert!(!zero);
        assert!(effects::protections().lock().unwrap()[&ID].one_hit);
        let mut first = false;
        native::damage(ID, source, 0, 0, 100000., &mut first);
        assert!(first);
        let mut next = false;
        native::damage(ID, source, 0, 0, 1., &mut next);
        assert!(!next);
    }
    effects::protections().lock().unwrap().remove(&ID);
}
#[test]
fn rejected_disarmed_or_already_cancelled_hits_do_not_consume_shield() {
    const ID: i32 = 654322;
    const ATTACKER: i32 = 654323;
    effects::protections().lock().unwrap().insert(
        ID,
        effects::Protection {
            one_hit: true,
            shield_epoch: 1,
            ..Default::default()
        },
    );
    effects::protections().lock().unwrap().insert(
        ATTACKER,
        effects::Protection {
            disarmed: true,
            ..Default::default()
        },
    );
    let mut canceled = false;
    native::damage(
        ID,
        "minecraft:player_attack",
        ATTACKER,
        ATTACKER,
        4.,
        &mut canceled,
    );
    assert!(canceled);
    assert!(effects::protections().lock().unwrap()[&ID].one_hit);
    let mut external = true;
    native::damage(ID, "minecraft:arrow", 0, 0, 4., &mut external);
    assert!(effects::protections().lock().unwrap()[&ID].one_hit);
    effects::protections().lock().unwrap().remove(&ID);
    effects::protections().lock().unwrap().remove(&ATTACKER);
}
