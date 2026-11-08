use super::geometry::{V3, distance};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Smoke {
    #[serde(rename = "p", alias = "center")]
    pub center: V3,
    #[serde(rename = "r", alias = "radius")]
    pub radius: f64,
    #[serde(rename = "t", alias = "ticks")]
    pub ticks: u32,
    #[serde(rename = "o")]
    pub owner: u128,
    #[serde(rename = "c", alias = "color")]
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

/// Low-poly utility silhouette: sixteen double-sided triangles encoded as quads.
/// Keep small deployables cheap even with every object slot occupied.
pub fn orb(center: V3, radius: f64) -> Vec<[[f32; 3]; 4]> {
    if !radius.is_finite() || radius <= 0. || !center.into_iter().all(f64::is_finite) {
        return vec![];
    }
    let ring = [[1., 0., 0.], [0., 0., 1.], [-1., 0., 0.], [0., 0., -1.]];
    let mut out = Vec::with_capacity(16);
    for top in [-1., 1.] {
        for i in 0..4 {
            let tip = [0., top, 0.];
            let q = [tip, ring[i], ring[(i + 1) % 4], tip]
                .map(|p| std::array::from_fn(|axis| (center[axis] + p[axis] * radius) as f32));
            out.push(q);
            out.push([q[3], q[2], q[1], q[0]]);
        }
    }
    out
}
