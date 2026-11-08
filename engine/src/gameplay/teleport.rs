use super::{Actor, World, geometry::*};
pub fn safe(world: &mut impl World, actor: &Actor, position: V3) -> bool {
    let super::Entity::Player(id) = actor.entity else {
        return false;
    };
    let body = actor.bounds.translated(sub(position, actor.position));
    if clear_box(world, body)
        && !world
            .actors()
            .iter()
            .any(|a| a.entity != actor.entity && a.eligible && a.bounds.intersects(body))
    {
        world.teleport_player(id, position)
    } else {
        false
    }
}

/// Aim-selected, loaded destination with solid ground and enough space for the real body.
pub fn target(world: &impl World, a: &Actor, range: f64) -> Option<V3> {
    let from = a.eye();
    let end = add(from, scale(a.direction, range));
    let point = if let Some(hit) = block_hit(world, from, end, 0.0) {
        add(
            add(from, scale(sub(end, from), hit)),
            scale(a.direction, -0.4),
        )
    } else {
        end
    };
    let top = add(point, [0.0, 0.15, 0.0]);
    let bottom = add(top, [0.0, -6.0, 0.0]);
    let hit = block_hit(world, top, bottom, 0.0)?;
    let feet = add(add(top, scale(sub(bottom, top), hit)), [0.0, 0.02, 0.0]);
    (distance(a.position, feet) >= 0.5
        && distance(a.position, feet) <= range + 0.05
        && clear_box(world, a.bounds.translated(sub(feet, a.position))))
    .then_some(feet)
}

// Native bridge event cursor. Keys include the level epoch and dimension so
// separate worlds with identical player UUIDs cannot release each other's tethers.
use std::{
    collections::HashMap,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};
type Key = (u64, String, u128);
fn events() -> &'static Mutex<HashMap<Key, u64>> {
    static EVENTS: OnceLock<Mutex<HashMap<Key, u64>>> = OnceLock::new();
    EVENTS.get_or_init(Default::default)
}
pub fn record(key: Key) {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let mut all = events().lock().unwrap_or_else(|p| p.into_inner());
    if all.len() >= 1024 && !all.contains_key(&key) {
        all.clear();
    }
    all.insert(key, NEXT.fetch_add(1, Ordering::Relaxed));
}
pub fn stamp(key: &Key) -> u64 {
    events()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(key)
        .copied()
        .unwrap_or(0)
}
pub fn clear_events() {
    events().lock().unwrap_or_else(|p| p.into_inner()).clear();
}
