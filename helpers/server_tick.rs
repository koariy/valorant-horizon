fn valorant_server_tick(game: &mut crate::game::Game<'_>, dimension: &str) {
    horizon_client::valorant_native_state::register_downed(
        valorant_horizon_engine::gameplay::effects::is_downed,
    );
    game.vh_server_tick(dimension);
}
