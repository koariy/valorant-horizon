use super::World;
use serde::{Deserialize, Serialize};
pub type V3 = [f64; 3];
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Box3 {
    pub min: V3,
    pub max: V3,
}
pub fn add(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] + b[i])
}
pub fn sub(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] - b[i])
}
pub fn scale(a: V3, t: f64) -> V3 {
    a.map(|v| v * t)
}
pub fn length(a: V3) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
pub fn unit(a: V3) -> V3 {
    let n = length(a);
    if n.is_finite() && n > 1e-9 {
        scale(a, 1.0 / n)
    } else {
        [0.0; 3]
    }
}
pub fn distance(a: V3, b: V3) -> f64 {
    length(sub(a, b))
}
impl Box3 {
    pub fn translated(self, d: V3) -> Self {
        Self {
            min: add(self.min, d),
            max: add(self.max, d),
        }
    }
    pub fn intersects(self, b: Self) -> bool {
        (0..3).all(|i| self.min[i] < b.max[i] - 1e-6 && self.max[i] > b.min[i] + 1e-6)
    }
    pub fn expanded(self, r: f64) -> Self {
        Self {
            min: self.min.map(|x| x - r),
            max: self.max.map(|x| x + r),
        }
    }
    /// Slab segment test; no tunnelling even for fast knives or thin entities.
    pub fn segment(self, from: V3, to: V3) -> Option<f64> {
        let mut lo = 0.0_f64;
        let mut hi = 1.0_f64;
        for i in 0..3 {
            let d = to[i] - from[i];
            if d.abs() < 1e-12 {
                if from[i] < self.min[i] || from[i] > self.max[i] {
                    return None;
                }
            } else {
                let a = (self.min[i] - from[i]) / d;
                let b = (self.max[i] - from[i]) / d;
                lo = lo.max(a.min(b));
                hi = hi.min(a.max(b));
                if lo > hi {
                    return None;
                }
            }
        }
        Some(lo)
    }
}
pub fn clear_box(world: &impl World, b: Box3) -> bool {
    if !b
        .min
        .iter()
        .chain(&b.max)
        .all(|v| v.is_finite() && v.abs() < 30_000_000.0)
    {
        return false;
    }
    for x in b.min[0].floor() as i32 - 1..=b.max[0].floor() as i32 + 1 {
        for y in b.min[1].floor() as i32 - 1..=b.max[1].floor() as i32 + 1 {
            for z in b.min[2].floor() as i32 - 1..=b.max[2].floor() as i32 + 1 {
                let Some(boxes) = world.boxes([x, y, z]) else {
                    return false;
                };
                if boxes.into_iter().any(|solid| b.intersects(solid)) {
                    return false;
                }
            }
        }
    }
    true
}
/// Sweep a player's box in small conservative steps (<= 0.05 block).
pub fn sweep(world: &impl World, b: Box3, delta: V3) -> V3 {
    let n = (length(delta) / 0.05).ceil() as usize;
    if n > 2048 || !delta.iter().all(|x| x.is_finite()) {
        return [0.0; 3];
    }
    let mut safe = [0.0; 3];
    for i in 1..=n.max(1) {
        let d = scale(delta, i as f64 / n.max(1) as f64);
        if !clear_box(world, b.translated(d)) {
            break;
        }
        safe = d;
    }
    safe
}
#[derive(Clone, Copy, Debug)]
pub struct Collision {
    pub time: f64,
    pub normal: V3,
    pub known: bool,
}
pub fn block_hit(world: &impl World, from: V3, to: V3, r: f64) -> Option<f64> {
    block_collision(world, from, to, r).map(|h| h.time)
}
pub fn block_collision(world: &impl World, from: V3, to: V3, r: f64) -> Option<Collision> {
    let min = std::array::from_fn::<_, 3, _>(|i| from[i].min(to[i]) - r);
    let max = std::array::from_fn::<_, 3, _>(|i| from[i].max(to[i]) + r);
    let unknown = Collision {
        time: 0.,
        normal: [0.; 3],
        known: false,
    };
    if !min.iter().chain(&max).all(|v| v.is_finite()) || distance(from, to) > 128.0 {
        return Some(unknown);
    }
    let mut first: Option<Collision> = None;
    for x in min[0].floor() as i32..=max[0].floor() as i32 {
        for y in min[1].floor() as i32..=max[1].floor() as i32 {
            for z in min[2].floor() as i32..=max[2].floor() as i32 {
                let Some(boxes) = world.boxes([x, y, z]) else {
                    return Some(unknown);
                };
                for b in boxes {
                    let b = b.expanded(r);
                    if let Some(t) = b.segment(from, to)
                        && first.as_ref().is_none_or(|old| t < old.time)
                    {
                        // The entry plane, rather than the nearest face at the hit point,
                        // determines the normal. At block edges a parallel face can be
                        // equally close, but reflecting/placing against it is incorrect.
                        let (axis, _) = (0..3)
                            .filter(|i| (to[*i] - from[*i]).abs() > 1e-12)
                            .map(|i| {
                                let d = to[i] - from[i];
                                let plane = if d > 0. { b.min[i] } else { b.max[i] };
                                (i, (plane - from[i]) / d)
                            })
                            .max_by(|(_, a), (_, b)| a.total_cmp(b))
                            .unwrap_or((0, 0.));
                        let sign = if to[axis] > from[axis] { -1. } else { 1. };
                        let mut normal = [0.; 3];
                        normal[axis] = sign;
                        first = Some(Collision {
                            time: t,
                            normal,
                            known: true,
                        });
                    }
                }
            }
        }
    }
    first
}

pub fn aim(
    world: &impl World,
    a: &super::Actor,
    range: f64,
    players: bool,
) -> Option<super::Actor> {
    let from = a.eye();
    let to = add(from, scale(a.direction, range));
    let block = block_hit(world, from, to, 0.0).unwrap_or(1.0);
    world
        .actors()
        .into_iter()
        .filter(|t| {
            t.entity != a.entity
                && t.eligible
                && t.health > 0.0
                && (!players || matches!(t.entity, super::Entity::Player(_)))
        })
        .filter_map(|t| {
            t.bounds
                .expanded(0.15)
                .segment(from, to)
                .filter(|hit| *hit < block)
                .map(|hit| (hit, t))
        })
        .min_by(|(x, _), (y, _)| x.total_cmp(y))
        .map(|(_, t)| t)
}

/// Horizontal facing preserves yaw even when looking straight up/down.
pub fn horizontal(v: V3) -> V3 {
    let n = v[0].hypot(v[2]);
    if n.is_finite() && n > 0.0 {
        [v[0] / n, 0.0, v[2] / n]
    } else {
        [0.0, 0.0, 1.0]
    }
}

pub fn dot(a: V3, b: V3) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
