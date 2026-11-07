use crate::{
    abilities::{Ability, Kind, Runtime},
    gameplay::{geometry::*, projectiles::*, smoke::Smoke, zones::*, *},
};
pub const NAMES: [&str; 4] = ["Stim Beacon", "Incendiary", "Sky Smoke", "Orbital Strike"];
#[derive(Clone, Copy)]
pub struct Settings {
    pub range: f64,
    pub smoke_ticks: u32,
    pub strike_ticks: u32,
    pub fire_ticks: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            range: 48.,
            smoke_ticks: 380,
            strike_ticks: 80,
            fire_ticks: 140,
        }
    }
}
pub const DEFINITIONS: [Ability; 4] = [
    Ability {
        kind: Kind::Active,
        cooldown: 600,
        duration: 240,
        charges: 1,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 600,
        duration: 0,
        charges: 1,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 800,
        duration: 0,
        charges: 3,
    },
    Ability {
        kind: Kind::Ultimate,
        cooldown: 2400,
        duration: 80,
        charges: 1,
    },
];
pub struct Brimstone {
    pub slots: [Runtime; 4],
    pub equipped: Option<usize>,
    pub marks: Vec<[f64; 3]>,
    pub settings: Settings,
}
impl Default for Brimstone {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|i| Runtime {
                charges: DEFINITIONS[i].charges,
                ..Default::default()
            }),
            equipped: None,
            marks: vec![],
            settings: Settings::default(),
        }
    }
}
pub struct Context<'a, W: World> {
    pub world: &'a mut W,
    pub projectiles: &'a mut Vec<Projectile>,
    pub zones: &'a mut Vec<Zone>,
    pub smokes: &'a mut Vec<Smoke>,
}
impl Brimstone {
    pub fn busy(&self) -> bool {
        self.slots.iter().any(|r| r.remaining > 0)
    }
    pub fn cancel(&mut self) {
        self.equipped = None;
        self.marks.clear();
    }
    pub fn tick(&mut self, eligible: bool) {
        for (r, d) in self.slots.iter_mut().zip(DEFINITIONS) {
            r.tick(d);
        }
        if !eligible {
            self.cancel();
        }
    }
    pub fn activate(&mut self, slot: usize, a: &Actor) -> bool {
        if slot >= 4 || !a.eligible || self.slots[slot].charges == 0 {
            return false;
        }
        if self.equipped != Some(slot) {
            self.marks.clear();
        }
        self.equipped = Some(slot);
        true
    }
    pub fn fire(&mut self, alt: bool, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Entity::Player(owner) = a.entity else {
            return false;
        };
        let Some(slot) = self.equipped else {
            return false;
        };
        if !a.eligible {
            return false;
        }
        if slot == 2 && alt {
            if self.marks.is_empty()
                || self.marks.len() > self.slots[2].charges as usize
                || c.smokes.len() + self.marks.len() > MAX_SMOKES
            {
                return false;
            }
            if self.marks.iter().any(|p| {
                !clear_box(
                    c.world,
                    Box3 {
                        min: *p,
                        max: add(*p, [0.1; 3]),
                    },
                )
            }) {
                return false;
            }
            for at in self.marks.drain(..) {
                self.slots[2].activate(DEFINITIONS[2]);
                c.smokes.push(Smoke {
                    owner,
                    center: add(at, [0., 1., 0.]),
                    radius: 3.,
                    ticks: self.settings.smoke_ticks,
                    color: [155, 140, 115, 255],
                });
                c.world.ability_sound(at, "minecraft:block.fire.extinguish");
            }
            self.equipped = None;
            return true;
        }
        if slot == 1 {
            if c.projectiles.len() >= MAX_PROJECTILES || !self.slots[1].activate(DEFINITIONS[1]) {
                return false;
            }
            c.projectiles.push(Projectile {
                owner,
                kind: ProjectileKind::Incendiary {
                    duration: self.settings.fire_ticks,
                },
                position: a.eye(),
                velocity: scale(a.direction, 0.8),
                ticks: 100,
                guided: false,
                victims: vec![],
            });
        } else {
            let range = if slot == 0 { 4. } else { self.settings.range };
            let Some(at) = teleport::target(c.world, a, range) else {
                return false;
            };
            if slot == 2 {
                if self.marks.len() >= self.slots[2].charges as usize {
                    return false;
                }
                self.marks.push(at);
                return true;
            }
            if c.zones.len() >= MAX_ZONES || !self.slots[slot].activate(DEFINITIONS[slot]) {
                return false;
            }
            let radius = if slot == 0 { 3. } else { 4. };
            c.zones.push(Zone {
                owner,
                kind: if slot == 0 {
                    ZoneKind::Stim
                } else {
                    ZoneKind::Orbital
                },
                bounds: Box3 {
                    min: add(at, [-radius, 0., -radius]),
                    max: add(at, [radius, if slot == 0 { 2. } else { 64. }, radius]),
                },
                ticks: if slot == 0 {
                    240
                } else {
                    self.settings.strike_ticks
                },
            });
            c.world.ability_sound(
                at,
                if slot == 0 {
                    "minecraft:block.beacon.activate"
                } else {
                    "minecraft:entity.lightning_bolt.thunder"
                },
            );
        }
        self.equipped = None;
        true
    }
}
