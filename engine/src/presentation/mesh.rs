use super::{Cue, Quality};
use crate::gameplay::{
    Snapshot,
    geometry::{V3, add, distance, sub},
    smoke,
    zones::ZoneKind,
};
pub type Quad = [[f32; 3]; 4];
#[derive(Clone, Copy, Debug)]
pub struct Face {
    pub points: Quad,
    pub color: [u8; 4],
    distance: f32,
}
pub struct Frame {
    pub faces: Vec<Face>,
    quality: Quality,
    pub dropped: usize,
    pub critical: bool,
    particles: usize,
}
impl Frame {
    pub fn new(quality: Quality) -> Self {
        Self {
            faces: Vec::with_capacity(quality.quads()),
            quality,
            dropped: 0,
            critical: false,
            particles: 0,
        }
    }
    pub fn push(&mut self, points: Quad, color: [u8; 4]) {
        if color[3] == 0 || !points.iter().flatten().all(|v| v.is_finite()) {
            return;
        }
        let center: [f32; 3] =
            std::array::from_fn(|i| points.iter().map(|p| p[i]).sum::<f32>() * 0.25);
        let d = center.iter().map(|v| v * v).sum::<f32>();
        // Cull only when every vertex is distant: retain large walls crossing the camera.
        let nearest: f64 = (0..3)
            .map(|axis| {
                let lo = points
                    .iter()
                    .map(|p| f64::from(p[axis]))
                    .fold(f64::INFINITY, f64::min);
                let hi = points
                    .iter()
                    .map(|p| f64::from(p[axis]))
                    .fold(f64::NEG_INFINITY, f64::max);
                0f64.clamp(lo, hi).powi(2)
            })
            .sum();
        if !self.critical && nearest > self.quality.distance().powi(2) {
            self.dropped += 1;
            return;
        }
        let face = Face {
            points,
            color,
            distance: d,
        };
        if self.faces.len() < self.quality.quads() {
            self.faces.push(face);
        } else if self.critical {
            self.faces.pop();
            self.faces.push(face);
            self.dropped += 1;
        } else {
            self.dropped += 1;
        }
    }
    pub fn sorted(&mut self) -> &[Face] {
        // Depth-writing native pass: opaque first, all alpha faces back to front.
        self.faces.sort_by(|a, b| {
            (b.color[3] == 255)
                .cmp(&(a.color[3] == 255))
                .then_with(|| b.distance.total_cmp(&a.distance))
        });
        &self.faces
    }
    pub fn reset(&mut self, quality: Quality) {
        self.faces.clear();
        self.quality = quality;
        self.dropped = 0;
        self.critical = false;
        self.particles = 0;
        if self.faces.capacity() > quality.quads() * 2 {
            self.faces.shrink_to(quality.quads());
        }
    }
    pub fn sphere(&mut self, center: V3, radius: f64, color: [u8; 4]) {
        for points in sphere_mesh(center, radius, self.quality) {
            self.push(points, color);
        }
    }
    pub fn smoke(&mut self, s: &smoke::Smoke, camera: V3, time: f64) {
        self.critical = true;
        let center = sub(s.center, camera);
        // Preserve full gameplay occlusion on every quality preset, including fade-out.
        self.sphere(center, s.radius, [s.color[0], s.color[1], s.color[2], 255]);
        self.critical = false;
        let fade = (f64::from(s.ticks) / 12.).clamp(0., 1.);
        let pulse = 0.04 * (time * 0.07 + center[0]).sin();
        self.sphere(
            center,
            s.radius + 0.08 + pulse,
            [s.color[0], s.color[1], s.color[2], (55. * fade) as u8],
        );
    }
    pub fn ring(&mut self, center: V3, r: f64, width: f64, color: [u8; 4]) {
        let n = self.quality.segments() * 2;
        for i in 0..n {
            let point = |j: usize, r: f64| {
                let a = j as f64 * std::f64::consts::TAU / n as f64;
                add(center, [a.cos() * r, 0., a.sin() * r]).map(|v| v as f32)
            };
            let q = [
                point(i, r),
                point(i + 1, r),
                point(i + 1, (r - width).max(0.)),
                point(i, (r - width).max(0.)),
            ];
            self.push(q, color);
            self.push([q[3], q[2], q[1], q[0]], color);
        }
    }
    pub fn surface(&mut self, z: &crate::gameplay::zones::Zone, camera: V3, time: f64) {
        use crate::gameplay::{geometry::scale, zones};
        let (color, _) = zone_style(z.kind);
        if matches!(
            z.kind,
            ZoneKind::FireWall | ZoneKind::WaterWall | ZoneKind::ElectricWall | ZoneKind::CyberCage
        ) {
            let core = match z.kind {
                ZoneKind::WaterWall => [35, 140, 190, 255],
                ZoneKind::ElectricWall => [30, 150, 230, 255],
                ZoneKind::CyberCage => [55, 120, 145, 255],
                _ => [235, 80, 15, 255],
            };
            for q in zones::box_quads(z.bounds, camera) {
                self.push(q, core);
            }
            // Flowing stripes remain cosmetic; the opaque silhouette never fades away early.
            let width = z.bounds.max[0] - z.bounds.min[0];
            let depth = z.bounds.max[2] - z.bounds.min[2];
            for i in 0..4 {
                let t = (time * 0.035 + i as f64 * 0.25).rem_euclid(1.);
                let mut b = z.bounds;
                b.min[1] += t * (z.bounds.max[1] - z.bounds.min[1]);
                b.max[1] = (b.min[1] + 0.06).min(z.bounds.max[1]);
                if width > depth {
                    b.min[2] -= 0.015;
                    b.max[2] += 0.015;
                } else {
                    b.min[0] -= 0.015;
                    b.max[0] += 0.015;
                }
                for q in zones::box_quads(b, camera) {
                    self.push(q, [color[0], color[1], color[2], 100]);
                }
            }
        } else {
            let mid = scale(add(z.bounds.min, z.bounds.max), 0.5);
            let r = (z.bounds.max[0] - z.bounds.min[0]) * 0.5;
            let at = sub([mid[0], z.bounds.min[1] + 0.025, mid[2]], camera);
            let fade = (z.ticks as f64 / 10.).clamp(0., 1.);
            self.ring(at, r, r, [color[0], color[1], color[2], (90. * fade) as u8]);
            self.ring(add(at, [0., 0.01, 0.]), r, 0.07, color);
            if matches!(z.kind, ZoneKind::Orbital) {
                for q in crate::gameplay::projectiles::beam(
                    at,
                    add(at, [0., (z.bounds.max[1] - z.bounds.min[1]).min(32.), 0.]),
                    r * 0.5,
                ) {
                    self.push(q, [255, 165, 70, 100]);
                }
            }
        }
    }
    pub fn zones(&mut self, s: &Snapshot, camera: V3, time: f64) {
        for z in &s.zones {
            let (color, speed) = zone_style(z.kind);
            let center: V3 = std::array::from_fn(|i| (z.bounds.min[i] + z.bounds.max[i]) * 0.5);
            let r = ((z.bounds.max[0] - z.bounds.min[0]) * 0.5).min(12.);
            let at = sub([center[0], z.bounds.min[1] + 0.035, center[2]], camera);
            if !matches!(
                z.kind,
                ZoneKind::WaterWall | ZoneKind::ElectricWall | ZoneKind::FireWall
            ) {
                self.ring(at, r, 0.055, color);
            }
            let count = self.quality.particles().min(24) / 3;
            for i in 0..count {
                let seed = i as f64 * 2.399963;
                let cycle = (time * speed + i as f64 * 0.17).rem_euclid(1.);
                let p = sub(
                    [
                        center[0] + seed.cos() * r * 0.8,
                        z.bounds.min[1]
                            + 0.1
                            + cycle * (z.bounds.max[1] - z.bounds.min[1]).clamp(0.3, 2.),
                        center[2] + seed.sin() * r * 0.8,
                    ],
                    camera,
                );
                self.spark(
                    p,
                    0.035 + cycle * 0.04,
                    [
                        color[0],
                        color[1],
                        color[2],
                        ((1. - cycle) * f64::from(color[3])) as u8,
                    ],
                );
            }
        }
    }
    pub fn spark(&mut self, p: V3, r: f64, color: [u8; 4]) {
        if self.particles >= self.quality.particles() {
            return;
        }
        self.particles += 1;
        // Crossed vertical ribbons, no camera-facing shader or transparency API assumed.
        for axis in [0, 2] {
            let mut q = [p; 4];
            q[0][axis] -= r;
            q[1][axis] -= r;
            q[1][1] += r * 2.;
            q[2][axis] += r;
            q[2][1] += r * 2.;
            q[3][axis] += r;
            let q = q.map(|p| p.map(|v| v as f32));
            self.push(q, color);
            self.push([q[3], q[2], q[1], q[0]], color);
        }
    }
}
#[derive(Clone, Debug)]
pub struct Particle {
    pub at: V3,
    pub velocity: V3,
    pub life: u8,
    pub color: [u8; 4],
}
#[derive(Default)]
pub struct Scene {
    pub particles: Vec<Particle>,
    previous: Option<V3>,
    active: [bool; 4],
    phased: bool,
    initialized: bool,
    charges: [u8; 4],
    flash: bool,
    pub cues: Vec<Cue>,
    agent: Option<usize>,
}
impl Scene {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn tick(&mut self, s: &Snapshot, position: Option<V3>, quality: Quality) {
        self.cues.clear();
        if self.agent != Some(s.agent) {
            self.initialized = false;
            self.previous = None;
            self.particles.clear();
            self.agent = Some(s.agent);
        }
        if self.initialized {
            if s.active[3] > 0 && !self.active[3] {
                self.cues.push(Cue::Ultimate);
            } else if s.charges.iter().zip(self.charges).any(|(a, b)| *a < b) {
                self.cues.push(Cue::Cast);
            }
            if s.charges.iter().zip(self.charges).any(|(a, b)| *a > b) {
                self.cues.push(Cue::Ready);
            }
            if s.vision.flash > 0 && !self.flash {
                self.cues.push(Cue::Flash);
            }
            if s.vision.phased != self.phased {
                self.cues.push(Cue::Phase);
            }
        }
        self.charges = s.charges;
        self.flash = s.vision.flash > 0;
        for p in &mut self.particles {
            p.life = p.life.saturating_sub(1);
            p.at = add(p.at, p.velocity);
            p.velocity[1] += 0.001;
        }
        self.particles.retain(|p| p.life > 0);
        self.particles.truncate(quality.particles());
        if let Some(at) = position.filter(|p| p.iter().all(|v| v.is_finite())) {
            if self.initialized && s.camera.is_none() {
                if let Some(old) = self.previous {
                    let d = distance(old, at);
                    if d > 0.9 {
                        let count = if d > 4. { 12 } else { 6 };
                        for i in 0..count {
                            let t = i as f64 / count as f64;
                            let p = std::array::from_fn(|a| old[a] + (at[a] - old[a]) * t);
                            self.burst(p, [100, 220, 255, 190], quality, 1);
                        }
                    }
                }
                if s.active[3] > 0 && !self.active[3] {
                    self.burst(add(at, [0., 1., 0.]), [255, 100, 130, 210], quality, 16);
                }
                if s.vision.phased != self.phased {
                    self.burst(add(at, [0., 0.7, 0.]), [170, 110, 255, 190], quality, 12);
                }
            }
            self.previous = if s.camera.is_none() { Some(at) } else { None };
        } else {
            self.previous = None;
        }
        self.active = s.active.map(|x| x > 0);
        self.phased = s.vision.phased;
        self.initialized = true;
    }
    pub fn burst(&mut self, at: V3, color: [u8; 4], quality: Quality, count: usize) {
        if !at.iter().all(|v| v.is_finite()) {
            return;
        }
        for i in 0..count.min(quality.particles().saturating_sub(self.particles.len())) {
            let a = i as f64 * 2.399963;
            self.particles.push(Particle {
                at,
                velocity: [a.cos() * 0.04, 0.015, a.sin() * 0.04],
                life: 20,
                color,
            });
        }
    }
    pub fn draw(&self, frame: &mut Frame, camera: V3, partial: f32) {
        for p in &self.particles {
            let at = sub(
                add(
                    p.at,
                    p.velocity.map(|v| v * f64::from(partial.clamp(0., 1.))),
                ),
                camera,
            );
            let mut c = p.color;
            c[3] = (f32::from(c[3]) * (f32::from(p.life) / 20.).min(1.)) as u8;
            frame.spark(at, 0.055, c);
        }
    }
}

/// Unit meshes cached once per preset; per-frame transforms allocate no intermediate vectors.
pub fn sphere_mesh(center: V3, radius: f64, quality: Quality) -> impl Iterator<Item = Quad> {
    static UNIT: std::sync::OnceLock<[Vec<Quad>; 3]> = std::sync::OnceLock::new();
    let meshes = UNIT.get_or_init(|| {
        std::array::from_fn(|i| {
            let q = [Quality::Low, Quality::Medium, Quality::High][i];
            let n = q.segments();
            let lat = (n / 2).next_multiple_of(2);
            let point = |i: usize, j: usize| {
                let a = std::f64::consts::PI * i as f64 / lat as f64;
                let b = std::f64::consts::TAU * j as f64 / n as f64;
                [
                    (a.sin() * b.cos()) as f32,
                    a.cos() as f32,
                    (a.sin() * b.sin()) as f32,
                ]
            };
            let mut faces = Vec::with_capacity(lat * n * 2);
            for i in 0..lat {
                for j in 0..n {
                    let q = [
                        point(i, j),
                        point(i + 1, j),
                        point(i + 1, j + 1),
                        point(i, j + 1),
                    ];
                    faces.push(q);
                    faces.push([q[3], q[2], q[1], q[0]]);
                }
            }
            faces
        })
    });
    let index = match quality {
        Quality::Low => 0,
        Quality::Medium => 1,
        Quality::High => 2,
    };
    let valid = radius.is_finite() && radius > 0. && center.iter().all(|v| v.is_finite());
    meshes[index]
        .iter()
        .take(if valid { usize::MAX } else { 0 })
        .map(move |q| {
            q.map(|p| std::array::from_fn(|i| (center[i] + f64::from(p[i]) * radius) as f32))
        })
}

fn zone_style(kind: ZoneKind) -> ([u8; 4], f64) {
    match kind {
        ZoneKind::WaterWall | ZoneKind::Whirlpool => ([50, 190, 235, 145], 0.04),
        ZoneKind::Acid => ([135, 230, 30, 150], 0.02),
        ZoneKind::ElectricWall | ZoneKind::ElectricPulse => ([80, 220, 255, 220], 0.12),
        ZoneKind::Slow => ([170, 235, 250, 150], 0.01),
        ZoneKind::Stim => ([130, 235, 175, 140], 0.03),
        ZoneKind::Gravity | ZoneKind::Nova | ZoneKind::Nightfall | ZoneKind::Seize => {
            ([180, 95, 235, 160], 0.035)
        }
        ZoneKind::CyberCage => ([100, 225, 235, 150], 0.03),
        _ => ([255, 150, 45, 190], 0.07),
    }
}
