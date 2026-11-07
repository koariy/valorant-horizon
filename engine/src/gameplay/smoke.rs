use super::geometry::{V3, distance};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Smoke {
    pub center: V3,
    pub radius: f64,
    pub ticks: u32,
    pub owner: u128,
    pub color: [u8; 4],
}
impl Smoke {
    pub fn contains(&self, p: V3) -> bool {
        distance(p, self.center) < self.radius
    }
}
/// Opaque sphere quads, both windings. Inside rendering also applies dense fog.
pub fn sphere(center: V3, radius: f64) -> Vec<[[f32; 3]; 4]> {
    let mut quads = Vec::new();
    let point = |lat: usize, lon: usize| {
        let a = std::f64::consts::PI * lat as f64 / 12.0;
        let b = std::f64::consts::TAU * lon as f64 / 24.0;
        [
            center[0] + radius * a.sin() * b.cos(),
            center[1] + radius * a.cos(),
            center[2] + radius * a.sin() * b.sin(),
        ]
        .map(|v| v as f32)
    };
    for lat in 0..12 {
        for lon in 0..24 {
            let q = [
                point(lat, lon),
                point(lat + 1, lon),
                point(lat + 1, lon + 1),
                point(lat, lon + 1),
            ];
            quads.push(q);
            quads.push([q[3], q[2], q[1], q[0]]);
        }
    }
    quads
}
