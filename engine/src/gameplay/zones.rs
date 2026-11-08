use super::geometry::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ZoneKind {
    CyberCage,
    WaterWall,
    Whirlpool,
    Gravity,
    Nova,
    Acid,
    Stim,
    Orbital,
    Incendiary,
    FireWall,
    FirePool,
    Slow,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Zone {
    #[serde(rename = "o")]
    pub owner: u128,
    pub kind: ZoneKind,
    pub bounds: Box3,
    pub ticks: u32,
}
impl Zone {
    pub fn touches(&self, b: Box3) -> bool {
        if !self.bounds.intersects(b) {
            return false;
        }
        if matches!(self.kind, ZoneKind::FireWall | ZoneKind::WaterWall) {
            return true;
        }
        let cx = (self.bounds.min[0] + self.bounds.max[0]) * 0.5;
        let cz = (self.bounds.min[2] + self.bounds.max[2]) * 0.5;
        let r = (self.bounds.max[0] - self.bounds.min[0]) * 0.5;
        (cx - cx.clamp(b.min[0], b.max[0])).hypot(cz - cz.clamp(b.min[2], b.max[2])) <= r
    }
}

pub fn box_quads(b: Box3, camera: V3) -> Vec<[[f32; 3]; 4]> {
    let [x, y, z] = sub(b.min, camera).map(|v| v as f32);
    let [a, c, d] = sub(b.max, camera).map(|v| v as f32);
    vec![
        [[x, y, z], [x, c, z], [a, c, z], [a, y, z]],
        [[a, y, d], [a, c, d], [x, c, d], [x, y, d]],
        [[x, y, d], [x, c, d], [x, c, z], [x, y, z]],
        [[a, y, z], [a, c, z], [a, c, d], [a, y, d]],
        [[x, c, z], [x, c, d], [a, c, d], [a, c, z]],
        [[x, y, d], [x, y, z], [a, y, z], [a, y, d]],
    ]
}
