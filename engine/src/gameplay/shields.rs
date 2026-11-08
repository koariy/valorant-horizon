//! Shared destructible projectile shields. Smoke survives shield destruction.
use super::{geometry::*, *};
use serde::{Deserialize, Serialize};
pub const MAX_SHIELDS: usize = 4;
#[derive(Clone, Copy, Default)]
pub struct CollisionPadding {
    pub actors: f64,
    pub terrain: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shield {
    pub owner: u128,
    pub center: V3,
    pub radius: f64,
    pub ticks: u32,
    pub health: f32,
    pub active: bool,
    pub spent: bool,
}
impl Shield {
    pub fn damage(&mut self, amount: f32) {
        self.health = (self.health - amount.max(0.)).max(0.);
        if self.health == 0. {
            self.active = false;
        }
    }
    pub fn crossing(&self, from: V3, to: V3) -> Option<f64> {
        if !self.active || self.health <= 0. || self.ticks == 0 {
            return None;
        }
        let v = sub(to, from);
        let o = sub(from, self.center);
        let a = dot(v, v);
        if a < 1e-12 {
            return None;
        }
        let b = 2. * dot(o, v);
        let c = dot(o, o) - self.radius * self.radius;
        let d = b * b - 4. * a * c;
        if d < 0. {
            return None;
        }
        [(-b - d.sqrt()) / (2. * a), (-b + d.sqrt()) / (2. * a)]
            .into_iter()
            .filter(|t| *t >= 0. && *t <= 1.)
            .min_by(f64::total_cmp)
    }
}
pub fn intercept(w: &impl World, shields: &mut [Shield], from: V3, to: V3, damage: f32) -> bool {
    if !shields.iter().any(|s| s.active) {
        return false;
    }
    intercept_until(
        w,
        shields,
        from,
        to,
        damage,
        (1., CollisionPadding::default()),
    )
}
fn intercept_until(
    w: &impl World,
    shields: &mut [Shield],
    from: V3,
    to: V3,
    damage: f32,
    trace: (f64, CollisionPadding),
) -> bool {
    let (boundary, padding) = trace;
    let limit = impact_limit(w, from, to, padding).min(boundary);
    let hit = shields
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.crossing(from, to).filter(|t| *t <= limit).map(|t| (i, t)))
        .min_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((i, _)) = hit {
        let s = &mut shields[i];
        s.damage(damage);
        true
    } else {
        false
    }
}

pub fn impact_limit(w: &impl World, from: V3, to: V3, padding: CollisionPadding) -> f64 {
    let limit = block_hit(w, from, to, padding.terrain).unwrap_or(1.);
    w.actors()
        .iter()
        .filter(|a| a.eligible && a.bounds.expanded(0.1).segment(from, from).is_none())
        .filter_map(|a| a.bounds.expanded(padding.actors).segment(from, to))
        .fold(limit, f64::min)
}

/// Resolve the nearest shield/Divide before terrain or an ordinary actor.
pub fn intercept_barriers(
    w: &impl World,
    shields: &mut [Shield],
    walls: &[divide::Divide],
    from: V3,
    to: V3,
    damage: f32,
    padding: CollisionPadding,
) -> bool {
    let plane = walls
        .iter()
        .filter(|d| d.ticks > 0)
        .filter_map(|d| d.crossing(from, to))
        .min_by(f64::total_cmp);
    intercept_until(w, shields, from, to, damage, (plane.unwrap_or(1.), padding))
        || plane.is_some_and(|t| t <= impact_limit(w, from, to, padding))
}
