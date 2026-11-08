fn recon_outline(entity_id: u32, outline: &mut Option<u32>) {
    if valorant_horizon_engine::state()
        .lock()
        .ok()
        .is_some_and(|s| {
            s.snapshot
                .as_ref()
                .is_some_and(|s| s.hidden_entities.contains(&entity_id))
        })
    {
        *outline = None;
        return;
    }
    if valorant_horizon_engine::state()
        .lock()
        .ok()
        .is_some_and(|s| {
            s.snapshot.as_ref().is_some_and(|s| {
                s.contacts
                    .iter()
                    .any(|c| c.entity == entity_id && c.ticks > 0)
            })
        })
    {
        let color = valorant_horizon_engine::state()
            .lock()
            .ok()
            .map(
                |s| match valorant_horizon_engine::presentation::ui::role(s.selected) {
                    valorant_horizon_engine::presentation::ui::Role::Duelist => 0xff667b,
                    valorant_horizon_engine::presentation::ui::Role::Controller => 0xb28cff,
                    valorant_horizon_engine::presentation::ui::Role::Sentinel => 0x70dfbd,
                    _ => 0x50c8ff,
                },
            )
            .unwrap_or(0x50c8ff);
        *outline = Some(color);
    }
}
