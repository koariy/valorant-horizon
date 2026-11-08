fn valorant_protection(
    entity: i32,
    damage_type: &str,
    causing: i32,
    direct: i32,
    amount: f32,
    canceled: &mut bool,
) {
    if *canceled {
        return;
    }
    if let Ok(mut all) = valorant_horizon_engine::gameplay::effects::protections().lock() {
        if causing != 0
            && causing == direct
            && all.get(&causing).is_some_and(|p| p.disarmed || p.downed)
        {
            *canceled = true;
            return;
        }
        if let Some(p) = all.get_mut(&entity) {
            if p.invulnerable && damage_type != "minecraft:fell_out_of_world" {
                *canceled = true;
                return;
            }
            if p.one_hit && amount > 0. {
                p.one_hit = false;
                *canceled = true;
            }
        }
    }
}
