// Horizon's world canvas early-outs when no trait mods/render hooks are installed.
// A source-only mixin bundle has neither. Handle precisely that path ourselves.
fn valorant_world_enable(
    game: &mut crate::game::Game<'_>,
    camera: glam::DVec3,
    partial: f32,
    out: &mut Vec<horizon_client::mesher::Vertex>,
) -> std::ops::ControlFlow<()> {
    if !horizon_client::gfx::native_render::world_drawn_hooked()
        && crate::mod_api::mods().is_empty()
    {
        let mut draw = NativeWorldDraw::new(camera.to_array(), partial, game.world.game_time);
        game.mark_stage(out.len(), "NONE");
        game.native_world_submit(&mut draw);
        game.resolve_inline_profiles(&draw.profiles);
        game.map_patches.extend(draw.patches);
        out.extend(draw.draw.into_vertices());
        return std::ops::ControlFlow::Break(());
    }
    std::ops::ControlFlow::Continue(())
}
