// Shared plane geometry and audio state. This source is also compiled by the renderer adapter.
use serde::{Deserialize, Serialize};
use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};
static DEAFENED: AtomicBool = AtomicBool::new(false);
pub fn set_deafened(active: bool) {
    DEAFENED.store(active, Ordering::Relaxed);
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Divide {
    pub point: [f64; 3],
    pub normal: [f64; 3],
    pub ticks: u32,
}
impl Divide {
    pub fn new(a: [f64; 3], b: [f64; 3]) -> Option<Self> {
        if !a.into_iter().chain(b).all(f64::is_finite) {
            return None;
        }
        let x = b[0] - a[0];
        let z = b[2] - a[2];
        let n = x.hypot(z);
        if !n.is_finite() || n < 1. {
            return None;
        }
        Some(Self {
            point: a,
            normal: [-z / n, 0., x / n],
            ticks: 400,
        })
    }
    pub fn crossing(&self, a: [f64; 3], b: [f64; 3]) -> Option<f64> {
        let side = |p: [f64; 3]| {
            (0..3)
                .map(|i| (p[i] - self.point[i]) * self.normal[i])
                .sum::<f64>()
        };
        let x = side(a);
        let y = side(b);
        if x.is_finite() && x.abs() <= 1e-6 {
            return Some(0.);
        }
        let d = x - y;
        if !d.is_finite() || d.abs() < 1e-9 {
            return None;
        }
        let t = x / d;
        (t > 1e-6 && t <= 1.).then_some(t)
    }
}
pub fn audio_planes() -> &'static Mutex<Vec<Divide>> {
    static D: OnceLock<Mutex<Vec<Divide>>> = OnceLock::new();
    D.get_or_init(Default::default)
}
pub fn gain(from: [f64; 3], listener: [f64; 3]) -> f32 {
    if audio_planes()
        .lock()
        .ok()
        .is_some_and(|p| p.iter().any(|d| d.crossing(from, listener).is_some()))
    {
        0.1
    } else {
        1.
    }
}
pub fn listener() -> &'static Mutex<[f64; 3]> {
    static L: OnceLock<Mutex<[f64; 3]>> = OnceLock::new();
    L.get_or_init(|| Mutex::new([0.; 3]))
}
pub fn sound_gain(from: [f64; 3]) -> f32 {
    let gain = listener().lock().ok().map_or(1., |p| gain(from, *p));
    if DEAFENED.load(Ordering::Relaxed) {
        gain * 0.03
    } else {
        gain
    }
}
