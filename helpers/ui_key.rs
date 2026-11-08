fn ui_key(game: &crate::game::Game<'_>, key: &str, canceled: &mut bool) {
    if game.native_mod_screen() == Some(valorant_horizon_engine::ID) {
        *canceled = true;
        game.vh_ui_key(key);
    }
}
