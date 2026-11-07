//! Jett: rules adapted to 20 Hz Minecraft. Numerical values are mod tuning.
use crate::{
    abilities::{Ability, Kind, Runtime},
    gameplay::{
        geometry::*,
        projectiles::{Projectile, ProjectileKind},
        *,
    },
};
pub const NAMES: [&str; 4] = ["Cloudburst", "Updraft", "Tailwind", "Blade Storm"];
pub const DEFINITIONS: [Ability; 4] = [
    Ability {
        kind: Kind::Active,
        cooldown: 400,
        duration: 50,
        charges: 2,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 500,
        duration: 0,
        charges: 1,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 600,
        duration: 150,
        charges: 1,
    },
    Ability {
        kind: Kind::Ultimate,
        cooldown: 1800,
        duration: 600,
        charges: 1,
    },
];
pub struct Jett {
    pub slots: [Runtime; 4],
    pub knives: u8,
    pub armed: u32,
    pub dash: u32,
    pub dash_direction: V3,
    pub guiding: bool,
    pub kills: u8,
    pub fire_delay: u32,
}
impl Default for Jett {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|i| Runtime {
                charges: DEFINITIONS[i].charges,
                ..Runtime::default()
            }),
            knives: 0,
            armed: 0,
            dash: 0,
            dash_direction: [0.0; 3],
            guiding: false,
            kills: 0,
            fire_delay: 0,
        }
    }
}
impl Jett {
    pub fn busy(&self) -> bool {
        self.armed > 0
            || self.dash > 0
            || self.slots.iter().any(|r| r.active > 0 || r.remaining > 0)
    }
    pub fn cancel(&mut self) {
        self.armed = 0;
        self.dash = 0;
        self.knives = 0;
        self.slots[3].active = 0;
        self.guiding = false;
    }
    pub fn kill(&mut self) {
        self.kills += 1;
        if self.kills >= 2 {
            self.kills = 0;
            self.slots[2].charges = 1;
            self.slots[2].remaining = 0;
        }
    }
    pub fn activate(
        &mut self,
        slot: usize,
        a: &Actor,
        w: &mut impl World,
        objects: &mut Vec<Projectile>,
    ) -> bool {
        let Entity::Player(owner) = a.entity else {
            return false;
        };
        if slot >= 4 || !a.eligible || self.dash > 0 {
            return false;
        }
        if slot == 2 && self.armed > 0 {
            self.armed = 0;
            self.dash = 4;
            let movement = unit([a.movement[0], 0.0, a.movement[2]]);
            self.dash_direction = if length(movement) > 0.1 {
                movement
            } else {
                horizontal(a.direction)
            };
            return true;
        }
        if slot == 1 {
            if self.slots[1].charges == 0 {
                return false;
            }
            // Refuse head obstruction before consuming a charge. Subsequent physics uses Minecraft collision.
            if length(sweep(w, a.bounds, [0.0, 0.25, 0.0])) < 0.24 {
                return false;
            }
            if !w.velocity(owner, [a.velocity[0], 1.05, a.velocity[2]], true) {
                return false;
            }
        }
        if slot == 0 && objects.len() >= MAX_PROJECTILES {
            return false;
        }
        if !self.slots[slot].activate(DEFINITIONS[slot]) {
            return false;
        }
        match slot {
            0 => {
                for projectile in objects
                    .iter_mut()
                    .filter(|p| p.owner == owner && matches!(p.kind, ProjectileKind::Cloudburst))
                {
                    projectile.guided = false;
                }
                self.guiding = true;
                objects.push(Projectile {
                    owner,
                    kind: ProjectileKind::Cloudburst,
                    position: a.eye(),
                    velocity: scale(a.direction, 0.9),
                    ticks: 35,
                    guided: true,
                    victims: Vec::new(),
                });
            }
            2 => self.armed = 150,
            3 => self.knives = 5,
            _ => {}
        }
        true
    }
    pub fn fire(&mut self, alternate: bool, a: &Actor, objects: &mut Vec<Projectile>) -> bool {
        let Entity::Player(owner) = a.entity else {
            return false;
        };
        if self.knives == 0
            || self.slots[3].active == 0
            || self.fire_delay > 0
            || objects.len() + 5 > MAX_PROJECTILES
        {
            return false;
        }
        let n = if alternate { self.knives } else { 1 };
        self.knives -= n;
        self.fire_delay = 4;
        for i in 0..n {
            let side = if alternate {
                (f64::from(i) - (f64::from(n) - 1.0) / 2.0) * 0.07
            } else {
                0.0
            };
            let direction = unit(add(
                a.direction,
                [a.direction[2] * side, 0.0, -a.direction[0] * side],
            ));
            objects.push(Projectile {
                owner,
                kind: ProjectileKind::Knife { alternate },
                position: a.eye(),
                velocity: scale(direction, 3.0),
                ticks: 30,
                guided: false,
                victims: Vec::new(),
            });
        }
        true
    }
    pub fn tick(&mut self, a: &Actor, w: &mut impl World) {
        let Entity::Player(owner) = a.entity else {
            return;
        };
        for (r, d) in self.slots.iter_mut().zip(DEFINITIONS) {
            r.tick(d);
        }
        self.armed = self.armed.saturating_sub(1);
        self.fire_delay = self.fire_delay.saturating_sub(1);
        if self.slots[3].active == 0 {
            self.knives = 0;
        }
        if !a.eligible {
            self.cancel();
            return;
        }
        if self.dash > 0 {
            let safe = sweep(w, a.bounds, scale(self.dash_direction, 2.4));
            if length(safe) < 0.05 || !w.move_player(owner, add(a.position, safe)) {
                self.dash = 0;
            } else {
                self.dash -= 1;
            }
        } else if a.jump && !a.ground && a.velocity[1] < -0.08 {
            w.velocity(owner, [a.velocity[0], -0.08, a.velocity[2]], true);
        }
        self.slots[2].active = self.armed.max(self.dash);
    }
}
