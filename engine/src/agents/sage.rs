use crate::{
    abilities::{Ability, Kind, Runtime},
    gameplay::{barrier::*, geometry::*, projectiles::*, *},
};
pub const NAMES: [&str; 4] = ["Barrier Orb", "Slow Orb", "Healing Orb", "Resurrection"];
pub const DEFINITIONS: [Ability; 4] = [
    Ability {
        kind: Kind::Active,
        cooldown: 800,
        duration: 600,
        charges: 1,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 500,
        duration: 120,
        charges: 2,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 900,
        duration: 100,
        charges: 1,
    },
    Ability {
        kind: Kind::Ultimate,
        cooldown: 2400,
        duration: 0,
        charges: 0,
    },
];
pub struct Sage {
    pub slots: [Runtime; 4],
    pub equipped: Option<usize>,
    pub rotated: bool,
    pub healing: Option<(Entity, u32)>,
}
impl Default for Sage {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|i| Runtime {
                charges: DEFINITIONS[i].charges,
                ..Default::default()
            }),
            equipped: None,
            rotated: false,
            healing: None,
        }
    }
}
impl Sage {
    pub fn busy(&self) -> bool {
        self.slots.iter().any(|s| s.remaining > 0) || self.healing.is_some()
    }
    pub fn cancel(&mut self) {
        self.equipped = None;
        self.healing = None;
    }
    pub fn activate(&mut self, slot: usize, a: &Actor) -> bool {
        if slot >= 3 || !a.eligible || self.slots[slot].charges == 0 {
            return false;
        }
        self.equipped = Some(slot);
        true
    }
    pub fn fire(
        &mut self,
        alt: bool,
        a: &Actor,
        w: &mut impl World,
        objects: &mut Vec<Projectile>,
        owned: &mut Vec<OwnedBlock>,
    ) -> bool {
        let Entity::Player(owner) = a.entity else {
            return false;
        };
        let Some(slot) = self.equipped else {
            return false;
        };
        if slot == 0 && alt {
            self.rotated = !self.rotated;
            return true;
        }
        match slot {
            0 => {
                if !place(w, a, self.rotated, owned) {
                    return false;
                }
            }
            1 => {
                if objects.len() >= MAX_PROJECTILES {
                    return false;
                }
                objects.push(Projectile {
                    owner,
                    kind: ProjectileKind::SlowOrb,
                    position: a.eye(),
                    velocity: scale(a.direction, 0.8),
                    ticks: 30,
                    guided: false,
                    victims: Vec::new(),
                });
            }
            2 => {
                let target = if alt {
                    Some(a.clone())
                } else {
                    aim(w, a, 15.0, true)
                };
                let Some(target) = target.filter(|t| t.health > 0.0 && t.health < t.max_health)
                else {
                    return false;
                };
                self.healing = Some((target.entity, 100));
            }
            _ => return false,
        }
        if !self.slots[slot].activate(DEFINITIONS[slot]) {
            return false;
        }
        self.equipped = None;
        true
    }
    pub fn tick(&mut self, a: &Actor, w: &mut impl World) {
        for (r, d) in self.slots.iter_mut().zip(DEFINITIONS) {
            r.tick(d);
        }
        if !a.eligible {
            self.cancel();
            return;
        }
        if let Some((target, left)) = self.healing {
            let Some(t) = w
                .actors()
                .into_iter()
                .find(|t| t.entity == target && t.eligible && t.health > 0.0)
            else {
                self.healing = None;
                return;
            };
            if distance(a.eye(), t.eye()) > 15.0
                || (a.entity != target && block_hit(w, a.eye(), t.eye(), 0.0).is_some())
            {
                self.healing = None;
                return;
            }
            if left % 10 == 0 {
                w.heal(target, if target == a.entity { 0.6 } else { 2.0 });
            }
            self.healing = (left > 1).then_some((target, left.saturating_sub(1)));
        }
    }
}
