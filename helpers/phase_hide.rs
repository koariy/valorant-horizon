fn valorant_phase_hide(value: bool, id: u32) -> bool {
    value
        || valorant_horizon_engine::state()
            .lock()
            .ok()
            .is_some_and(|s| {
                s.snapshot
                    .as_ref()
                    .is_some_and(|s| s.hidden_entities.contains(&id))
            })
}
