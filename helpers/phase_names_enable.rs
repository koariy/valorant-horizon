// Source-only bundles do not register trait mods; names.rs otherwise skips name hooks.
fn valorant_phase_names_enable(value: bool) -> bool {
    value
        || valorant_horizon_engine::state()
            .lock()
            .ok()
            .is_some_and(|s| {
                s.snapshot
                    .as_ref()
                    .is_some_and(|s| !s.hidden_entities.is_empty())
            })
}
