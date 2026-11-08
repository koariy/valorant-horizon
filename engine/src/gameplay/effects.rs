use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Vision {
    pub flash: u32,
    pub nearsight: u32,
    pub deafen: u32,
    pub phased: bool,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Protection {
    pub health: f32,
    pub one_hit: bool,
    pub shield_epoch: u64,
    pub invulnerable: bool,
    pub disarmed: bool,
    pub returning: bool,
    pub requested: bool,
}
pub fn protections() -> &'static Mutex<HashMap<i32, Protection>> {
    static P: OnceLock<Mutex<HashMap<i32, Protection>>> = OnceLock::new();
    P.get_or_init(Default::default)
}
/// Final, post-armour damage: keep Run It Back alive until its next server tick.
pub fn vulnerable() -> &'static Mutex<HashMap<i32, bool>> {
    static V: OnceLock<Mutex<HashMap<i32, bool>>> = OnceLock::new();
    V.get_or_init(Default::default)
}
pub fn health_damage(entity: i32, amount: &mut f32) {
    if vulnerable()
        .lock()
        .ok()
        .is_some_and(|v| v.get(&entity).copied().unwrap_or(false))
    {
        *amount *= 2.;
    }

    if let Ok(mut all) = protections().lock()
        && let Some(p) = all.get_mut(&entity)
        && p.returning
        && *amount >= p.health
    {
        *amount = (p.health - 1.0).max(0.0);
        p.requested = true;
    }
}

pub fn vulnerability_ids() -> &'static Mutex<HashMap<super::Entity, i32>> {
    static I: OnceLock<Mutex<HashMap<super::Entity, i32>>> = OnceLock::new();
    I.get_or_init(Default::default)
}

/// Negative statuses used by shared abilities; phase is not a generic buff purge.
pub fn hostile_status(name: &str) -> bool {
    matches!(
        name,
        "minecraft:blindness"
            | "minecraft:slowness"
            | "minecraft:weakness"
            | "minecraft:poison"
            | "minecraft:wither"
    )
}

/// A repeated tick must not restore a consumed shield of the same grant.
/// A fresh orb increments shield_epoch; returning's lethal latch is separate.
pub fn merge_protection(old: Option<Protection>, mut next: Protection) -> Protection {
    if next.one_hit && old.is_some_and(|old| old.shield_epoch == next.shield_epoch && !old.one_hit)
    {
        next.one_hit = false;
    }
    next.requested = next.returning && old.is_some_and(|old| old.requested);
    next
}
