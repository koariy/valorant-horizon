fn valorant_damage_post(causing: i32, entity: i32, amount: f32) {
    valorant_horizon_engine::gameplay::combat::record(entity, causing, amount);
    if let Ok(mut all) = valorant_horizon_engine::gameplay::effects::protections().lock()
        && let Some(p) = all.get_mut(&entity)
    {
        p.health = (p.health - amount).max(0.0);
    }
}
