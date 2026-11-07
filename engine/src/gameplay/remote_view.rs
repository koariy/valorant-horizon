//! Server-side camera geometry for future astral/drone views. Never moves the actor.
use super::{Actor, World, geometry::*};
#[derive(Clone, Debug)]
pub struct RemoteView {
    pub origin: V3,
    pub position: V3,
    pub direction: V3,
    range: f64,
}
fn finite(v: V3) -> bool {
    v.into_iter().all(f64::is_finite)
}
fn body(at: V3) -> Box3 {
    Box3 {
        min: add(at, [-0.15; 3]),
        max: add(at, [0.15; 3]),
    }
}
impl RemoteView {
    pub fn new(world: &impl World, at: V3, range: f64) -> Option<Self> {
        if !finite(at)
            || at.into_iter().any(|v| v.abs() > 1_000_000_000.)
            || !range.is_finite()
            || range <= 0.
            || range > 64.
            || !clear_box(world, body(at))
        {
            return None;
        }
        Some(Self {
            origin: at,
            position: at,
            direction: [0., 0., 1.],
            range,
        })
    }
    /// Delta is a server-derived movement intent; clamp speed and sweep the whole path.
    pub fn step(&mut self, world: &impl World, delta: V3) -> bool {
        if !finite(delta) || !length(delta).is_finite() {
            return false;
        }
        let delta = scale(unit(delta), length(delta).min(0.5));
        let next = add(self.position, delta);
        let from_origin = sub(next, self.origin);
        let next = if length(from_origin) > self.range {
            add(self.origin, scale(unit(from_origin), self.range))
        } else {
            next
        };
        let movement = sweep(world, body(self.position), sub(next, self.position));
        self.position = add(self.position, movement);
        length(movement) > 0.
    }
    pub fn look(&mut self, direction: V3) -> bool {
        if !finite(direction) || !length(direction).is_finite() || length(direction) < 1e-6 {
            return false;
        }
        self.direction = unit(direction);
        true
    }
    /// Use the remote eye, not the stationary body's crosshair; reuse common occlusion.
    pub fn target(&self, world: &impl World, owner: &Actor, range: f64) -> Option<Actor> {
        if !range.is_finite() || range <= 0. || range > 64. {
            return None;
        }
        let proxy = Actor {
            position: self.position,
            bounds: Box3 {
                min: self.position,
                max: self.position,
            },
            direction: self.direction,
            ..owner.clone()
        };
        aim(world, &proxy, range, false)
    }
}
