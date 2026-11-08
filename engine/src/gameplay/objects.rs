use super::{geometry::*, *};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ObjectKind {
    Trapwire,
    Spycam,
    CageDevice,
    Drone,
    ReconBolt,
    Shade,
    Leer,
    Soul,
    BoomBot,
    BlastPack,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Object {
    pub id: u64,
    #[serde(rename = "o")]
    pub owner: u128,
    pub kind: ObjectKind,
    pub position: V3,
    pub health: f32,
    pub ticks: u32,
    pub velocity: V3,
    pub target: Option<Entity>,
}
impl ObjectKind {
    pub fn is_advanced(self) -> bool {
        matches!(self, Self::Trapwire | Self::Spycam | Self::CageDevice)
    }
}
impl Object {
    pub fn hit(&self, from: V3, to: V3, padding: f64) -> Option<f64> {
        if !matches!(self.kind, ObjectKind::Trapwire) {
            return self.bounds().expanded(padding).segment(from, to);
        }
        let d = sub(self.velocity, self.position);
        let n = dot(d, d);
        if n < 1e-9 {
            return None;
        }
        let steps = (distance(from, to) / 0.04).ceil().clamp(1., 256.) as u32;
        (0..=steps).find_map(|i| {
            let t = f64::from(i) / f64::from(steps);
            let p = add(from, scale(sub(to, from), t));
            let q = add(
                self.position,
                scale(d, (dot(sub(p, self.position), d) / n).clamp(0., 1.)),
            );
            (distance(p, q) < 0.05 + padding).then_some(t)
        })
    }

    pub fn bounds(&self) -> Box3 {
        if matches!(self.kind, ObjectKind::Trapwire) {
            return Box3 {
                min: std::array::from_fn(|i| self.position[i].min(self.velocity[i]) - 0.05),
                max: std::array::from_fn(|i| self.position[i].max(self.velocity[i]) + 0.05),
            };
        }
        let (width, height) = match self.kind {
            ObjectKind::Trapwire | ObjectKind::Spycam | ObjectKind::CageDevice => (0.12, 0.25),
            ObjectKind::Drone => (0.15, 0.3),
            ObjectKind::ReconBolt => (0.12, 0.25),
            ObjectKind::Soul => (0.15, 0.3),
            ObjectKind::Leer => (0.35, 0.7),
            ObjectKind::Shade => (0.35, 1.8),
            ObjectKind::BoomBot => (0.3, 0.45),
            ObjectKind::BlastPack => (0.2, 0.12),
        };
        Box3 {
            min: add(self.position, [-width, 0.0, -width]),
            max: add(self.position, [width, height, width]),
        }
    }
}
pub fn strike(objects: &mut Vec<Object>, a: &Actor, w: &impl World) -> bool {
    let from = a.eye();
    let to = add(from, scale(a.direction, 3.0));
    let wall = block_hit(w, from, to, 0.0).unwrap_or(1.0);
    let wall = w
        .actors()
        .iter()
        .filter(|t| t.entity != a.entity && t.eligible)
        .filter_map(|t| t.bounds.segment(from, to))
        .fold(wall, f64::min);
    let hit = objects
        .iter()
        .enumerate()
        .filter_map(|(i, o)| o.hit(from, to, 0.).filter(|t| *t < wall).map(|t| (t, i)))
        .min_by(|(a, _), (b, _)| a.total_cmp(b))
        .map(|(_, i)| i);
    if let Some(i) = hit {
        objects[i].health -= 4.0;
        objects.retain(|o| o.health > 0.0);
        true
    } else {
        false
    }
}
