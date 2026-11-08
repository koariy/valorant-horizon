//! Client-only presentation. No world mutations or authoritative targeting.
pub mod mesh;
pub mod ui;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quality {
    Low,
    #[default]
    Medium,
    High,
}
impl Quality {
    pub fn next(self) -> Self {
        match self {
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Low,
        }
    }
    pub fn particles(self) -> usize {
        match self {
            Self::Low => 48,
            Self::Medium => 128,
            Self::High => 256,
        }
    }
    pub fn quads(self) -> usize {
        match self {
            Self::Low => 8192,
            Self::Medium => 16384,
            Self::High => 24576,
        }
    }
    pub fn distance(self) -> f64 {
        match self {
            Self::Low => 48.,
            Self::Medium => 80.,
            Self::High => 128.,
        }
    }
    pub fn segments(self) -> usize {
        match self {
            Self::Low => 8,
            Self::Medium => 12,
            Self::High => 18,
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub quality: Quality,
    pub scale: f32,
    pub x: f32,
    pub y: f32,
    pub sound: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            quality: Quality::Medium,
            scale: 1.,
            x: 0.5,
            y: 0.88,
            sound: true,
        }
    }
}
impl Settings {
    pub fn decode(text: &str) -> Self {
        let mut s: Self = serde_json::from_str(text).unwrap_or_default();
        s.sanitize();
        s
    }
    pub fn sanitize(&mut self) {
        self.scale = finite(self.scale, 1.).clamp(0.65, 1.5);
        self.x = finite(self.x, 0.5).clamp(0., 1.);
        self.y = finite(self.y, 0.88).clamp(0., 1.);
    }
    pub fn encode(&self) -> String {
        let mut s = *self;
        s.sanitize();
        serde_json::to_string(&s).unwrap_or_default()
    }
    pub fn origin(&self, screen: [f32; 2], size: [f32; 2]) -> [f32; 2] {
        std::array::from_fn(|i| (screen[i] - size[i] * self.scale).max(0.) * [self.x, self.y][i])
    }
}
fn finite(v: f32, default: f32) -> f32 {
    if v.is_finite() { v } else { default }
}

/// Fixed-size, cross-dimension cosmetic emission budget. No retained entity IDs.
#[derive(Default)]
pub struct EmissionBudget {
    tick: Option<i64>,
    count: u16,
}
impl EmissionBudget {
    pub fn allow(&mut self, tick: i64) -> bool {
        if self.tick != Some(tick) {
            self.tick = Some(tick);
            self.count = 0;
        }
        if self.count >= 32 {
            return false;
        }
        self.count += 1;
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    Cast,
    Ultimate,
    Ready,
    Flash,
    Phase,
}
impl Cue {
    pub fn sound(self, agent: usize) -> (&'static str, f32, f32) {
        match self {
            Self::Ultimate => ("minecraft:block.beacon.activate", 0.35, 0.8),
            Self::Ready => ("minecraft:block.note_block.pling", 0.16, 1.5),
            Self::Flash => ("minecraft:block.amethyst_block.chime", 0.25, 1.6),
            Self::Phase => ("minecraft:entity.enderman.teleport", 0.25, 1.1),
            Self::Cast => match agent {
                2 | 4 | 6 => ("minecraft:item.firecharge.use", 0.25, 1.1),
                9 => ("minecraft:entity.player.splash", 0.25, 1.1),
                14 | 18 => ("minecraft:block.amethyst_block.hit", 0.25, 1.6),
                1 | 20 => ("minecraft:block.enchantment_table.use", 0.25, 1.2),
                7 => ("minecraft:block.brewing_stand.brew", 0.25, 0.9),
                _ => ("minecraft:entity.evoker.cast_spell", 0.2, 1.4),
            },
        }
    }
}
