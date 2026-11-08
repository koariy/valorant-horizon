//! Authoritative, bounded ability control. Suppression never disables vanilla tools.
use super::{Entity, World};
use std::collections::HashMap;
pub const MAX_CONTROLLED: usize = 128;
#[derive(Default, Clone, Copy)]
pub struct Control {
    pub suppressed: u32,
    pub detained: u32,
    pub(crate) applied: bool,
}
impl Control {
    pub fn blocked(self) -> bool {
        self.suppressed > 0 || self.detained > 0
    }
}
pub type Ledger = HashMap<Entity, Control>;
pub fn suppress(ledger: &mut Ledger, world: &impl World, target: Entity, ticks: u32) {
    if world.status_immune(target)
        || (ledger.len() >= MAX_CONTROLLED && !ledger.contains_key(&target))
    {
        return;
    }
    let state = ledger.entry(target).or_default();
    state.suppressed = state.suppressed.max(ticks.min(400));
}
pub fn tick(ledger: &mut Ledger) {
    for state in ledger.values_mut() {
        state.suppressed = state.suppressed.saturating_sub(1);
        state.detained = state.detained.saturating_sub(1);
    }
    ledger.retain(|_, state| state.blocked());
}

pub fn detain(ledger: &mut Ledger, world: &impl World, target: Entity, ticks: u32) {
    if world.status_immune(target)
        || (ledger.len() >= MAX_CONTROLLED && !ledger.contains_key(&target))
    {
        return;
    }
    let state = ledger.entry(target).or_default();
    state.detained = state.detained.max(ticks.clamp(1, 160));
}
/// Remove our previous disarm layer before kit protection is recomputed.
pub fn reset_interactions(ledger: &Ledger, w: &mut impl World) {
    for (entity, state) in ledger {
        if state.detained > 0
            && let Entity::Player(id) = entity
        {
            let mut p = w.protection_state(*id);
            p.disarmed = false;
            w.protect(*id, p);
        }
    }
}
pub fn apply(ledger: &Ledger, w: &mut impl World, actors: &[super::Actor]) {
    for (entity, state) in ledger {
        if state.detained == 0 || w.status_immune(*entity) {
            continue;
        }
        let Some(a) = actors
            .iter()
            .find(|a| a.entity == *entity && a.eligible && a.health > 0.)
        else {
            continue;
        };
        w.effect(*entity, "minecraft:slowness", 2, 5);
        if let Entity::Player(id) = entity {
            let mut p = w.protection_state(*id);
            p.disarmed = true;
            w.protect(*id, p);
        } else {
            w.effect(*entity, "minecraft:weakness", 2, 4);
        }
        w.push_entity(
            *entity,
            [a.velocity[0] * 0.25, a.velocity[1], a.velocity[2] * 0.25],
        );
    }
}
