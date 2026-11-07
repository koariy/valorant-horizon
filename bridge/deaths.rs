// Child of lan_actions: access to the real guest death/combat path, not a fake event.
use super::*;
use valorant_horizon_engine::gameplay::{Entity, deaths::Death};
impl Game<'_> {
    pub(in crate::game) fn vh_guest_credit(
        &self,
        host: &LanHost,
        gi: usize,
        conn: u32,
    ) -> Option<Death> {
        let target = host.hub.guests.get(gi)?;
        if target.health > 0. {
            return None;
        }
        let owner = self
            .guest_combat
            .get(&conn)
            .and_then(|c| c.last_blow())
            .and_then(|(_, attacker)| {
                host.hub
                    .guests
                    .iter()
                    .find(|g| g.entity_id as u32 == attacker)
            })
            .map(|g| g.uuid)
            .filter(|id| *id != target.uuid)
            .unwrap_or(0);
        let level = self.native_level(&target.dimension)?;
        Some(Death {
            key: (level.epoch().get(), target.dimension.clone()),
            owner,
            target: Entity::Player(target.uuid),
            position: target.position,
        })
    }
}
