fn valorant_phase_impact(entity: Option<u32>, canceled: &mut bool) {
    if entity.is_some_and(|id| {
        valorant_horizon_engine::gameplay::effects::protections()
            .lock()
            .ok()
            .is_some_and(|p| p.get(&(id as i32)).is_some_and(|p| p.invulnerable))
    }) {
        *canceled = true;
    }
}
