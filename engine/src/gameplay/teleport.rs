use super::{Actor, World, geometry::*};
pub fn safe(world: &mut impl World, actor: &Actor, position: V3) -> bool {
    let super::Entity::Player(id) = actor.entity else {
        return false;
    };
    let body = actor.bounds.translated(sub(position, actor.position));
    if clear_box(world, body)
        && !world
            .actors()
            .iter()
            .any(|a| a.entity != actor.entity && a.eligible && a.bounds.intersects(body))
    {
        world.move_player(id, position)
    } else {
        false
    }
}

/// Aim-selected, loaded destination with solid ground and enough space for the real body.
pub fn target(world: &impl World, a: &Actor, range: f64) -> Option<V3> {
    let from = a.eye();
    let end = add(from, scale(a.direction, range));
    let point = if let Some(hit) = block_hit(world, from, end, 0.0) {
        add(
            add(from, scale(sub(end, from), hit)),
            scale(a.direction, -0.4),
        )
    } else {
        end
    };
    let top = add(point, [0.0, 0.15, 0.0]);
    let bottom = add(top, [0.0, -6.0, 0.0]);
    let hit = block_hit(world, top, bottom, 0.0)?;
    let feet = add(add(top, scale(sub(bottom, top), hit)), [0.0, 0.02, 0.0]);
    (distance(a.position, feet) >= 0.5
        && distance(a.position, feet) <= range + 0.05
        && clear_box(world, a.bounds.translated(sub(feet, a.position))))
    .then_some(feet)
}
