fn clove_death_hud(game: &crate::game::Game<'_>, batch: &mut crate::hud::HudBatch) {
    if game.screen != crate::game::Screen::Dead {
        return;
    }
    let snapshot = valorant_horizon_engine::state()
        .lock()
        .ok()
        .and_then(|s| s.snapshot.clone());
    if let Some(s) = snapshot.filter(|s| s.agent == 10 && s.after_death) {
        game.native_text(
            batch,
            "Clove Ruse: E equip | arrows aim | Z mark | R confirm | B cancel",
            [4., batch.height - 30.],
            0xE0B0FF,
            true,
        );
        let text = format!(
            "Ruse charges: {} | marks: {} | marker: {:?} | X resurrection unavailable",
            s.charges[2],
            s.clove_marks,
            s.preview.map(|p| p.center.map(|n| n.round() as i32))
        );
        game.native_text(batch, &text, [4., batch.height - 18.], 0xFFFFFF, true);
    }
}
