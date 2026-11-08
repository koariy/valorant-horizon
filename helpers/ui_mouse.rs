fn ui_mouse(
    game: &crate::game::Game<'_>,
    screen: &crate::mod_api::NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) {
    if game.native_mod_screen() == Some(valorant_horizon_engine::ID) {
        *canceled = true;
        if button == 0 {
            game.vh_ui_click(screen.mouse, [screen.rect[2], screen.rect[3]]);
        }
    }
}
