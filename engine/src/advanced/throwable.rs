//! Shared short swept flight. Callers decide fuse, sticking or reflection policy.
use super::*;
pub enum Flight {
    Air,
    Surface(V3),
    Unknown,
}
pub fn advance(w: &impl World, o: &mut Object, gravity: f64, radius: f64) -> Flight {
    o.velocity[1] -= gravity;
    let end = add(o.position, o.velocity);
    if let Some(hit) = block_collision(w, o.position, end, radius) {
        if !hit.known {
            o.health = 0.;
            return Flight::Unknown;
        }
        o.position = add(
            add(o.position, scale(o.velocity, hit.time)),
            scale(hit.normal, 0.03),
        );
        Flight::Surface(hit.normal)
    } else {
        o.position = end;
        Flight::Air
    }
}
pub fn reflect(o: &mut Object, normal: V3, damping: f64) {
    o.velocity = scale(
        sub(o.velocity, scale(normal, 2. * dot(o.velocity, normal))),
        damping.clamp(0., 1.),
    );
}
