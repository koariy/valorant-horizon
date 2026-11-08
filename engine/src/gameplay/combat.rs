//! Positive, post-armour hits only. Raw entity IDs are routed to their current world.
use super::Entity;
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};
type Key = (u64, String);
struct Credit {
    key: Key,
    owner: u128,
    target: Entity,
}
#[derive(Default)]
struct State {
    routes: HashMap<i32, Vec<(Key, Entity)>>,
    hits: Vec<Credit>,
}
fn state() -> &'static Mutex<State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(Default::default)
}
pub fn register(key: Key, entities: Vec<(i32, Entity)>) {
    if let Ok(mut s) = state().lock() {
        for r in s.routes.values_mut() {
            r.retain(|(k, _)| *k != key);
        }
        s.routes.retain(|_, v| !v.is_empty());
        for (id, e) in entities {
            s.routes.entry(id).or_default().push((key.clone(), e));
        }
    }
}
pub fn record(target: i32, owner: i32, amount: f32) {
    if !amount.is_finite() || amount <= 0. {
        return;
    }
    if let Ok(mut s) = state().lock() {
        let Some(a) = s
            .routes
            .get(&owner)
            .filter(|v| v.len() == 1)
            .and_then(|v| v.first())
            .cloned()
        else {
            return;
        };
        let Some(b) = s
            .routes
            .get(&target)
            .filter(|v| v.len() == 1)
            .and_then(|v| v.first())
            .cloned()
        else {
            return;
        };
        let Entity::Player(owner) = a.1 else {
            return;
        };
        if a.0 != b.0 || a.1 == b.1 || s.hits.len() >= 256 {
            return;
        }
        s.hits.push(Credit {
            key: a.0,
            owner,
            target: b.1,
        });
    }
}
pub fn take(key: &Key) -> Vec<(u128, Entity)> {
    let Ok(mut s) = state().lock() else {
        return vec![];
    };
    let mut out = vec![];
    s.hits.retain(|h| {
        if h.key == *key {
            out.push((h.owner, h.target));
            false
        } else {
            true
        }
    });
    out
}
pub fn clear_epoch(epoch: u64) {
    if let Ok(mut s) = state().lock() {
        s.hits.retain(|h| h.key.0 != epoch);
        for r in s.routes.values_mut() {
            r.retain(|(k, _)| k.0 != epoch);
        }
        s.routes.retain(|_, v| !v.is_empty());
    }
}

pub fn clear() {
    if let Ok(mut s) = state().lock() {
        *s = State::default();
    }
}
pub fn raw_ids(player: u128) -> Vec<i32> {
    state().lock().ok().map_or_else(Vec::new, |s| {
        s.routes
            .iter()
            .filter(|(_, routes)| routes.iter().any(|(_, e)| *e == Entity::Player(player)))
            .map(|(id, _)| *id)
            .collect()
    })
}
pub fn detach(player: u128) -> Vec<i32> {
    let ids = raw_ids(player);
    if let Ok(mut s) = state().lock() {
        for routes in s.routes.values_mut() {
            routes.retain(|(_, e)| *e != Entity::Player(player));
        }
        s.routes.retain(|_, routes| !routes.is_empty());
        s.hits.retain(|hit| hit.owner != player);
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn positive_hits_route_only_with_unique_world_ids() {
        clear();
        let k = (91, "test".into());
        register(
            k.clone(),
            vec![(100, Entity::Player(1)), (101, Entity::Mob(2))],
        );
        record(101, 100, 0.);
        record(101, 100, f32::NAN);
        record(101, 100, 1.);
        assert_eq!(take(&k), vec![(1, Entity::Mob(2))]);
        register((92, "other".into()), vec![(101, Entity::Mob(3))]);
        record(101, 100, 1.);
        assert!(take(&k).is_empty());
        clear_epoch(92);
        record(101, 100, 1.);
        assert_eq!(take(&k).len(), 1);
        clear();
    }
}
