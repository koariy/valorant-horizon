fn valorant_phase_name(entity_id: u32, visible: &mut bool) {
    if valorant_horizon_engine::state()
        .lock()
        .ok()
        .is_some_and(|s| {
            s.snapshot
                .as_ref()
                .is_some_and(|s| s.hidden_entities.contains(&entity_id))
        })
    {
        *visible = false;
    }
}
