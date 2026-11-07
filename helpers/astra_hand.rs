fn valorant_hand(canceled: &mut bool) {
    if valorant_horizon_engine::state()
        .lock()
        .ok()
        .is_some_and(|s| s.snapshot.as_ref().is_some_and(|s| s.camera.is_some()))
    {
        *canceled = true;
    }
}
