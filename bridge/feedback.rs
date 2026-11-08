// Bounded authoritative hit/heal/teleport cosmetics; no gameplay state changes.
use super::Game;
use std::sync::{Mutex, OnceLock};
use valorant_horizon_engine::{
    gameplay::{Entity, geometry::V3},
    presentation::EmissionBudget,
};
impl Game<'_> {
    pub(super) fn vh_feedback(&mut self, target: Entity, kind: u8) {
        let position = match target {
            Entity::Player(id) => self
                .native_server_players()
                .into_iter()
                .find(|p| p.uuid == id)
                .map(|p| p.position),
            Entity::Mob(id) => self.native_mob(self.dim.id(), id).map(|p| p.pos),
        };
        if let Some(mut at) = position {
            at[1] += 0.8;
            self.vh_feedback_at(at, kind);
        }
    }
    pub(super) fn vh_feedback_at(&mut self, at: V3, kind: u8) {
        static BUDGET: OnceLock<Mutex<EmissionBudget>> = OnceLock::new();
        if !at.iter().all(|v| v.is_finite())
            || !BUDGET
                .get_or_init(Default::default)
                .lock()
                .ok()
                .is_some_and(|mut b| {
                    b.allow({
                        static START: OnceLock<std::time::Instant> = OnceLock::new();
                        (START
                            .get_or_init(std::time::Instant::now)
                            .elapsed()
                            .as_millis()
                            / 50) as i64
                    })
                })
        {
            return;
        }
        use horizon_client::particles::{ParticleOptions, ParticleType};
        let ty = match kind {
            0 => ParticleType::DamageIndicator,
            1 => ParticleType::Heart,
            _ => ParticleType::ReversePortal,
        };
        let options = ParticleOptions::simple(ty);
        self.native_level_send_particles(&options, None, at, [0.2, 0.3, 0.2], 0., 3);
    }
}
