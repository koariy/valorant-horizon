//! Bounded target ordering and short swept contact for controlled companions/plasma.
use super::*;
pub fn nearest(c: &mut Context<'_, impl World>, owner: u128, at: V3, range: f64) -> Vec<Actor> {
    let mut targets = c.enemies(owner, at, range);
    targets.sort_by(|a, b| distance(a.position, at).total_cmp(&distance(b.position, at)));
    targets
}
pub struct Contact {
    pub at: V3,
    pub target: Option<Actor>,
    pub blocked: bool,
}
pub fn contact(
    c: &mut Context<'_, impl World>,
    owner: u128,
    from: V3,
    to: V3,
    padding: f64,
    utilities: bool,
) -> Contact {
    if c.budget.rays == 0 || distance(from, to) > 2. {
        return Contact {
            at: from,
            target: None,
            blocked: true,
        };
    }
    c.budget.rays -= 1;
    let terrain = block_collision(c.world, from, to, padding);
    let mut limit = terrain.map_or(1., |h| if h.known { h.time } else { 0. });
    let mut blocked = terrain.is_some();
    let mut target = None;
    for actor in c.actors {
        if c.budget.actors == 0 {
            return Contact {
                at: from,
                target: None,
                blocked: true,
            };
        }
        c.budget.actors -= 1;
        if actor.eligible
            && actor.health > 0.
            && actor.entity != Entity::Player(owner)
            && !c.world.status_immune(actor.entity)
            && let Some(t) = actor.bounds.expanded(padding).segment(from, to)
            && t < limit
        {
            limit = t;
            target = Some(actor.clone());
            blocked = true;
        }
    }
    if utilities
        && let Some(hit) = projectiles::utility_hit(
            c.shields,
            c.divides,
            c.objects,
            from,
            to,
            limit,
            Some(Entity::Player(owner)),
        )
    {
        limit = hit.time;
        target = None;
        blocked = true;
    }
    Contact {
        at: add(from, scale(sub(to, from), limit)),
        target,
        blocked,
    }
}
