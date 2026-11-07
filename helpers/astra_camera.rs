fn valorant_camera(position: &mut [f64; 3]) {
    if let Some(at) = valorant_horizon_engine::state()
        .lock()
        .ok()
        .and_then(|s| s.snapshot.as_ref().and_then(|s| s.camera))
    {
        *position = at;
    }
}
