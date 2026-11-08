fn valorant_damage_pre(entity: i32, damage_type: &str, amount: &mut f32) {
    valorant_horizon_engine::gameplay::effects::health_damage_kind(entity, damage_type, amount);
}
