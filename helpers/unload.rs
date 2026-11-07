fn valorant_unload(game: &mut crate::game::Game<'_>) {
    let _ = game;
    if let Ok(mut d) = horizon_client_render::valorant_spatial::audio_planes().lock() {
        d.clear();
    }
    if let Ok(mut s) = valorant_horizon_engine::state().lock() {
        s.snapshot = None;
        s.held.clear();
        s.pending_equipped = None;
        s.heartbeat = 0;
        s.menu = false;
        s.consumed = None;
    }
}
