//! Budgeted hitscan shared by temporary abilities, without native weapon dependency.
use super::*;
impl<W: World> Context<'_, W> {
    pub fn terrain_limit(&mut self, from: V3, to: V3) -> Option<f64> {
        let parts = (distance(from, to) / 2.).ceil().max(1.) as u32;
        if parts > 64 || parts > self.budget.rays {
            return Some(0.);
        }
        self.budget.rays -= parts;
        let delta = sub(to, from);
        for i in 0..parts {
            if let Some(t) = block_hit(
                self.world,
                add(from, scale(delta, f64::from(i) / f64::from(parts))),
                add(from, scale(delta, f64::from(i + 1) / f64::from(parts))),
                0.,
            ) {
                return Some((f64::from(i) + t) / f64::from(parts));
            }
        }
        None
    }
    /// Stop at the first surface, including allies/protected actors; use real HP
    /// for utilities and Cove. Cosmic Divide blocks the ray without taking damage.
    pub fn hitscan(&mut self, owner: u128, from: V3, to: V3, damage: f32) -> V3 {
        let mut boundary = self.terrain_limit(from, to).unwrap_or(1.);
        if boundary == 0. {
            return from;
        }
        let mut target = None;
        for a in self.actors {
            if self.budget.actors == 0 {
                return from;
            }
            self.budget.actors -= 1;
            if a.eligible
                && !self.world.status_immune(a.entity)
                && a.health > 0.
                && a.entity != Entity::Player(owner)
                && let Some(t) = a.bounds.segment(from, to)
                && t < boundary
            {
                boundary = t;
                target = Some(a.entity);
            }
        }
        if let Some(hit) = projectiles::utility_hit(
            self.shields,
            self.divides,
            self.objects,
            from,
            to,
            boundary,
            Some(Entity::Player(owner)),
        ) {
            boundary = hit.time;
            projectiles::apply_utility(
                hit.hit,
                self.shields,
                self.objects,
                damage,
                Some(Entity::Player(owner)),
                from,
            );
        } else if let Some(target) = target
            && self.world.recon_target(owner, target)
        {
            self.hurt(owner, target, damage);
        }
        add(from, scale(sub(to, from), boundary))
    }
}
