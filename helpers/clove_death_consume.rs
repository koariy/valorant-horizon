fn clove_death_consume(
    game: &crate::game::Game<'_>,
    key: winit::keyboard::KeyCode,
) -> std::ops::ControlFlow<Result<()>> {
    use valorant_horizon_engine::{DEFAULTS, ID, NAMES, state};
    if let Some(name) = horizon_client::keybinds::key_name(key)
        && state().lock().ok().is_some_and(|s| {
            s.snapshot
                .as_ref()
                .is_some_and(|x| x.agent == 10 && x.after_death)
        })
        && (name == game.native_mod_key(ID, NAMES[2], DEFAULTS[2])
            || [
                "key.keyboard.left",
                "key.keyboard.right",
                "key.keyboard.up",
                "key.keyboard.down",
                "key.keyboard.z",
                "key.keyboard.r",
                "key.keyboard.b",
            ]
            .contains(&name))
    {
        return std::ops::ControlFlow::Break(Ok(()));
    }
    std::ops::ControlFlow::Continue(())
}
