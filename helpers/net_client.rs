fn valorant_net_client(game: &mut crate::game::Game<'_>, player: u128, channel: &str, data: &[u8]) {
    game.vh_request(player, channel, data);
}
