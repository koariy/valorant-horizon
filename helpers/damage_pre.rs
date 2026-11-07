fn valorant_damage_pre(entity: i32, amount: &mut f32) {
    valorant_horizon_engine::gameplay::effects::health_damage(entity, amount);
}
