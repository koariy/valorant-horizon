fn valorant_block_changed(
    level: &crate::game::GameLevel<'_>,
    dimension: horizon_client::mod_api::LevelDimension,
    pos: horizon_core::position::BlockPos,
    old: horizon_client::mod_api::BlockStateId,
    new: horizon_client::mod_api::BlockStateId,
) {
    if !level.native_authoritative || level.facts.get(old).name == level.facts.get(new).name {
        return;
    }
    let dimension = match dimension {
        horizon_client::mod_api::LevelDimension::Overworld => "minecraft:overworld",
        horizon_client::mod_api::LevelDimension::Nether => "minecraft:the_nether",
        horizon_client::mod_api::LevelDimension::End => "minecraft:the_end",
    };
    let epoch =
        valorant_horizon_engine::server::ACTIVE_EPOCH.load(std::sync::atomic::Ordering::Relaxed);
    if let Ok(mut all) = valorant_horizon_engine::server::sessions().lock()
        && let Some(s) = all.get_mut(&(epoch, dimension.into()))
    {
        let before = s.blocks.len();
        s.blocks.retain(|b| b.position != [pos.x, pos.y, pos.z]);
        s.blocks_dirty |= s.blocks.len() != before;
    }
}
