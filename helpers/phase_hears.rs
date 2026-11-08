fn valorant_phase_hears(hears: &mut bool) {
    if valorant_horizon_engine::gameplay::effects::protections()
        .lock()
        .ok()
        .is_some_and(|p| p.values().any(|p| p.invulnerable))
    {
        *hears = true;
    }
}
