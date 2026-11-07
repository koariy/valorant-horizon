//! Deferred events avoid reentrant session locks during synchronous Horizon damage.
use super::{Entity, geometry::V3};
use std::sync::{Mutex, OnceLock};
#[derive(Clone)]
pub struct Death {
    pub key: (u64, String),
    pub owner: u128,
    pub target: Entity,
    pub position: V3,
}
pub fn events() -> &'static Mutex<Vec<Death>> {
    static E: OnceLock<Mutex<Vec<Death>>> = OnceLock::new();
    E.get_or_init(Default::default)
}
pub fn queue(event: Option<Death>) {
    if let Some(e) = event
        && let Ok(mut q) = events().lock()
        && q.len() < 256
    {
        q.push(e);
    }
}
