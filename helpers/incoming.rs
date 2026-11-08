fn valorant_protection(
    entity: i32,
    damage_type: &str,
    causing: i32,
    direct: i32,
    canceled: &mut bool,
) {
    if let Ok(all) = valorant_horizon_engine::gameplay::effects::protections().lock() {
        if all.get(&entity).is_some_and(|p| p.invulnerable)
            && damage_type != "minecraft:fell_out_of_world"
        {
            *canceled = true;
        }
        if causing != 0 && causing == direct && all.get(&causing).is_some_and(|p| p.disarmed) {
            *canceled = true;
        }
    }
}
