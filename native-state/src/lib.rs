// The SDK library cannot depend on the gameplay engine's bundle crate.
// A safe read-only callback bridges the layers without duplicating game state.
use std::sync::OnceLock;
static DOWNED: OnceLock<fn(i32) -> bool> = OnceLock::new();
pub fn register_downed(predicate: fn(i32) -> bool) {
    let _ = DOWNED.set(predicate);
}
pub fn is_downed(entity: i32) -> bool {
    DOWNED.get().is_some_and(|predicate| predicate(entity))
}
