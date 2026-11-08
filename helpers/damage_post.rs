fn valorant_damage_post(causing: i32, entity: i32, amount: f32) {
    // Mob Damage::Who::Player has no numeric cause in v1; the synchronous
    // native attack scope supplies it without predicting or crediting refused hits.
    let cause = if causing != 0 {
        causing
    } else {
        horizon_client::health::knockback_attacker()
    };
    valorant_horizon_engine::gameplay::combat::record(entity, cause, amount);
    if let Ok(mut all) = valorant_horizon_engine::gameplay::effects::protections().lock()
        && let Some(p) = all.get_mut(&entity)
    {
        p.health = (p.health - amount).max(0.0);
    }
}
