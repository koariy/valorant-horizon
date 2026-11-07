mod support;
use support::*;
use valorant_horizon_engine::{
    agents::sage::Sage,
    gameplay::{barrier::*, geometry::*, *},
    server::Session,
};
#[test]
fn barrier_places_real_blocks_and_rotation_changes_axis() {
    let mut w = TestWorld::default();
    w.actors.truncate(1);
    let a = w.actors[0].clone();
    let mut sage = Sage::default();
    let mut ledger = vec![];
    assert!(sage.activate(0, &a));
    assert!(sage.fire(false, &a, &mut w, &mut vec![], &mut ledger));
    assert_eq!(ledger.len(), 12);
    assert_eq!(
        w.blocks.values().filter(|b| b.as_str() == BLOCK).count(),
        12
    );
    assert_ne!(cells(&a, false), cells(&a, true));
    assert_eq!(sage.slots[0].charges, 0);
}
#[test]
fn wall_rejects_occupied_space_without_overwrite_or_charge() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let cell = cells(&a, false)[0];
    w.blocks.insert(cell, "minecraft:chest".into());
    let mut sage = Sage::default();
    sage.activate(0, &a);
    assert!(!sage.fire(false, &a, &mut w, &mut vec![], &mut vec![]));
    assert_eq!(w.blocks[&cell], "minecraft:chest");
    assert_eq!(sage.slots[0].charges, 1);
}
#[test]
fn ledger_survives_reload_and_cleanup_preserves_replacements() {
    let mut w = TestWorld::default();
    w.actors.truncate(1);
    let a = w.actors[0].clone();
    let mut owned = vec![];
    assert!(place(&mut w, &a, false, &mut owned));
    let replaced = owned[0].position;
    w.set_block(replaced, "minecraft:diamond_block");
    w.time = 601;
    let mut s = Session::default();
    s.tick(&mut w);
    assert_eq!(w.blocks[&replaced], "minecraft:diamond_block");
    assert!(s.blocks.is_empty());
    assert!(w.ledger.is_empty());
    assert_eq!(w.blocks.values().filter(|s| s.as_str() == BLOCK).count(), 0);
}
#[test]
fn healing_requires_injured_visible_target_and_channels_over_time() {
    let mut w = TestWorld::default();
    w.actors[1] = actor(2, [0.0, 1.0, 5.0]);
    w.actors[1].health = 5.0;
    let a = w.actors[0].clone();
    let mut sage = Sage::default();
    sage.activate(2, &a);
    assert!(sage.fire(false, &a, &mut w, &mut vec![], &mut vec![]));
    for _ in 0..100 {
        sage.tick(&a, &mut w);
    }
    assert_eq!(w.actors[1].health, 20.0);
    assert_eq!(sage.slots[2].charges, 0);
}
#[test]
fn resurrection_is_explicitly_unavailable_and_slow_has_real_status() {
    let mut w = TestWorld::default();
    let a = w.actors[0].clone();
    let mut sage = Sage::default();
    assert!(!sage.activate(3, &a));
    assert_eq!(sage.slots[3].charges, 0);
    let mut s = Session::default();
    s.zones.push(zones::Zone {
        owner: 1,
        kind: zones::ZoneKind::Slow,
        bounds: Box3 {
            min: [-3.0, 1.0, -3.0],
            max: [3.0, 2.0, 3.0],
        },
        ticks: 100,
    });
    s.tick(&mut w);
    assert!(
        w.effects
            .iter()
            .any(|(_, kind)| kind == "minecraft:slowness")
    );
    assert!(w.damage.is_empty());
}
