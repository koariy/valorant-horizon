fn valorant_locked_interaction(game: &crate::game::Game<'_>, canceled: &mut bool) {
    if game.vh_control_locked() {
        *canceled = true;
    }
}
