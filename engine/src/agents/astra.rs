use crate::{
    abilities::{Ability, Kind, Runtime},
    gameplay::{geometry::*, remote_view::RemoteView, smoke::Smoke, zones::*, *},
};
use serde::{Deserialize, Serialize};
pub const MAX_STARS: usize = 16;
pub const NAMES: [&str; 4] = [
    "Gravity Well",
    "Nova Pulse",
    "Nebula",
    "Astral Form / Cosmic Divide",
];
pub const DEFINITIONS: [Ability; 4] = [
    Ability {
        kind: Kind::Active,
        cooldown: 400,
        duration: 0,
        charges: 1,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 400,
        duration: 0,
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
        cooldown: 2400,
        duration: 400,
        charges: 1,
    },
];
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Star {
    pub owner: u128,
    pub position: V3,
    pub ticks: u32,
}
pub struct Astra {
    pub slots: [Runtime; 4],
    pub stars: Vec<Star>,
    pub available: u8,
    pub returns: Vec<u32>,
    pub view: Option<RemoteView>,
    pub anchor: Option<V3>,
    pub first: Option<V3>,
    pub divide_aim: bool,
    pub lifetime: u32,
}
impl Default for Astra {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|i| Runtime {
                charges: DEFINITIONS[i].charges,
                ..Default::default()
            }),
            stars: vec![],
            available: 5,
            returns: vec![],
            view: None,
            anchor: None,
            first: None,
            divide_aim: false,
            lifetime: 0,
        }
    }
}
pub struct Context<'a, W: World> {
    pub world: &'a mut W,
    pub smokes: &'a mut Vec<Smoke>,
    pub zones: &'a mut Vec<Zone>,
    pub divides: &'a mut Vec<crate::gameplay::divide::Divide>,
}
impl Astra {
    pub fn busy(&self) -> bool {
        self.view.is_some() || self.available < 5 || self.slots.iter().any(|s| s.remaining > 0)
    }
    pub fn cancel(&mut self) {
        self.view = None;
        self.anchor = None;
        self.first = None;
        self.divide_aim = false;
    }
    fn proxy(&self, a: &Actor) -> Actor {
        if let Some(v) = &self.view {
            Actor {
                position: v.position,
                bounds: Box3 {
                    min: v.position,
                    max: v.position,
                },
                direction: v.direction,
                ..a.clone()
            }
        } else {
            a.clone()
        }
    }
    fn ground(&self, a: &Actor, w: &impl World) -> Option<V3> {
        let proxy = self.proxy(a);
        let end = add(proxy.eye(), scale(proxy.direction, 64.));
        let t = block_hit(w, proxy.eye(), end, 0.)?;
        let p = add(
            proxy.eye(),
            scale(sub(end, proxy.eye()), (t - 0.002).max(0.)),
        );
        (distance(a.position, p) <= 64.).then_some(p)
    }
    fn aimed(&self, a: &Actor) -> Option<usize> {
        let eye = self.proxy(a);
        self.stars
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                let d = sub(s.position, eye.eye());
                let n = length(d);
                (n <= 64. && dot(unit(d), eye.direction) > 0.96).then_some((n, i))
            })
            .min_by(|x, y| x.0.total_cmp(&y.0))
            .map(|(_, i)| i)
    }
    pub fn activate(&mut self, slot: usize, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        if !a.eligible || slot >= 4 {
            return false;
        }
        if slot == 3 {
            if self.view.is_some() {
                self.cancel();
                return true;
            }
            if !a.ground {
                return false;
            }
            let at = add(a.eye(), [0., 12., 0.]);
            let Some(mut view) = RemoteView::new(c.world, at, 32.) else {
                return false;
            };
            view.look(a.direction);
            self.view = Some(view);
            self.anchor = Some(a.position);
            self.lifetime = 600;
            return true;
        }
        let Some(i) = self.aimed(a) else {
            return false;
        };
        if self.slots[slot].charges == 0
            || (slot == 2 && c.smokes.len() >= MAX_SMOKES)
            || (slot < 2 && c.zones.len() >= MAX_ZONES)
        {
            return false;
        }
        if !self.slots[slot].activate(DEFINITIONS[slot]) {
            return false;
        }
        let star = self.stars.remove(i);
        self.returns.push(800);
        if slot == 2 {
            c.smokes.push(Smoke {
                owner: star.owner,
                center: add(star.position, [0., 1., 0.]),
                radius: 3.,
                ticks: 280,
                color: [105, 55, 155, 255],
            });
        } else {
            c.zones.push(Zone {
                owner: star.owner,
                kind: if slot == 0 {
                    ZoneKind::Gravity
                } else {
                    ZoneKind::Nova
                },
                bounds: Box3 {
                    min: add(star.position, [-3., -0.2, -3.]),
                    max: add(star.position, [3., 3., 3.]),
                },
                ticks: if slot == 0 { 60 } else { 20 },
            });
        }
        c.world
            .ability_sound(star.position, "minecraft:block.amethyst_block.chime");
        true
    }
    pub fn fire(&mut self, alt: bool, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        if self.view.is_none() || !a.eligible {
            return false;
        }
        if alt {
            if self.slots[3].charges == 0 {
                return false;
            }
            self.divide_aim = !self.divide_aim;
            self.first = None;
            return true;
        }
        let Some(at) = self.ground(a, c.world) else {
            return false;
        };
        if self.divide_aim {
            if let Some(first) = self.first {
                if c.divides.len() >= 2 {
                    return false;
                }
                let Some(d) = crate::gameplay::divide::Divide::new(first, at) else {
                    return false;
                };
                if !self.slots[3].activate(DEFINITIONS[3]) {
                    return false;
                }
                c.divides.push(d);
                self.cancel();
            } else {
                self.first = Some(at);
            }
            return true;
        }
        if self.available == 0 || self.stars.len() >= 5 {
            return false;
        }
        let Entity::Player(owner) = a.entity else {
            return false;
        };
        self.available -= 1;
        self.stars.push(Star {
            owner,
            position: at,
            ticks: 1200,
        });
        true
    }
    pub fn dissipate(&mut self, a: &Actor, smokes: &mut Vec<Smoke>) -> bool {
        let Some(i) = self.aimed(a) else {
            return false;
        };
        if smokes.len() >= MAX_SMOKES {
            return false;
        }
        let star = self.stars.remove(i);
        self.returns.push(80);
        smokes.push(Smoke {
            owner: star.owner,
            center: add(star.position, [0., 1., 0.]),
            radius: 3.,
            ticks: 20,
            color: [105, 55, 155, 255],
        });
        true
    }
    pub fn tick(&mut self, a: &Actor, w: &mut impl World) {
        for (r, d) in self.slots.iter_mut().zip(DEFINITIONS) {
            r.tick(d);
        }
        for r in &mut self.returns {
            *r = r.saturating_sub(1);
        }
        let restored = self.returns.iter().filter(|t| **t == 0).count();
        self.available = (self.available + restored as u8).min(5);
        self.returns.retain(|t| *t > 0);
        for s in &mut self.stars {
            s.ticks = s.ticks.saturating_sub(1);
            if s.ticks == 0 {
                self.available = (self.available + 1).min(5);
            }
        }
        self.stars.retain(|s| s.ticks > 0);
        if !a.eligible
            || self.anchor.is_some_and(|p| {
                block_hit(w, add(p, [0., 0.05, 0.]), add(p, [0., -0.3, 0.]), 0.).is_none()
            })
        {
            self.cancel();
        }
        if self.view.is_some() {
            self.lifetime = self.lifetime.saturating_sub(1);
            if self.lifetime == 0 {
                self.cancel();
            }
        }
        if let Some(v) = &mut self.view {
            v.look(a.direction);
            let vertical = a.direction[1] * dot(horizontal(a.direction), a.movement);
            v.step(
                w,
                add(
                    a.movement,
                    [0., vertical + if a.jump { 0.3 } else { 0. }, 0.],
                ),
            );
        }
        if let (Entity::Player(id), Some(at)) = (a.entity, self.anchor) {
            w.move_player(id, at);
            w.velocity(id, [0.; 3], false);
            w.protect(
                id,
                effects::Protection {
                    health: a.health,
                    disarmed: true,
                    ..Default::default()
                },
            );
        } else if let Entity::Player(id) = a.entity {
            w.protect(
                id,
                effects::Protection {
                    health: a.health,
                    ..Default::default()
                },
            );
        }
    }
}
