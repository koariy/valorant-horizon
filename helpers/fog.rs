fn valorant_fog(
    game: &crate::game::Game<'_>,
    camera: [f64; 3],
    fog: &mut horizon_client::fog::FogData,
) {
    game.vh_fog(camera, fog);
}
