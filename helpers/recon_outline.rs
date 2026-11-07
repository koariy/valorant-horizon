fn recon_outline(entity_id: u32, outline: &mut Option<u32>) {
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
        *outline = Some(0x50C8FF);
    }
}
