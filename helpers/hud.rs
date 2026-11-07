fn hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut crate::hud::HudBatch,
    corners: &mut crate::extras_hud::Corners,
) {
    use valorant_horizon_engine::{DEFAULTS, ID, NAMES, state};
    let Ok(s) = state().lock() else {
        return;
    };
    let bindings = std::array::from_fn(|i| game.native_mod_key(ID, NAMES[i], DEFAULTS[i]));
    let lines = s.lines(&bindings);
    let Some(corner) = crate::extras::Corner::ALL
        .into_iter()
        .find(|c| !game.native_corner_shown(*c))
    else {
        return;
    };
    let width = lines
        .iter()
        .map(|l| game.native_text_width(l))
        .fold(0.0_f32, f32::max)
        + 8.0;
    let height = 8.0 + lines.len() as f32 * 12.0;
    let (x, y) = corners.place(corner, width, height);
    let active = s.runtime.iter().any(|r| r.active > 0);
    game.native_fill(
        batch,
        [x, y, width, height],
        if active { 0xC0206050 } else { 0xB0101820 },
    );
    for (i, line) in lines.iter().enumerate() {
        game.native_text(
            batch,
            line,
            [x + 4.0, y + 4.0 + i as f32 * 12.0],
            0xFFFFFF,
            true,
        );
    }
}
