fn ui_render(game: &crate::game::Game<'_>, batch: &mut crate::hud::HudBatch) {
    if game.native_mod_screen() == Some(valorant_horizon_engine::ID) {
        game.vh_ui_draw(batch);
    }
}
