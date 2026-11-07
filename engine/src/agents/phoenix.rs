use crate::{
    abilities::{Ability, Kind, Runtime},
    gameplay::{geometry::*, projectiles::*, zones::*, *},
};
pub const NAMES: [&str; 4] = ["Blaze", "Hot Hands", "Curveball", "Run It Back"];
pub const DEFINITIONS: [Ability; 4] = [
    Ability {
        kind: Kind::Active,
        cooldown: 400,
        duration: 160,
        charges: 1,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 600,
        duration: 120,
        charges: 1,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 400,
        duration: 0,
        charges: 2,
    },
    Ability {
        kind: Kind::Ultimate,
        cooldown: 1800,
        duration: 200,
        charges: 1,
    },
];
pub struct Phoenix {
    pub slots: [Runtime; 4],
    pub equipped: Option<usize>,
    pub anchor: Option<V3>,
    pub wall: Option<(V3, u8)>,
    pub guiding: bool,
    pub wall_direction: V3,
}
impl Default for Phoenix {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|i| Runtime {
                charges: DEFINITIONS[i].charges,
                ..Runtime::default()
            }),
            equipped: None,
            anchor: None,
            wall: None,
            guiding: false,
            wall_direction: [0.0, 0.0, 1.0],
        }
    }
}
impl Phoenix {
    pub fn busy(&self) -> bool {
        self.slots.iter().any(|s| s.remaining > 0) || self.anchor.is_some() || self.wall.is_some()
    }
    pub fn cancel(&mut self) {
        self.equipped = None;
        self.guiding = false;
        self.wall = None;
    }
    pub fn activate(&mut self, slot: usize, a: &Actor) -> bool {
        if slot >= 4 || !a.eligible {
            return false;
        }
        if slot == 3 {
            if self.anchor.is_some() || !self.slots[3].activate(DEFINITIONS[3]) {
                return false;
            }
            self.anchor = Some(a.position);
        } else {
            if self.slots[slot].charges == 0 {
                return false;
            }
            self.equipped = Some(slot);
        }
        true
    }
    pub fn fire(&mut self, alt: bool, a: &Actor, objects: &mut Vec<Projectile>) -> bool {
        let Entity::Player(owner) = a.entity else {
            return false;
        };
        let Some(slot) = self.equipped else {
            return false;
        };
        if objects.len() >= MAX_PROJECTILES || !self.slots[slot].activate(DEFINITIONS[slot]) {
            return false;
        }
        self.equipped = None;
        match slot {
            0 => {
                self.wall = Some((add(a.position, [0.0, 0.25, 0.0]), 10));
                self.guiding = true;
                self.wall_direction = unit([a.direction[0], 0.0, a.direction[2]]);
            }
            1 | 2 => objects.push(Projectile {
                owner,
                kind: if slot == 1 {
                    ProjectileKind::HotHands
                } else {
                    ProjectileKind::Curveball { right: alt }
                },
                position: a.eye(),
                velocity: scale(a.direction, if slot == 1 && alt { 0.35 } else { 0.8 }),
                ticks: if slot == 1 { 24 } else { 12 },
                guided: false,
                victims: Vec::new(),
            }),
            _ => {}
        }
        true
    }
    pub fn tick(&mut self, a: &Actor, w: &mut impl World, zones: &mut Vec<Zone>) {
        let Entity::Player(owner) = a.entity else {
            return;
        };
        for (r, d) in self.slots.iter_mut().zip(DEFINITIONS) {
            r.tick(d);
        }
        if let Some(anchor) = self.anchor
            && (self.slots[3].active == 0 || w.return_requested(owner))
        {
            // A blocked anchor is refused, never phase into a changed Minecraft wall.
            let _ = crate::gameplay::teleport::safe(w, a, anchor);
            w.heal(a.entity, a.max_health);
            self.anchor = None;
            self.slots[3].active = 0;
        }
        if !a.eligible {
            self.cancel();
            self.anchor = None;
            return;
        }
        if let Some((tip, left)) = self.wall
            && zones.len() < MAX_ZONES
        {
            if self.guiding {
                self.wall_direction = unit([a.direction[0], 0.0, a.direction[2]]);
            }
            let next = add(tip, scale(self.wall_direction, 1.1));
            if crate::gameplay::geometry::block_hit(w, tip, next, 0.05).is_none() {
                zones.push(Zone {
                    owner,
                    kind: ZoneKind::FireWall,
                    bounds: Box3 {
                        min: [
                            tip[0].min(next[0]) - 0.15,
                            tip[1],
                            tip[2].min(next[2]) - 0.15,
                        ],
                        max: [
                            tip[0].max(next[0]) + 0.15,
                            tip[1] + 2.5,
                            tip[2].max(next[2]) + 0.15,
                        ],
                    },
                    ticks: 160,
                });
                self.wall = (left > 1).then_some((next, left.saturating_sub(1)));
            } else {
                self.wall = None;
            }
        }
        w.protect(
            owner,
            crate::gameplay::effects::Protection {
                health: a.health,
                returning: self.anchor.is_some(),
                ..Default::default()
            },
        );
    }
}
