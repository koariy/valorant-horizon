fn valorant_mob_death(game: &crate::game::Game<'_>, mob: &horizon_client::mobs::Mob) {
    valorant_horizon_engine::gameplay::deaths::queue(game.vh_mob_credit(mob));
}
