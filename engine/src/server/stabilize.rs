//! Server-picked teammate channel. The wire carries only hold/release intent.
use super::*;
pub struct Channel {
    pub target: u128,
    lease: u32,
    ticks: u32,
    origin: V3,
    hurt: u64,
    pool: f32,
}
impl Session {
    fn stabilization_target(
        &mut self,
        w: &impl World,
        actor: &Actor,
        actors: &[Actor],
    ) -> Option<u128> {
        let Entity::Player(id) = actor.entity else {
            return None;
        };
        let state = w.protection_state(id);
        if !actor.eligible
            || actor.health <= 0.
            || state.invulnerable
            || state.downed
            || state.disarmed && !self.stabilizers.contains_key(&id)
        {
            return None;
        }
        let end = add(actor.eye(), scale(unit(actor.direction), 3.));
        let nearest = actors
            .iter()
            .filter(|a| a.entity != actor.entity && a.eligible && a.health > 0.)
            .filter_map(|a| {
                a.bounds
                    .expanded(0.05)
                    .segment(actor.eye(), end)
                    .map(|t| (t, a))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))?
            .1;
        let Entity::Player(target) = nearest.entity else {
            return None;
        };
        if distance(actor.position, nearest.position) > 3.
            || !w.intel_viewers(id).contains(&target)
            || !w.protection_state(target).downed
            || !self
                .players
                .get(&target)
                .is_some_and(|p| p.agent == 18 && p.extra.kayo.downed())
            || self.budget.rays == 0
        {
            return None;
        }
        self.budget.rays -= 1;
        if block_hit(w, actor.eye(), nearest.eye(), 0.).is_some() {
            return None;
        }
        Some(target)
    }
    pub(super) fn hold_stabilization(
        &mut self,
        w: &impl World,
        a: &Actor,
        actors: &[Actor],
        held: bool,
    ) -> bool {
        let Entity::Player(id) = a.entity else {
            return false;
        };
        if !held {
            self.stabilizers.remove(&id);
            return true;
        }
        let Some(target) = self.stabilization_target(w, a, actors) else {
            self.stabilizers.remove(&id);
            return false;
        };
        if self
            .stabilizers
            .iter()
            .any(|(other, ch)| *other != id && ch.target == target)
        {
            return false;
        }
        if let Some(channel) = self
            .stabilizers
            .get_mut(&id)
            .filter(|ch| ch.target == target)
        {
            channel.lease = 12;
            return true;
        }
        let state = w.protection_state(id);
        self.stabilizers.insert(
            id,
            Channel {
                target,
                lease: 12,
                ticks: 0,
                origin: a.position,
                hurt: state.hurt_epoch,
                pool: w.protection_state(target).downed_pool,
            },
        );
        true
    }
    pub(super) fn tick_stabilization(&mut self, w: &mut impl World, actors: &[Actor]) {
        let ids: Vec<_> = self.stabilizers.keys().copied().collect();
        for id in ids {
            let Some(a) = actors.iter().find(|a| a.entity == Entity::Player(id)) else {
                self.stabilizers.remove(&id);
                continue;
            };
            let target = self.stabilization_target(w, a, actors);
            let Some(mut channel) = self.stabilizers.remove(&id) else {
                continue;
            };
            channel.lease = channel.lease.saturating_sub(1);
            let target_state = w.protection_state(channel.target);
            if channel.lease == 0
                || target != Some(channel.target)
                || distance(a.position, channel.origin) > 0.3
                || w.protection_state(id).hurt_epoch != channel.hurt
                || target_state.downed_pool < channel.pool
            {
                continue;
            }
            channel.pool = target_state.downed_pool;
            channel.ticks += 1;
            let required = self
                .players
                .get(&channel.target)
                .map_or(60, |p| p.extra.kayo.settings.stabilize_ticks.clamp(20, 120));
            if channel.ticks >= required {
                if self
                    .players
                    .get_mut(&channel.target)
                    .is_some_and(|p| p.extra.kayo.stabilize())
                {
                    w.protect(channel.target, effects::Protection::default());
                    w.heal(Entity::Player(channel.target), 9.);
                    if let Some(at) = actors
                        .iter()
                        .find(|a| a.entity == Entity::Player(channel.target))
                        .map(|a| a.position)
                    {
                        w.ability_sound(at, "minecraft:block.beacon.activate");
                    }
                }
            } else {
                let mut protection = w.protection_state(id);
                protection.disarmed = true;
                w.protect(id, protection);
                self.stabilizers.insert(id, channel);
            }
        }
        self.stabilize_targets.clear();
        for actor in actors.iter().take(64) {
            if let Entity::Player(id) = actor.entity
                && let Some(target) = self.stabilization_target(w, actor, actors)
            {
                self.stabilize_targets.insert(id, target);
            }
        }
    }
    pub(super) fn stabilization_progress(&self, id: u128) -> u32 {
        self.stabilizers.get(&id).map_or(0, |ch| ch.ticks)
    }
}
