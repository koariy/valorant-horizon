fn valorant_stop(game: &mut crate::game::Game<'_>) {
    game.vh_flush_barriers();
    valorant_horizon_engine::gameplay::teleport::clear_events();
    valorant_horizon_engine::gameplay::combat::clear();
    if let Ok(mut v) = valorant_horizon_engine::gameplay::effects::vulnerability_ids().lock() {
        v.clear();
    }
    if let Ok(mut v) = valorant_horizon_engine::gameplay::effects::vulnerable().lock() {
        v.clear();
    }
    if let Ok(mut e) = valorant_horizon_engine::gameplay::deaths::events().lock() {
        e.clear();
    }
    if let Ok(mut p) = valorant_horizon_engine::gameplay::effects::protections().lock() {
        p.clear();
    }
    if let Ok(mut s) = valorant_horizon_engine::server::sessions().lock() {
        s.clear();
    }
}
