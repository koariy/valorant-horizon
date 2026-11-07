use crate::{
    abilities::{Ability, Kind, Runtime},
    gameplay::{geometry::*, projectiles::*, smoke::Smoke, *},
};
use std::collections::HashMap;
pub const NAMES: [&str; 4] = ["Pick-Me-Up", "Meddle", "Ruse", "Not Dead Yet (unavailable)"];
pub const DEFINITIONS: [Ability; 4] = [
    Ability {
        kind: Kind::Active,
        cooldown: 600,
        duration: 160,
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
        cooldown: 600,
        duration: 0,
        charges: 2,
    },
    Ability {
        kind: Kind::Ultimate,
        cooldown: 2400,
        duration: 0,
        charges: 0,
    },
];
#[derive(Clone, Copy)]
pub struct Settings {
    pub smoke_ticks: u32,
    pub range: f64,
    pub reward_ticks: u32,
    pub buff_ticks: u32,
    pub absorption_amplifier: u8,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            smoke_ticks: 240,
            range: 32.,
            reward_ticks: 160,
            buff_ticks: 160,
            absorption_amplifier: 1,
        }
    }
}
pub struct Clove {
    pub slots: [Runtime; 4],
    pub equipped: Option<usize>,
    pub marks: Vec<V3>,
    pub settings: Settings,
    pub contributions: HashMap<Entity, u32>,
    pub reward: u32,
    pub buff: u32,
    pub death: Option<V3>,
    pub preview: Option<V3>,
    pub offset: [i8; 2],
    was_alive: bool,
}
impl Default for Clove {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|i| Runtime {
                charges: DEFINITIONS[i].charges,
                ..Default::default()
            }),
            equipped: None,
            marks: vec![],
            settings: Settings::default(),
            contributions: HashMap::new(),
            reward: 0,
            buff: 0,
            death: None,
            preview: None,
            offset: [0, 4],
            was_alive: false,
        }
    }
}
pub struct Context<'a, W: World> {
    pub world: &'a mut W,
    pub projectiles: &'a mut Vec<Projectile>,
    pub smokes: &'a mut Vec<Smoke>,
}
impl Clove {
    pub fn busy(&self) -> bool {
        self.buff > 0 || self.slots.iter().any(|s| s.remaining > 0)
    }
    pub fn cancel(&mut self) {
        self.equipped = None;
        self.preview = None;
        self.marks.clear();
    }
    pub fn leave(&mut self) {
        self.cancel();
        self.death = None;
        self.was_alive = false;
        self.reward = 0;
        self.contributions.clear();
    }
    pub fn damage(&mut self, target: Entity) {
        if self.contributions.len() < 32 || self.contributions.contains_key(&target) {
            self.contributions.insert(target, 200);
        }
    }
    pub fn fallen(&mut self, target: Entity, killed: bool) {
        let assisted = self.contributions.remove(&target).is_some();
        if killed || assisted {
            self.reward = self.settings.reward_ticks.clamp(1, 600);
        }
    }
    pub fn offline(&mut self) {
        self.leave();
        for (r, d) in self.slots.iter_mut().zip(DEFINITIONS) {
            r.tick(d);
        }
        self.buff = self.buff.saturating_sub(1);
        self.slots[0].active = self.buff;
    }
    pub fn tick(&mut self, a: &Actor, w: &mut impl World) {
        for (r, d) in self.slots.iter_mut().zip(DEFINITIONS) {
            r.tick(d);
        }
        self.reward = self.reward.saturating_sub(1);
        self.contributions
            .values_mut()
            .for_each(|n| *n = n.saturating_sub(1));
        self.contributions.retain(|_, n| *n > 0);
        if !a.eligible {
            if self.was_alive && a.health <= 0. {
                self.death = Some(a.position);
                self.offset = [0, 4];
            }
            if a.health > 0. {
                self.death = None;
            }
            if self.was_alive || a.health > 0. {
                self.cancel();
            }
            self.reward = 0;
            self.contributions.clear();
        }
        if a.eligible && self.death.take().is_some() {
            self.cancel();
        }
        self.was_alive = a.eligible;
        if self.equipped == Some(2) {
            self.preview = self.target(a, w);
        }

        if self.buff > 0 && a.eligible {
            w.effect(a.entity, "minecraft:speed", 2, 1);
            self.buff -= 1;
        } else {
            self.buff = 0;
        }
        self.slots[0].active = self.buff;
    }
    pub fn dead_allowed(&self, a: &Actor) -> bool {
        a.health <= 0. && self.death.is_some()
    }
    pub fn activate(&mut self, slot: usize, a: &Actor, w: &mut impl World) -> bool {
        if slot >= 3 || (!a.eligible && !(slot == 2 && self.dead_allowed(a))) {
            return false;
        }
        if slot == 0 {
            if self.reward == 0 || !self.slots[0].activate(DEFINITIONS[0]) {
                return false;
            }

            self.reward = 0;
            self.buff = self.settings.buff_ticks.clamp(1, 600);
            // Use vanilla's duration/priority rules: never subtract another potion's shield.
            w.effect(
                a.entity,
                "minecraft:absorption",
                self.buff,
                self.settings.absorption_amplifier.min(4),
            );
            w.ability_sound(a.position, "minecraft:block.amethyst_block.chime");
            return true;
        }
        if self.slots[slot].charges == 0 {
            return false;
        }
        self.cancel();
        self.equipped = Some(slot);
        true
    }
    pub fn nudge(&mut self, dx: i8, dz: i8, a: &Actor) -> bool {
        if !self.dead_allowed(a)
            || self.equipped != Some(2)
            || !(-1..=1).contains(&dx)
            || !(-1..=1).contains(&dz)
        {
            return false;
        }
        self.offset[0] = (self.offset[0] + dx).clamp(-20, 20);
        self.offset[1] = (self.offset[1] + dz).clamp(-20, 20);
        true
    }
    pub fn target(&self, a: &Actor, w: &impl World) -> Option<V3> {
        if a.eligible {
            return teleport::target(w, a, self.settings.range.clamp(2., 48.));
        }
        let origin = self.death?;
        let top = add(
            origin,
            [f64::from(self.offset[0]), 4., f64::from(self.offset[1])],
        );
        let end = add(top, [0., -10., 0.]);
        let t = block_hit(w, top, end, 0.)?;
        let at = add(add(top, scale(sub(end, top), t)), [0., 0.02, 0.]);
        (distance(origin, at) <= self.settings.range.clamp(2., 48.)
            && clear_box(
                w,
                Box3 {
                    min: at,
                    max: add(at, [0.1; 3]),
                },
            ))
        .then_some(at)
    }
    pub fn fire(&mut self, alt: bool, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Entity::Player(owner) = a.entity else {
            return false;
        };
        let Some(slot) = self.equipped else {
            return false;
        };
        if !a.eligible && !(slot == 2 && self.dead_allowed(a)) {
            return false;
        }
        if slot == 1 {
            if c.projectiles.len() >= MAX_PROJECTILES || !self.slots[1].activate(DEFINITIONS[1]) {
                return false;
            }
            c.projectiles.push(Projectile {
                owner,
                kind: ProjectileKind::Meddle,
                position: a.eye(),
                velocity: scale(a.direction, 0.65),
                ticks: 24,
                guided: false,
                victims: vec![],
            });
            self.equipped = None;
            return true;
        }
        if slot != 2 {
            return false;
        }
        if !alt {
            let Some(at) = self.target(a, c.world) else {
                return false;
            };
            if self.marks.len() >= usize::from(self.slots[2].charges) {
                return false;
            }
            self.marks.push(at);
            return true;
        }
        if self.marks.is_empty()
            || self.marks.len() > usize::from(self.slots[2].charges)
            || c.smokes.len() + self.marks.len() > MAX_SMOKES
        {
            return false;
        }
        let origin = if a.eligible {
            a.position
        } else {
            let Some(at) = self.death else {
                return false;
            };
            at
        };
        if self.marks.iter().any(|at| {
            distance(origin, *at) > self.settings.range.clamp(2., 48.) + 0.05
                || !clear_box(
                    c.world,
                    Box3 {
                        min: *at,
                        max: add(*at, [0.1; 3]),
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
                ticks: self.settings.smoke_ticks.clamp(1, 600),
                color: [160, 115, 195, 255],
            });
            c.world
                .ability_sound(at, "minecraft:block.amethyst_block.resonate");
        }
        self.equipped = None;
        true
    }
}
