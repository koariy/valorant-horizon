fn valorant_downed_healing(value: (), entity: u32, canceled: &mut bool) {
    if crate::valorant_native_state::is_downed(entity as i32) {
        *canceled = true;
    }
    value
}
