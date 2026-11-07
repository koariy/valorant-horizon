fn valorant_net_server(game: &mut crate::game::Game<'_>, channel: &str, data: &[u8]) {
    game.vh_snapshot(channel, data);
}
