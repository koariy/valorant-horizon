fn valorant_consume(
    game: &crate::game::Game<'_>,
    raw: winit::keyboard::KeyCode,
) -> std::ops::ControlFlow<Result<()>> {
    use valorant_horizon_engine::{DEFAULTS, ID, NAMES, state};
    if game.native_window()
        && game.native_playing()
        && let Some(name) = horizon_client::keybinds::key_name(raw)
    {
        let binding = NAMES
            .iter()
            .zip(DEFAULTS)
            .any(|(slot, default)| game.native_mod_key(ID, slot, default) == name);
        let ui = state()
            .lock()
            .ok()
            .is_some_and(|s| s.menu || s.rebind.is_some() || s.consumed.as_deref() == Some(name));
        let recall = name == "key.keyboard.r"
            && state()
                .lock()
                .ok()
                .is_some_and(|s| s.selected == 8 || s.selected >= 12);
        if binding || ui || recall || name == "key.keyboard.v" || name == "key.keyboard.b" {
            return std::ops::ControlFlow::Break(Ok(()));
        }
    }
    std::ops::ControlFlow::Continue(())
}
