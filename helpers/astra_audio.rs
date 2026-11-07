fn valorant_audio(value: f32, instance: &crate::sound_engine::SoundInstance) -> f32 {
    if instance.relative {
        value
    } else {
        value * valorant_horizon_spatial::sound_gain(instance.position)
    }
}
