use crate::{
    abilities::{Ability, Kind, Runtime},
    gameplay::{geometry::*, projectiles::*, toxin::*, *},
};
pub const NAMES: [&str; 4] = ["Snake Bite", "Poison Cloud", "Toxic Screen", "Viper's Pit"];
pub const DEFINITIONS: [Ability; 4] = [
    Ability {
        kind: Kind::Active,
        cooldown: 700,
        duration: 0,
        charges: 1,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 1200,
        duration: 0,
        charges: 1,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 1200,
        duration: 0,
        charges: 1,
    },
    Ability {
        kind: Kind::Ultimate,
        cooldown: 2400,
        duration: 600,
        charges: 1,
    },
];
#[derive(Clone, Copy)]
pub struct Settings {
    pub drain: f32,
    pub regeneration: f32,
    pub minimum: f32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            drain: 0.2,
            regeneration: 0.12,
            minimum: 20.,
        }
    }
}
pub struct Viper {
    pub slots: [Runtime; 4],
    pub equipped: Option<usize>,
    pub fuel: f32,
    pub cloud: Option<V3>,
    pub screen: Option<Box3>,
    pub cloud_on: bool,
    pub screen_on: bool,
    pub pit: Option<V3>,
    pub pit_ticks: u32,
    pub outside: u32,
    pub placed_ticks: u32,
    pub recovery: u32,
    pub toggle: u32,
    pub settings: Settings,
}
impl Default for Viper {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|i| Runtime {
                charges: DEFINITIONS[i].charges,
                ..Default::default()
            }),
            equipped: None,
            fuel: 100.,
            cloud: None,
            screen: None,
            cloud_on: false,
            screen_on: false,
            pit: None,
            pit_ticks: 0,
            outside: 0,
            placed_ticks: 0,
            recovery: 0,
            toggle: 0,
            settings: Settings::default(),
        }
    }
}
impl Viper {
    pub fn busy(&self) -> bool {
        self.slots.iter().any(|s| s.remaining > 0)
            || self.cloud.is_some()
            || self.screen.is_some()
            || self.pit.is_some()
            || self.fuel < 100.
    }
    pub fn suppress(&mut self) {
        if self.cloud_on || self.screen_on {
            self.recovery = 20;
        }
        self.equipped = None;
        self.cloud_on = false;
        self.screen_on = false;
    }
    pub fn cancel(&mut self) {
        self.suppress();
        self.pit = None;
        self.pit_ticks = 0;
    }
    pub fn activate(&mut self, slot: usize, a: &Actor) -> bool {
        if !a.eligible || slot >= 4 {
            return false;
        }
        if (slot == 1 && self.cloud.is_some()) || (slot == 2 && self.screen.is_some()) {
            let on = if slot == 1 {
                &mut self.cloud_on
            } else {
                &mut self.screen_on
            };
            if self.toggle > 0 || (!*on && self.fuel < self.settings.minimum) {
                return false;
            }
            *on = !*on;
            self.toggle = 10;
            self.recovery = 20;
            return true;
        }
        if self.slots[slot].charges == 0 {
            return false;
        }
        self.equipped = Some(slot);
        true
    }
    pub fn fire(
        &mut self,
        a: &Actor,
        w: &mut impl World,
        projectiles: &mut Vec<Projectile>,
    ) -> bool {
        let Entity::Player(owner) = a.entity else {
            return false;
        };
        let Some(slot) = self.equipped else {
            return false;
        };
        if !a.eligible {
            return false;
        }
        if slot <= 1 {
            if projectiles.len() >= MAX_PROJECTILES || !self.slots[slot].activate(DEFINITIONS[slot])
            {
                return false;
            }
            projectiles.push(Projectile {
                owner,
                kind: if slot == 0 {
                    ProjectileKind::SnakeBite
                } else {
                    ProjectileKind::PoisonEmitter
                },
                position: a.eye(),
                velocity: scale(a.direction, 0.65),
                ticks: 80,
                guided: false,
                victims: vec![],
            });
        } else if slot == 2 {
            let dir = horizontal(a.direction);
            let along_x = dir[0].abs() > dir[2].abs();
            let step = if along_x {
                [dir[0].signum(), 0., 0.]
            } else {
                [0., 0., dir[2].signum()]
            };
            let start = add(a.position, scale(step, 1.));
            let mut end = start;
            for n in 1..=20 {
                let p = add(start, scale(step, n as f64));
                if w.boxes(p.map(|v| v.floor() as i32)).is_none() {
                    break;
                }
                end = p;
            }
            if distance(start, end) < 2. || !self.slots[2].activate(DEFINITIONS[2]) {
                return false;
            }
            self.screen = Some(Box3 {
                min: std::array::from_fn(|i| start[i].min(end[i]) - if i == 1 { 0. } else { 0.2 }),
                max: std::array::from_fn(|i| start[i].max(end[i]) + if i == 1 { 4. } else { 0.2 }),
            });
            self.placed_ticks = 1200;
        } else {
            if !self.slots[3].activate(DEFINITIONS[3]) {
                return false;
            }
            self.pit = Some(add(a.position, [0., 2., 0.]));
            self.pit_ticks = 600;
            self.outside = 0;
        }
        w.ability_sound(a.position, "minecraft:block.fire.extinguish");
        self.equipped = None;
        true
    }
    pub fn tick(&mut self, a: &Actor, capacity: usize) -> Vec<Field> {
        for (r, d) in self.slots.iter_mut().zip(DEFINITIONS) {
            r.tick(d);
        }
        self.toggle = self.toggle.saturating_sub(1);
        self.placed_ticks = self.placed_ticks.saturating_sub(1);
        if self.placed_ticks == 0 {
            self.cloud = None;
            self.screen = None;
            self.cloud_on = false;
            self.screen_on = false;
        }
        if !a.eligible {
            self.cancel();
        }
        self.pit_ticks = self.pit_ticks.saturating_sub(1);
        if let Some(p) = self.pit {
            if distance(a.eye(), p) > 7. {
                self.outside += 1;
            } else {
                self.outside = 0;
            }
            if self.outside >= 100 || self.pit_ticks == 0 {
                self.pit = None;
            }
        }
        let needed = usize::from(self.cloud_on)
            + usize::from(self.screen_on)
            + usize::from(self.pit.is_some());
        if needed > capacity {
            self.cancel();
        }
        if self.cloud_on || self.screen_on {
            self.fuel = (self.fuel - self.settings.drain).max(0.);
            self.recovery = 20;
            if self.fuel == 0. {
                self.cloud_on = false;
                self.screen_on = false;
            }
        } else {
            self.recovery = self.recovery.saturating_sub(1);
            if self.recovery == 0 {
                self.fuel = (self.fuel + self.settings.regeneration).min(100.);
            }
        }
        self.slots[1].active = u32::from(self.cloud_on);
        self.slots[2].active = u32::from(self.screen_on);
        self.slots[3].active = self.pit_ticks * u32::from(self.pit.is_some());
        let Entity::Player(owner) = a.entity else {
            return vec![];
        };
        let mut fields = vec![];
        if self.cloud_on
            && let Some(p) = self.cloud
        {
            fields.push(Field::ball(owner, p, 3.));
        }
        if self.screen_on
            && let Some(bounds) = self.screen
        {
            fields.push(Field {
                owner,
                bounds,
                sphere: false,
            });
        }
        if let Some(p) = self.pit {
            fields.push(Field::ball(owner, p, 7.));
        }
        fields
    }
}
