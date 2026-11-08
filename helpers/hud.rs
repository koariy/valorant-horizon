fn hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut crate::hud::HudBatch,
    corners: &mut crate::extras_hud::Corners,
) {
    let _ = corners;
    game.vh_hud(batch);
}
