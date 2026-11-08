//! Bounded target ordering and short swept contact for controlled companions/plasma.
use super::*;
pub fn nearest(c: &mut Context<'_, impl World>, owner: u128, at: V3, range: f64) -> Vec<Actor> {
    let mut targets = c.enemies(owner, at, range);
    targets.retain(|t| c.world.bot_target(owner, t.entity));
    targets.sort_by(|a, b| distance(a.position, at).total_cmp(&distance(b.position, at)));
    targets
}
pub struct Contact {
    pub at: V3,
    pub target: Option<Entity>,
    pub blocked: bool,
}
pub fn contact(
    c: &mut Context<'_, impl World>,
    owner: u128,
    from: V3,
    to: V3,
    padding: f64,
    utilities: bool,
    ignore: Option<u64>,
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
            target = Some(actor.entity);
            blocked = true;
        }
    }
    if utilities
        && let Some(hit) = projectiles::utility_hit_from(
            c.shields,
            c.divides,
            c.objects,
            from,
            to,
            limit,
            projectiles::UtilitySource {
                attacker: Some(Entity::Player(owner)),
                ignore_object: ignore,
            },
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

/// Shared remote ground companion; caller supplies damage/concuss/detain policy.
pub struct Pilot {
    pub camera: Camera,
    pub left: u32,
    pub hurt: u64,
    pub leap: u32,
}
pub enum Drive {
    Moving,
    Impact { at: V3, target: Option<Entity> },
    Ended,
}
impl Pilot {
    pub fn lunge(
        &mut self,
        direction: V3,
        c: &mut Context<'_, impl World>,
        speed: f64,
        lift: f64,
    ) -> bool {
        if self.leap > 0 {
            return false;
        }
        let Some(o) = c
            .objects
            .iter_mut()
            .find(|o| o.id == self.camera.source && o.health > 0.)
        else {
            return false;
        };
        o.velocity = add(
            scale(horizontal(direction), speed.clamp(0.3, 0.9)),
            [0., lift.clamp(0.1, 0.4), 0.],
        );
        self.leap = 12;
        true
    }
    pub fn step(
        &mut self,
        id: u128,
        a: &Actor,
        c: &mut Context<'_, impl World>,
        speed: f64,
    ) -> Drive {
        self.left = self.left.saturating_sub(1);
        if self.left == 0
            || c.world.protection_state(id).hurt_epoch != self.hurt
            || !self.camera.update(a, c, false)
        {
            return Drive::Ended;
        }
        let Some(mut object) = c
            .objects
            .iter()
            .find(|o| o.id == self.camera.source && o.health > 0. && o.ticks > 0)
            .cloned()
        else {
            return Drive::Ended;
        };
        let mut impact = None;
        if self.leap > 0 {
            self.leap -= 1;
            object.velocity[1] -= 0.07;
            let end = add(object.position, object.velocity);
            let hit = contact(
                c,
                id,
                add(object.position, [0., 0.25, 0.]),
                add(end, [0., 0.25, 0.]),
                0.25,
                false,
                None,
            );
            object.position = add(hit.at, [0., -0.25, 0.]);
            if hit.blocked || self.leap == 0 {
                impact = Some(hit.target);
            }
        } else {
            walking::step(
                c.world,
                &mut object,
                a.movement,
                if length(a.movement) > 0.01 {
                    speed.clamp(0.1, 0.4)
                } else {
                    0.
                },
            );
        }
        if distance(self.camera.anchor, object.position) > 28. {
            return Drive::Ended;
        }
        self.camera.remote.position = add(object.position, [0., 0.35, 0.]);
        let at = object.position;
        if let Some(stored) = c.objects.iter_mut().find(|o| o.id == object.id) {
            *stored = object;
        }
        if let Some(target) = impact {
            Drive::Impact { at, target }
        } else {
            Drive::Moving
        }
    }
}
