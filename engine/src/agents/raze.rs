use crate::{
    abilities::{Ability, Kind, Runtime},
    gameplay::{explosions::*, geometry::*, objects::*, projectiles::*, *},
};
pub const NAMES: [&str; 4] = ["Boom Bot", "Blast Pack", "Paint Shells", "Showstopper"];
pub const DEFINITIONS: [Ability; 4] = [
    Ability {
        kind: Kind::Active,
        cooldown: 700,
        duration: 200,
        charges: 1,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 300,
        duration: 100,
        charges: 2,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 700,
        duration: 0,
        charges: 1,
    },
    Ability {
        kind: Kind::Ultimate,
        cooldown: 2400,
        duration: 200,
        charges: 1,
    },
];
pub struct Raze {
    pub slots: [Runtime; 4],
    pub equipped: Option<usize>,
    pub rocket: bool,
    pub kills: u8,
}
impl Default for Raze {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|i| Runtime {
                charges: DEFINITIONS[i].charges,
                ..Default::default()
            }),
            equipped: None,
            rocket: false,
            kills: 0,
        }
    }
}
pub struct Context<'a, W: World> {
    pub world: &'a mut W,
    pub projectiles: &'a mut Vec<Projectile>,
    pub objects: &'a mut Vec<Object>,
    pub bursts: &'a mut Vec<Explosion>,
    pub next: &'a mut u64,
}
impl Raze {
    pub fn busy(&self) -> bool {
        self.slots.iter().any(|s| s.remaining > 0) || self.rocket
    }
    pub fn cancel(&mut self) {
        self.equipped = None;
        self.rocket = false;
        self.slots[3].active = 0;
    }
    pub fn kill(&mut self) {
        self.kills += 1;
        if self.kills >= 2 {
            self.kills = 0;
            self.slots[2].charges = 1;
            self.slots[2].remaining = 0;
        }
    }
    pub fn activate(&mut self, slot: usize, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Entity::Player(owner) = a.entity else {
            return false;
        };
        if slot >= 4 || !a.eligible {
            return false;
        }
        if slot == 1 {
            if let Some(i) = c
                .objects
                .iter()
                .position(|o| o.owner == owner && matches!(o.kind, ObjectKind::BlastPack))
            {
                let at = c.objects.remove(i).position;
                c.bursts.push(pack(owner, at));
                return true;
            }
            if let Some(i) = c
                .projectiles
                .iter()
                .position(|p| p.owner == owner && matches!(p.kind, ProjectileKind::BlastPack))
            {
                let at = c.projectiles.remove(i).position;
                c.bursts.push(pack(owner, at));
                return true;
            }
            if c.projectiles.len() >= MAX_PROJECTILES || !self.slots[1].activate(DEFINITIONS[1]) {
                return false;
            }
            c.projectiles.push(Projectile {
                owner,
                kind: ProjectileKind::BlastPack,
                position: a.eye(),
                velocity: add(scale(a.direction, 0.45), [0.0, 0.08, 0.0]),
                ticks: 80,
                guided: false,
                victims: vec![],
            });
            return true;
        }
        if self.slots[slot].charges == 0 {
            return false;
        }
        if slot == 3 {
            if self.rocket || !self.slots[3].activate(DEFINITIONS[3]) {
                return false;
            }
            self.rocket = true;
            self.equipped = Some(3);
        } else {
            self.equipped = Some(slot);
        }
        true
    }
    pub fn fire(&mut self, alt: bool, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Entity::Player(owner) = a.entity else {
            return false;
        };
        let Some(slot) = self.equipped else {
            return false;
        };
        if slot == 0 {
            if c.objects.len() >= MAX_OBJECTS {
                return false;
            }
            let position = add(a.position, scale(horizontal(a.direction), 0.8));
            let temp = Object {
                last_hit: None,
                id: 0,
                owner,
                kind: ObjectKind::BoomBot,
                position,
                health: 6.0,
                ticks: 200,
                velocity: scale(horizontal(a.direction), 0.18),
                target: None,
            };
            if !a.ground
                || !clear_box(c.world, temp.bounds())
                || !self.slots[0].activate(DEFINITIONS[0])
            {
                return false;
            }
            *c.next = c.next.saturating_add(1);
            c.objects.push(Object {
                last_hit: None,
                id: *c.next,
                ..temp
            });
        } else {
            if c.projectiles.len() >= MAX_PROJECTILES {
                return false;
            }
            let kind = if slot == 3 {
                if !self.rocket {
                    return false;
                }
                self.rocket = false;
                ProjectileKind::Rocket
            } else {
                if !self.slots[2].activate(DEFINITIONS[2]) {
                    return false;
                }
                ProjectileKind::Grenade { cluster: false }
            };
            c.projectiles.push(Projectile {
                owner,
                kind,
                position: a.eye(),
                velocity: scale(
                    a.direction,
                    if slot == 3 {
                        1.8
                    } else if alt {
                        0.35
                    } else {
                        0.75
                    },
                ),
                ticks: if slot == 3 { 40 } else { 24 },
                guided: false,
                victims: vec![],
            });
        }
        self.equipped = None;
        true
    }
    pub fn tick(&mut self, a: &Actor) {
        for (r, d) in self.slots.iter_mut().zip(DEFINITIONS) {
            r.tick(d);
        }
        if !a.eligible || self.slots[3].active == 0 {
            self.rocket = false;
            if self.equipped == Some(3) {
                self.equipped = None;
            }
        }
    }
}
pub fn pack(owner: u128, position: V3) -> Explosion {
    Explosion {
        owner,
        position,
        radius: 3.0,
        damage: 4.0,
        push: 1.8,
        self_damage: false,
    }
}
