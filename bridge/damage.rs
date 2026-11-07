// This module is placed under commands/world_cmds.rs to use its real damage policy.
use super::*;
use valorant_horizon_engine::gameplay::{Entity, Hit};
impl Game<'_> {
    pub(in crate::game) fn vh_damage(
        &mut self,
        owner: u128,
        target: Entity,
        amount: f32,
        source: &str,
    ) -> Hit {
        let Some(attacker) = self.target_by_uuid(owner) else {
            return Hit::default();
        };
        let from = self.target_position(attacker);
        let before = match target {
            Entity::Mob(id) => self
                .mobs
                .iter()
                .find(|m| m.id == id)
                .map_or(0.0, |m| m.health),
            Entity::Player(id) => self
                .native_server_players()
                .iter()
                .find(|p| p.uuid == id)
                .map_or(0.0, |p| p.health),
        };
        let accepted = match target {
            Entity::Mob(id) => self
                .for_player(attacker, |g| {
                    g.command_damage_mob(
                        id,
                        amount,
                        Damage {
                            kind: match source {
                                "minecraft:in_fire" => DamageKind::InFire,
                                "minecraft:explosion" => DamageKind::Explosion,
                                _ => DamageKind::Arrow,
                            },
                            attacker: Some(Who::Player),
                            from,
                            attacker_species: None,
                        },
                    )
                })
                .unwrap_or(false),
            Entity::Player(id) => {
                let Some(victim) = self.target_by_uuid(id) else {
                    return Hit::default();
                };
                // Respect the host's PvP policy before applying player-sourced damage.
                let players = self.native_server_players();
                let team = |id| {
                    players
                        .iter()
                        .find(|p| p.uuid == id)
                        .and_then(|p| self.score.board.players_team(&p.name))
                };
                if id != owner
                    && (!self.ui.rules.bool("pvp")
                        || !horizon_client::scoreboard::teams_allow_damage(team(id), team(owner)))
                {
                    return Hit::default();
                }
                let name = self.target_name(attacker);
                self.for_player(victim, |g| {
                    g.note_direct_entity(source, name);
                    g.hurt_player_from(amount, from, source, Some(None))
                })
                .unwrap_or(false)
            }
        };
        let after = match target {
            Entity::Mob(id) => self
                .mobs
                .iter()
                .find(|m| m.id == id)
                .map_or(0.0, |m| m.health),
            Entity::Player(id) => self
                .native_server_players()
                .iter()
                .find(|p| p.uuid == id)
                .map_or(0.0, |p| p.health),
        };
        Hit {
            accepted,
            killed: accepted && before > 0.0 && after <= 0.0,
        }
    }
}
