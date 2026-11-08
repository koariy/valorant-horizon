//! Shared, nonlethal and reversible decay; normal damage is never refunded.
use super::{geometry::*, *};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
pub const MAX_FIELDS: usize = 8;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Field {
    #[serde(rename = "o", alias = "owner")]
    pub owner: u128,
    #[serde(rename = "b", alias = "bounds")]
    pub bounds: Box3,
    #[serde(rename = "s", alias = "sphere")]
    pub sphere: bool,
}
impl Field {
    pub fn contains(&self, p: V3) -> bool {
        if self.sphere {
            distance(p, scale(add(self.bounds.min, self.bounds.max), 0.5))
                < (self.bounds.max[0] - self.bounds.min[0]) * 0.5
        } else {
            (0..3).all(|i| p[i] >= self.bounds.min[i] && p[i] <= self.bounds.max[i])
        }
    }
    pub fn crossed(&self, a: &Actor) -> bool {
        let to = a.eye();
        let from = sub(to, a.velocity);
        if !self.sphere {
            return self.bounds.segment(from, to).is_some();
        }
        let center = scale(add(self.bounds.min, self.bounds.max), 0.5);
        let delta = sub(to, from);
        let length2 = dot(delta, delta);
        let t = if length2 > 0. {
            (dot(sub(center, from), delta) / length2).clamp(0., 1.)
        } else {
            0.
        };
        self.contains(add(from, scale(delta, t)))
    }
    pub fn ball(owner: u128, center: V3, r: f64) -> Self {
        Self {
            owner,
            bounds: Box3 {
                min: add(center, [-r; 3]),
                max: add(center, [r; 3]),
            },
            sphere: true,
        }
    }
}
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Decay {
    pub debt: f32,
    pub delay: u32,
}
pub fn tick(w: &mut impl World, fields: &[Field], debts: &mut HashMap<Entity, Decay>) {
    for a in w
        .actors()
        .into_iter()
        .filter(|a| a.eligible && a.health > 0.)
    {
        let exposed = fields.iter().any(|f| {
            a.entity != Entity::Player(f.owner) && w.can_harm(f.owner, a.entity) && f.crossed(&a)
        });
        if exposed {
            let d = debts.entry(a.entity).or_default();
            d.delay = 40;
            let amount = 0.4f32
                .min((a.health - 1.).max(0.))
                .min((a.max_health * 0.9 - d.debt).max(0.));
            let ceiling = (a.max_health - d.debt - amount).max(1.);
            let target = (a.health - amount).min(ceiling).max(1.);
            if target < a.health && w.temporary_health(a.entity, target) {
                d.debt += amount;
            }
        } else if let Some(d) = debts.get_mut(&a.entity) {
            d.delay = d.delay.saturating_sub(1);
            if d.delay == 0 {
                let amount = d.debt.min(0.25).min((a.max_health - a.health).max(0.));
                if amount > 0. && w.heal(a.entity, amount) {
                    d.debt -= amount;
                }
                if a.health >= a.max_health {
                    d.debt = 0.;
                }
            }
        }
    }
    let actors = w.actors();
    debts.retain(|e, d| {
        d.debt > 0.001
            && match actors.iter().find(|a| a.entity == *e) {
                Some(a) => a.eligible && a.health > 0.,
                None => matches!(e, Entity::Player(_)),
            }
    });
}

/// One-shot nonlethal decay, sharing Viper's persisted repayment ledger.
pub fn pulse(
    w: &mut impl World,
    a: &Actor,
    debts: &mut HashMap<Entity, Decay>,
    amount: f32,
    delay: u32,
) -> bool {
    let amount = amount.max(0.).min((a.health - 1.).max(0.));
    if amount <= 0. || !w.temporary_health(a.entity, a.health - amount) {
        return false;
    }
    let d = debts.entry(a.entity).or_default();
    d.debt += amount;
    d.delay = d.delay.max(delay);
    true
}
