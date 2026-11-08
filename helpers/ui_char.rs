fn ui_char(game: &crate::game::Game<'_>, c: char, canceled: &mut bool) {
    if game.native_mod_screen() == Some(valorant_horizon_engine::ID) {
        *canceled = true;
        if c != '/' {
            if let Ok(mut s) = valorant_horizon_engine::state().lock() {
                s.ui.typed(c);
            }
        }
    }
}
