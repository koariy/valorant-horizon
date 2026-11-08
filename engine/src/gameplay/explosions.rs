use super::{geometry::*, objects::*, *};
#[derive(Clone, Copy)]
pub struct Explosion {
    pub owner: u128,
    pub position: V3,
    pub radius: f64,
    pub damage: f32,
    pub push: f64,
    pub self_damage: bool,
}
pub fn apply(w: &mut impl World, objects: &mut Vec<Object>, e: Explosion) -> Vec<(u128, Entity)> {
    w.burst_fx(e.position);
    let mut kills = Vec::new();
    for a in w
        .actors()
        .into_iter()
        .filter(|a| a.eligible && a.health > 0.0)
    {
        let point = add(
            a.position,
            [0.0, (a.bounds.max[1] - a.bounds.min[1]) * 0.5, 0.0],
        );
        let d = distance(e.position, point);
        if d > e.radius || block_hit(w, e.position, point, 0.0).is_some() {
            continue;
        }
        let own = a.entity == Entity::Player(e.owner);
        if !own && !w.can_harm(e.owner, a.entity) {
            continue;
        }
        let factor = (1.0 - d / e.radius).max(0.0);
        if !own || e.self_damage {
            let hit = w.hurt_with(
                e.owner,
                a.entity,
                e.damage * factor as f32,
                "minecraft:explosion",
            );
            if hit.killed {
                kills.push((e.owner, a.entity));
            }
        }
        let force = add(
            scale(unit(sub(point, e.position)), e.push * factor),
            [0.0, e.push * 0.2 * factor, 0.0],
        );
        w.push_entity(a.entity, add(a.velocity, force));
    }
    for o in objects.iter_mut().filter(|o| o.kind.damageable()) {
        let point = add(o.position, [0.0, 0.2, 0.0]);
        let d = distance(e.position, point);
        if d < e.radius && block_hit(w, e.position, point, 0.0).is_none() {
            o.damage(
                e.damage * (1.0 - d / e.radius) as f32,
                Some(Entity::Player(e.owner)),
                e.position,
            );
        }
    }
    objects.retain(|o| o.health > 0.0 || matches!(o.kind, ObjectKind::Echo));
    kills
}
