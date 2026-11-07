use super::{geometry::*, *};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ObjectKind {
    Shade,
    Leer,
    Soul,
    BoomBot,
    BlastPack,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Object {
    pub id: u64,
    pub owner: u128,
    pub kind: ObjectKind,
    pub position: V3,
    pub health: f32,
    pub ticks: u32,
    pub velocity: V3,
    pub target: Option<Entity>,
}
impl Object {
    pub fn bounds(&self) -> Box3 {
        let (width, height) = match self.kind {
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
        .filter_map(|(i, o)| {
            o.bounds()
                .segment(from, to)
                .filter(|t| *t < wall)
                .map(|t| (t, i))
        })
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
