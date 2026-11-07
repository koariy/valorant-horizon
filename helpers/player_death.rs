fn valorant_player_death(
    game: &crate::game::Game<'_>,
    host: &crate::game::lan::LanHost,
    index: usize,
    connection: u32,
) {
    valorant_horizon_engine::gameplay::deaths::queue(game.vh_guest_credit(host, index, connection));
}
