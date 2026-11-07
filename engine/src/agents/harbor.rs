use crate::{
    abilities::{Ability, Kind, Runtime},
    gameplay::{geometry::*, projectiles::*, shields::*, zones::*, *},
};
pub const NAMES: [&str; 4] = ["Storm Surge", "High Tide", "Cove", "Reckoning"];
pub const DEFINITIONS: [Ability; 4] = [
    Ability {
        kind: Kind::Active,
        cooldown: 600,
        duration: 0,
        charges: 1,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 700,
        duration: 240,
        charges: 1,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 700,
        duration: 240,
        charges: 1,
    },
    Ability {
        kind: Kind::Ultimate,
        cooldown: 2400,
        duration: 0,
        charges: 1,
    },
];
/// Minecraft balance settings, configured when constructing the kit.
#[derive(Clone, Copy)]
pub struct Settings {
    pub cove_radius: f64,
    pub cove_health: f32,
    pub cove_ticks: u32,
    pub marker_range: f64,
    pub tide_segments: u8,
    pub tide_ticks: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            cove_radius: 3.,
            cove_health: 30.,
            cove_ticks: 240,
            marker_range: 48.,
            tide_segments: 20,
            tide_ticks: 240,
        }
    }
}
pub struct Harbor {
    pub settings: Settings,
    pub slots: [Runtime; 4],
    pub equipped: Option<usize>,
    pub distance: f64,
    pub preview: Option<V3>,
    pub tide: Option<(V3, V3, u8)>,
    pub guiding: bool,
}
impl Default for Harbor {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            slots: std::array::from_fn(|i| Runtime {
                charges: DEFINITIONS[i].charges,
                ..Default::default()
            }),
            equipped: None,
            distance: 16.,
            preview: None,
            tide: None,
            guiding: false,
        }
    }
}
pub struct Context<'a, W: World> {
    pub world: &'a mut W,
    pub projectiles: &'a mut Vec<Projectile>,
    pub shields: &'a mut Vec<Shield>,
}
impl Harbor {
    pub fn busy(&self) -> bool {
        self.slots.iter().any(|s| s.remaining > 0) || self.tide.is_some()
    }
    pub fn cancel(&mut self) {
        self.equipped = None;
        self.preview = None;
        self.tide = None;
        self.guiding = false;
    }
    pub fn activate(&mut self, slot: usize, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        if !a.eligible || slot >= 4 {
            return false;
        }
        if slot == 2 {
            if let Some(s) = c.shields.iter_mut().find(|s| {
                s.owner
                    == match a.entity {
                        Entity::Player(id) => id,
                        _ => 0,
                    }
                    && s.ticks > 0
            }) {
                if s.spent {
                    return false;
                }
                s.spent = true;
                s.active = true;
                c.world
                    .ability_sound(s.center, "minecraft:block.conduit.activate");
                return true;
            }
            if self.equipped == Some(2) {
                let Some(at) = self.preview else {
                    return false;
                };
                if c.shields.len() >= MAX_SHIELDS || !self.slots[2].activate(DEFINITIONS[2]) {
                    return false;
                }
                let Entity::Player(owner) = a.entity else {
                    return false;
                };
                c.shields.push(Shield {
                    owner,
                    center: add(at, [0., 1., 0.]),
                    radius: self.settings.cove_radius.clamp(1., 6.),
                    ticks: self.settings.cove_ticks.clamp(1, 1200),
                    health: self.settings.cove_health.clamp(1., 1000.),
                    active: false,
                    spent: false,
                });
                self.equipped = None;
                self.preview = None;
                return true;
            }
        }
        if self.slots[slot].charges == 0 {
            return false;
        }
        self.equipped = Some(slot);
        true
    }
    pub fn fire(&mut self, alt: bool, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Entity::Player(owner) = a.entity else {
            return false;
        };
        let Some(slot) = self.equipped else {
            if alt && self.tide.is_some() {
                self.tide = None;
                self.guiding = false;
                return true;
            }
            return self.tide.is_some();
        };
        if slot == 2 {
            self.distance = (self.distance + if alt { -0.5 } else { 0.5 })
                .clamp(2., self.settings.marker_range.clamp(2., 64.));
            self.preview = teleport::target(c.world, a, self.distance);
            return true;
        }
        if slot == 1 {
            if !a.ground || !self.slots[1].activate(DEFINITIONS[1]) {
                return false;
            }
            self.tide = Some((a.position, horizontal(a.direction), 0));
            self.guiding = true;
        } else {
            if c.projectiles.len() >= MAX_PROJECTILES
                || !self.slots[slot].activate(DEFINITIONS[slot])
            {
                return false;
            }
            c.projectiles.push(Projectile {
                owner,
                kind: if slot == 0 {
                    ProjectileKind::StormSurge
                } else {
                    ProjectileKind::Reckoning
                },
                position: if slot == 0 { a.eye() } else { a.position },
                velocity: scale(
                    if slot == 0 {
                        a.direction
                    } else {
                        horizontal(a.direction)
                    },
                    if slot == 0 { 0.6 } else { 0.8 },
                ),
                ticks: if slot == 0 { 30 } else { 40 },
                guided: false,
                victims: vec![],
            });
        }
        c.world
            .ability_sound(a.position, "minecraft:item.trident.riptide_1");
        self.equipped = None;
        true
    }
    pub fn tick(&mut self, a: &Actor, w: &mut impl World, zones: &mut Vec<Zone>) {
        for (r, d) in self.slots.iter_mut().zip(DEFINITIONS) {
            r.tick(d);
        }
        if !a.eligible {
            self.cancel();
            return;
        }
        if self.equipped == Some(2) {
            self.preview = teleport::target(w, a, self.distance);
        }
        if let Some((at, dir, count)) = self.tide {
            if zones.len() >= MAX_ZONES || count >= self.settings.tide_segments.clamp(1, 20) {
                self.tide = None;
                return;
            }
            let dir = if self.guiding {
                horizontal(a.direction)
            } else {
                dir
            };
            let next = add(at, dir);
            // Follow loaded ground; never alter blocks or create water flows.
            let top = add(next, [0., 1.5, 0.]);
            let down = add(next, [0., -2., 0.]);
            let Some(t) = block_hit(w, top, down, 0.) else {
                self.tide = None;
                return;
            };
            let feet = add(add(top, scale(sub(down, top), t)), [0., 0.02, 0.]);
            if w.boxes(feet.map(|v| v.floor() as i32)).is_none() {
                self.tide = None;
                return;
            }
            let Entity::Player(owner) = a.entity else {
                return;
            };
            zones.push(Zone {
                owner,
                kind: ZoneKind::WaterWall,
                bounds: Box3 {
                    min: add(feet, [-0.6, 0., -0.6]),
                    max: add(feet, [0.6, 3., 0.6]),
                },
                ticks: self.settings.tide_ticks.clamp(1, 1200),
            });
            self.tide = Some((feet, dir, count + 1));
        }
    }
}
