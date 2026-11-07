use crate::{
    abilities::{Ability, Kind, Runtime},
    gameplay::{effects::Protection, geometry::*, objects::*, *},
};
pub const NAMES: [&str; 4] = ["Leer", "Devour", "Dismiss", "Empress"];
pub const DEFINITIONS: [Ability; 4] = [
    Ability {
        kind: Kind::Active,
        cooldown: 600,
        duration: 60,
        charges: 2,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 600,
        duration: 60,
        charges: 2,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 600,
        duration: 40,
        charges: 2,
    },
    Ability {
        kind: Kind::Ultimate,
        cooldown: 2400,
        duration: 600,
        charges: 1,
    },
];
#[derive(Clone, Copy)]
pub struct Soul {
    pub position: V3,
    pub ticks: u32,
}
pub struct Reyna {
    pub slots: [Runtime; 4],
    pub equipped: bool,
    pub souls: Vec<Soul>,
    pub healing: Option<Soul>,
    pub phase: u32,
    pub frenzy: u32,
    pub overheal: f32,
    pub decay: u32,
}
impl Default for Reyna {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|i| Runtime {
                charges: DEFINITIONS[i].charges,
                ..Default::default()
            }),
            equipped: false,
            souls: vec![],
            healing: None,
            phase: 0,
            frenzy: 0,
            overheal: 0.,
            decay: 0,
        }
    }
}
impl Reyna {
    pub fn busy(&self) -> bool {
        self.phase > 0
            || self.frenzy > 0
            || self.slots.iter().any(|s| s.remaining > 0)
            || self.overheal > 0.
    }
    pub fn cancel(&mut self) {
        self.equipped = false;
        self.healing = None;
        self.phase = 0;
        self.frenzy = 0;
        self.slots[2].active = 0;
        self.slots[3].active = 0;
    }
    pub fn kill(&mut self, position: V3) {
        if self.souls.len() < 16 {
            self.souls.push(Soul {
                position,
                ticks: 60,
            });
        }
        if self.frenzy > 0 {
            self.frenzy = 600;
            self.healing = Some(Soul {
                position,
                ticks: 60,
            });
        }
    }
    pub fn activate(&mut self, slot: usize, a: &Actor, w: &impl World) -> bool {
        if !a.eligible || slot >= 4 || self.phase > 0 {
            return false;
        }
        if slot == 0 {
            if self.slots[0].charges == 0 {
                return false;
            }
            self.equipped = true;
            return true;
        }
        if slot == 3 {
            if self.frenzy > 0 || !self.slots[3].activate(DEFINITIONS[3]) {
                return false;
            }
            self.frenzy = 600;
            return true;
        }
        if self.healing.is_some() && slot == 1 {
            return false;
        }
        let index = self
            .souls
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                distance(a.eye(), s.position) <= 25.
                    && block_hit(w, a.eye(), s.position, 0.).is_none()
            })
            .min_by(|(_, x), (_, y)| {
                distance(a.eye(), x.position).total_cmp(&distance(a.eye(), y.position))
            })
            .map(|(i, _)| i);
        let Some(index) = index else {
            return false;
        };
        // Devour and Dismiss use the same pool, never two independent refills.
        if self.frenzy == 0 && !self.slots[1].activate(DEFINITIONS[1]) {
            return false;
        }
        self.slots[2].charges = self.slots[1].charges;
        self.slots[2].remaining = self.slots[1].remaining;
        let soul = self.souls.remove(index);
        if slot == 1 {
            self.healing = Some(Soul { ticks: 60, ..soul });
        } else {
            self.healing = None;
            self.phase = 40;
        }
        self.equipped = false;
        true
    }
    pub fn fire(
        &mut self,
        a: &Actor,
        w: &impl World,
        objects: &mut Vec<Object>,
        next: &mut u64,
    ) -> bool {
        let Entity::Player(owner) = a.entity else {
            return false;
        };
        if !self.equipped || self.phase > 0 || objects.len() >= MAX_OBJECTS {
            return false;
        }
        let end = add(a.eye(), scale(a.direction, 5.));
        let t = block_hit(w, a.eye(), end, 0.15).unwrap_or(1.);
        let at = add(a.eye(), scale(sub(end, a.eye()), (t - 0.04).max(0.)));
        if distance(at, a.eye()) < 0.5 || !self.slots[0].activate(DEFINITIONS[0]) {
            return false;
        }
        *next = next.saturating_add(1);
        objects.push(Object {
            id: *next,
            owner,
            kind: ObjectKind::Leer,
            position: at,
            health: 6.,
            ticks: 60,
            velocity: [0.; 3],
            target: None,
        });
        self.equipped = false;
        true
    }
    pub fn tick(&mut self, a: &Actor, w: &mut impl World) {
        let Entity::Player(owner) = a.entity else {
            return;
        };
        for i in [0, 1, 3] {
            self.slots[i].tick(DEFINITIONS[i]);
        }
        self.slots[2].charges = self.slots[1].charges;
        self.slots[2].remaining = self.slots[1].remaining;
        self.souls
            .iter_mut()
            .for_each(|s| s.ticks = s.ticks.saturating_sub(1));
        self.souls.retain(|s| s.ticks > 0);
        if !a.eligible {
            self.cancel();
        }
        self.phase = self.phase.saturating_sub(1);
        self.frenzy = self.frenzy.saturating_sub(1);
        self.slots[2].active = self.phase;
        self.slots[3].active = self.frenzy;
        w.protect(
            owner,
            Protection {
                health: a.health,
                invulnerable: self.phase > 0,
                disarmed: self.phase > 0,
                ..Default::default()
            },
        );
        if self.phase > 0 {
            w.effect(a.entity, "minecraft:speed", 2, 1);
            if self.frenzy > 0 {
                w.effect(a.entity, "minecraft:invisibility", 2, 0);
            }
        }
        if self.frenzy > 0 {
            w.effect(a.entity, "minecraft:haste", 2, 1);
            w.effect(a.entity, "minecraft:speed", 2, 0);
        }
        if let Some(mut s) = self.healing {
            if !a.eligible
                || distance(a.eye(), s.position) > 25.
                || block_hit(w, a.eye(), s.position, 0.).is_some()
            {
                self.healing = None;
            } else {
                if s.ticks % 5 == 0 {
                    if a.health < a.max_health {
                        w.heal(a.entity, 2.);
                    } else if self.overheal < 10. {
                        let gain = w.absorb(owner, (10. - self.overheal).min(2.));
                        self.overheal += gain.max(0.);
                    }
                }
                self.decay = 200;
                s.ticks = s.ticks.saturating_sub(1);
                self.healing = (s.ticks > 0).then_some(s);
            }
        }
        self.decay = self.decay.saturating_sub(1);
        if self.healing.is_none() && self.decay == 0 && self.overheal > 0. {
            let amount = self.overheal.min(0.1);
            w.absorb(owner, -amount);
            self.overheal = (self.overheal - amount).max(0.);
        }
    }
}
