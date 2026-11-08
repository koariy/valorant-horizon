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
