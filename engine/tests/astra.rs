mod support;
use support::*;
use valorant_horizon_engine::{
    agents::astra::*,
    gameplay::{geometry::*, remote_view::RemoteView, zones::*, *},
    server::Session,
};
fn star(owner: u128, p: V3) -> Star {
    Star {
        owner,
        position: p,
        ticks: 1200,
    }
}
#[test]
fn form_is_separate_from_ultimate_and_cancel_preserves_stars() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let mut kit = Astra::default();
    let (mut s, mut z, mut d) = (vec![], vec![], vec![]);
    let mut c = Context {
        world: &mut w,
        smokes: &mut s,
        zones: &mut z,
        divides: &mut d,
    };
    assert!(kit.activate(3, &a, &mut c));
    assert_eq!(kit.slots[3].charges, 1);
    kit.tick(&a, c.world);
    assert_eq!(c.world.actors[0].position, a.position);
    assert!(kit.view.is_some());
    kit.stars.push(star(1, [0., 1., 3.]));
    kit.available = 4;
    kit.cancel();
    assert_eq!(kit.available, 4);
    assert_eq!(kit.stars.len(), 1);
    assert!(kit.view.is_none());
}
#[test]
fn placements_use_one_shared_star_pool() {
    let mut w = TestWorld::default();
    let mut a = w.actors[0].clone();
    a.direction = unit([0., -1., 1.]);
    let mut kit = Astra {
        view: RemoteView::new(&w, [0., 3., 0.], 32.),
        ..Default::default()
    };
    kit.view.as_mut().unwrap().look(a.direction);
    let (mut s, mut z, mut d) = (vec![], vec![], vec![]);
    let mut c = Context {
        world: &mut w,
        smokes: &mut s,
        zones: &mut z,
        divides: &mut d,
    };
    for _ in 0..5 {
        assert!(kit.fire(false, &a, &mut c));
    }
    assert!(!kit.fire(false, &a, &mut c));
    assert_eq!(kit.available, 0);
    assert_eq!(kit.stars.len(), 5);
}
#[test]
fn dissipate_returns_only_after_delay_and_has_short_smoke() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let mut kit = Astra {
        available: 4,
        stars: vec![star(1, add(a.eye(), [0., 0., 3.]))],
        ..Default::default()
    };
    let mut smoke = vec![];
    assert!(kit.dissipate(&a, &mut smoke));
    assert_eq!(smoke[0].ticks, 20);
    for _ in 0..79 {
        kit.tick(&a, &mut w);
    }
    assert_eq!(kit.available, 4);
    kit.tick(&a, &mut w);
    assert_eq!(kit.available, 5);
}
#[test]
fn nebula_uses_slot_charge_and_spent_star_returns_separately() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let mut kit = Astra {
        available: 4,
        stars: vec![star(1, add(a.eye(), [0., 0., 3.]))],
        ..Default::default()
    };
    let (mut s, mut z, mut d) = (vec![], vec![], vec![]);
    assert!(kit.activate(
        2,
        &a,
        &mut Context {
            world: &mut w,
            smokes: &mut s,
            zones: &mut z,
            divides: &mut d
        }
    ));
    assert_eq!(s[0].ticks, 280);
    assert_eq!(kit.slots[2].charges, 1);
    assert_eq!(kit.available, 4);
    assert!(kit.stars.is_empty());
}
#[test]
fn gravity_pulls_then_applies_vulnerability_and_nova_waits() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.zones.push(Zone {
        owner: 1,
        kind: ZoneKind::Gravity,
        bounds: Box3 {
            min: [-3., 0., -3.],
            max: [3., 4., 3.],
        },
        ticks: 2,
    });
    s.tick(&mut w);
    assert!(length(w.actors[1].velocity) > 0.);
    assert!(s.vulnerable.is_empty());
    s.tick(&mut w);
    assert_eq!(s.vulnerable[&Entity::Player(2)], 60);
    assert!(s.zones.is_empty());
    s.zones.push(Zone {
        owner: 1,
        kind: ZoneKind::Nova,
        bounds: Box3 {
            min: [-3., 0., -3.],
            max: [3., 4., 3.],
        },
        ticks: 2,
    });
    s.tick(&mut w);
    assert!(w.effects.is_empty());
    s.tick(&mut w);
    assert!(
        w.effects
            .iter()
            .any(|(_, name)| name == "minecraft:weakness")
    );
}
#[test]
fn divide_plane_crossing_is_infinite_and_does_not_block_same_side() {
    let d = divide::Divide::new([0., 0., 0.], [1., 0., 0.]).unwrap();
    assert!(d.crossing([1000., 50., -3.], [1000., 50., 3.]).is_some());
    assert!(d.crossing([0., 0., 1.], [0., 0., 3.]).is_none());
    assert!(divide::Divide::new([0.; 3], [0.; 3]).is_none());
}
#[test]
fn divide_blocks_knives_but_not_utility_and_expires() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    let mut d = divide::Divide::new([-1., 0., 2.], [1., 0., 2.]).unwrap();
    d.ticks = 3;
    s.divides.push(d);
    s.projectiles.push(projectiles::Projectile {
        owner: 1,
        kind: projectiles::ProjectileKind::Knife { alternate: false },
        position: [0., 2., 1.],
        velocity: [0., 0., 3.],
        ticks: 20,
        guided: false,
        victims: vec![],
    });
    s.tick(&mut w);
    assert!(s.projectiles.is_empty());
    s.tick(&mut w);
    s.tick(&mut w);
    assert!(s.divides.is_empty());
}
#[test]
fn audio_gain_is_reduced_only_across_divide() {
    let d = divide::Divide::new([-1., 0., 2.], [1., 0., 2.]).unwrap();
    *divide::audio_planes().lock().unwrap() = vec![d];
    assert_eq!(divide::gain([0., 0., 1.], [0., 0., 3.]), 0.1);
    assert_eq!(divide::gain([0., 0., 3.], [0., 0., 4.]), 1.);
    divide::audio_planes().lock().unwrap().clear();
}
#[test]
fn selection_and_snapshot_include_form() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.enqueue(
        1,
        Request {
            sequence: 1,
            expected_agent: 0,
            action: Action::Select(8),
        },
    );
    s.tick(&mut w);
    assert_eq!(s.snapshot(1).unwrap().agent, 8);
    s.enqueue(
        1,
        Request {
            sequence: 2,
            expected_agent: 8,
            action: Action::Activate(3),
        },
    );
    s.tick(&mut w);
    let v = s.snapshot(1).unwrap();
    assert!(v.camera.is_some());
    assert!(v.vision.phased);
    assert_eq!(v.star_count, 5);
}
#[test]
fn vulnerability_changes_final_damage_and_respects_runback() {
    let id = 2147000000;
    effects::vulnerable().lock().unwrap().insert(id, true);
    let mut value = 3.;
    effects::health_damage(id, &mut value);
    assert_eq!(value, 6.);
    effects::protections().lock().unwrap().insert(
        id,
        effects::Protection {
            health: 5.,
            returning: true,
            ..Default::default()
        },
    );
    effects::health_damage(id, &mut value);
    assert_eq!(value, 4.);
    effects::vulnerable().lock().unwrap().remove(&id);
    effects::protections().lock().unwrap().remove(&id);
}
#[test]
fn cosmic_divide_spends_ultimate_only_after_two_valid_points() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let mut kit = Astra {
        view: RemoteView::new(&w, [0., 3., 0.], 32.),
        ..Default::default()
    };
    kit.view.as_mut().unwrap().look(unit([0., -1., 1.]));
    let (mut s, mut z, mut d) = (vec![], vec![], vec![]);
    let mut c = Context {
        world: &mut w,
        smokes: &mut s,
        zones: &mut z,
        divides: &mut d,
    };
    assert!(kit.fire(true, &a, &mut c));
    assert!(kit.fire(false, &a, &mut c));
    assert_eq!(kit.slots[3].charges, 1);
    assert!(c.divides.is_empty());
    kit.view.as_mut().unwrap().position[0] = 2.;
    assert!(kit.fire(false, &a, &mut c));
    assert_eq!(kit.slots[3].charges, 0);
    assert_eq!(c.divides.len(), 1);
    assert!(kit.view.is_none());
}
#[test]
fn vulnerability_expires_even_after_target_disappears() {
    let mut w = TestWorld::default();
    let mut s = Session::default();
    s.vulnerable.insert(Entity::Mob(999), 2);
    s.tick(&mut w);
    assert_eq!(s.vulnerable[&Entity::Mob(999)], 1);
    s.tick(&mut w);
    assert!(s.vulnerable.is_empty());
}

#[test]
fn shots_starting_on_divide_are_blocked() {
    let d = divide::Divide::new([-1., 0., 0.], [1., 0., 0.]).unwrap();
    assert_eq!(d.crossing([0., 2., 0.], [0., 2., 3.]), Some(0.));
    assert_eq!(d.crossing([0., 2., 0.], [3., 2., 0.]), Some(0.));
}

#[test]
fn dimension_transfer_does_not_move_stars_or_refill_immediately() {
    let mut p = valorant_horizon_engine::server::Player {
        agent: 8,
        ..Default::default()
    };
    p.astra.available = 4;
    p.astra.stars.push(star(1, [1., 1., 1.]));
    p.change_dimension();
    assert!(p.astra.stars.is_empty());
    assert_eq!(p.astra.available, 4);
    assert_eq!(p.astra.returns, vec![80]);
}

#[test]
fn dimension_transfer_clears_other_world_anchors_without_refunding() {
    let mut p = valorant_horizon_engine::server::Player::default();
    p.phoenix.anchor = Some([1.; 3]);
    p.viper.cloud = Some([2.; 3]);
    p.viper.screen = Some(Box3 {
        min: [0.; 3],
        max: [2.; 3],
    });
    p.reyna
        .souls
        .push(valorant_horizon_engine::agents::reyna::Soul {
            position: [3.; 3],
            ticks: 60,
        });
    p.viper.slots[1].remaining = 123;
    p.viper.slots[1].charges = 0;
    p.change_dimension();
    assert!(p.phoenix.anchor.is_none());
    assert!(p.viper.cloud.is_none());
    assert!(p.viper.screen.is_none());
    assert!(p.reyna.souls.is_empty());
    assert_eq!(p.viper.slots[1].remaining, 123);
    assert_eq!(p.viper.slots[1].charges, 0);
}
