use super::*;
use crate::abilities::Kind;
pub const NAMES: [&str; 4] = ["Fast Lane", "Relay Bolt", "High Gear", "Overdrive"];
pub const DEFINITIONS: [Ability; 4] = [
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
        kind: Kind::Active,
        cooldown: 1,
        duration: 0,
        charges: 1,
    },
    Ability {
        kind: Kind::Ultimate,
        cooldown: 2400,
        duration: 0,
        charges: 1,
    },
];
#[derive(Clone, Copy)]
pub struct Settings {
    pub energy_drain: f32,
    pub energy_regen: f32,
    pub sprint_speed: f64,
    pub slide_speed: f64,
    pub slide_ticks: u32,
    pub slide_recharge: u32,
    pub wall_ticks: u32,
    pub ultimate_ticks: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            energy_drain: 0.5,
            energy_regen: 0.5,
            sprint_speed: 0.42,
            slide_speed: 0.8,
            slide_ticks: 6,
            slide_recharge: 600,
            wall_ticks: 120,
            ultimate_ticks: 200,
        }
    }
}
struct Bolt {
    id: u64,
    bounces: u8,
}
struct Pulse {
    at: V3,
    delay: u32,
    life: u32,
}
pub struct Neon {
    pub core: Core,
    pub settings: Settings,
    pub energy: f32,
    pub running: bool,
    pub slide_charge: bool,
    pub slide: u32,
    pub ultimate: u32,
    pub kill_count: u32,
    slide_direction: V3,
    recharge: u32,
    regen_delay: u32,
    fire_delay: u32,
    walls: u32,
    bolts: Vec<Bolt>,
    pulses: Vec<Pulse>,
}
impl Default for Neon {
    fn default() -> Self {
        Self {
            core: Core::new(DEFINITIONS),
            settings: Settings::default(),
            energy: 100.,
            running: false,
            slide_charge: true,
            slide: 0,
            ultimate: 0,
            kill_count: 0,
            slide_direction: [0.; 3],
            recharge: 0,
            regen_delay: 0,
            fire_delay: 0,
            walls: 0,
            bolts: vec![],
            pulses: vec![],
        }
    }
}
impl Neon {
    fn walls(&mut self, id: u128, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let forward = horizontal(a.direction);
        let side = [-forward[2], 0., forward[0]];
        let mut zones = vec![];
        for n in [-1., 1.] {
            let mut prev = add(a.position, scale(side, n * 0.8));
            for step in 1..=8 {
                let p = add(
                    add(a.position, scale(side, n * 0.8)),
                    scale(forward, f64::from(step)),
                );
                if !c.visible(add(prev, [0., 1., 0.]), add(p, [0., 1., 0.])) {
                    break;
                }
                let top = add(p, [0., 1., 0.]);
                let down = add(p, [0., -2., 0.]);
                let Some(hit) = block_collision(c.world, top, down, 0.) else {
                    break;
                };
                if !hit.known || hit.normal[1] < 0.5 {
                    break;
                }
                let at = add(top, scale(sub(down, top), hit.time));
                zones.push(Zone {
                    owner: id,
                    kind: zones::ZoneKind::ElectricWall,
                    ticks: self.settings.wall_ticks.clamp(20, 240),
                    bounds: Box3 {
                        min: std::array::from_fn(|i| {
                            if i == 1 {
                                prev[1].min(at[1]) + 0.02
                            } else {
                                prev[i].min(at[i]) - 0.1
                            }
                        }),
                        max: std::array::from_fn(|i| {
                            if i == 1 {
                                prev[1].max(at[1]) + 3.
                            } else {
                                prev[i].max(at[i]) + 0.1
                            }
                        }),
                    },
                });
                prev = at;
            }
        }
        if zones.is_empty() || zones.len() + c.zones.len() > MAX_ZONES {
            return false;
        }
        self.walls = self.settings.wall_ticks.clamp(20, 240);
        c.zones.extend(zones);
        true
    }
    fn bolts(&mut self, id: u128, c: &mut Context<'_, impl World>) {
        let mut impacts = vec![];
        for bolt in &mut self.bolts {
            let Some(o) = c
                .objects
                .iter_mut()
                .find(|o| o.id == bolt.id && o.health > 0. && o.ticks > 0)
            else {
                continue;
            };
            o.velocity[1] -= 0.03;
            let end = add(o.position, o.velocity);
            if let Some(hit) = block_collision(c.world, o.position, end, 0.08) {
                if !hit.known {
                    o.health = 0.;
                    continue;
                }
                let at = add(o.position, scale(o.velocity, hit.time));
                impacts.push(add(at, scale(hit.normal, 0.15)));
                if bolt.bounces == 0 {
                    let v = o.velocity;
                    o.velocity = scale(sub(v, scale(hit.normal, 2. * dot(v, hit.normal))), 0.8);
                    o.position = add(at, scale(hit.normal, 0.12));
                    bolt.bounces = 1;
                } else {
                    o.health = 0.;
                }
            } else {
                o.position = end;
            }
        }
        self.bolts.retain(|b| {
            c.objects
                .iter()
                .any(|o| o.id == b.id && o.health > 0. && o.ticks > 0)
        });
        for at in impacts {
            let to = add(at, [0., -8., 0.]);
            let Some(hit) = block_collision(c.world, at, to, 0.) else {
                continue;
            };
            if !hit.known || hit.normal[1] < 0.5 || c.zones.len() >= MAX_ZONES {
                continue;
            }
            let floor = add(add(at, scale(sub(to, at), hit.time)), [0., 0.03, 0.]);
            self.pulses.push(Pulse {
                at: floor,
                delay: 12,
                life: 20,
            });
            c.zones.push(Zone {
                owner: id,
                kind: zones::ZoneKind::ElectricPulse,
                ticks: 32,
                bounds: Box3 {
                    min: add(floor, [-3., 0., -3.]),
                    max: add(floor, [3., 0.2, 3.]),
                },
            });
            c.world
                .ability_sound(floor, "minecraft:block.respawn_anchor.charge");
        }
    }
}
impl Agent for Neon {
    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }
    fn inactive_tick(&mut self) {
        self.leave();
        self.recharge = self.recharge.saturating_sub(1);
        if self.recharge == 0 {
            self.slide_charge = true;
        }
        self.energy = (self.energy + self.settings.energy_regen.clamp(0.1, 5.)).min(100.);
    }
    fn busy(&self) -> bool {
        self.core.busy()
            || self.energy < 100.
            || self.recharge > 0
            || self.running
            || self.slide > 0
            || self.ultimate > 0
    }
    fn equipped(&self) -> bool {
        self.core.equipped.is_some() || self.running || self.ultimate > 0 || self.slide > 0
    }
    fn protection(&self, a: &Actor) -> effects::Protection {
        effects::Protection {
            health: a.health,
            disarmed: self.running || self.slide > 0 || self.ultimate > 0,
            ..Default::default()
        }
    }
    fn owns(&self, id: u64) -> bool {
        self.bolts.iter().any(|b| b.id == id)
    }
    fn zone_active(&self, k: zones::ZoneKind) -> bool {
        match k {
            zones::ZoneKind::ElectricWall => self.walls > 0,
            zones::ZoneKind::ElectricPulse => !self.pulses.is_empty(),
            _ => false,
        }
    }
    fn stats(&self) -> [u32; 4] {
        [
            self.energy.clamp(0., 100.) as u32,
            u32::from(self.slide_charge),
            self.ultimate,
            self.kill_count % 2,
        ]
    }
    fn kill(&mut self) {
        self.kill_count = self.kill_count.wrapping_add(1);
        if self.kill_count.is_multiple_of(2) {
            self.slide_charge = true;
            self.recharge = 0;
        }
        if self.ultimate > 0 {
            self.ultimate = self.settings.ultimate_ticks.clamp(20, 400);
        }
    }
    fn cancel(&mut self) {
        self.core.cancel();
        self.running = false;
        self.slide = 0;
        self.ultimate = 0;
    }
    fn leave(&mut self) {
        self.cancel();
        self.bolts.clear();
        self.pulses.clear();
        self.walls = 0;
    }
    fn activate(&mut self, slot: usize, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Some(id) = owner(a) else {
            return false;
        };
        if !a.eligible || slot >= 4 {
            return false;
        }
        if slot == 2 {
            if self.energy <= 0. && !self.running {
                return false;
            }
            self.core.cancel();
            self.running = !self.running;
            return true;
        }
        if self.core.slots[slot].charges == 0 {
            return false;
        }
        if slot == 3 {
            if self.ultimate > 0 || !self.core.spend(3) {
                return false;
            }
            self.core.cancel();
            self.running = false;
            self.energy = 100.;
            self.ultimate = self.settings.ultimate_ticks.clamp(20, 400);
            c.world
                .ability_sound(a.position, "minecraft:entity.lightning_bolt.thunder");
            return true;
        }
        if slot == 1 {
            if c.objects.len() >= MAX_OBJECTS {
                return false;
            }
            let at = add(a.eye(), scale(a.direction, 0.4));
            if !clear_box(
                c.world,
                Box3 {
                    min: at.map(|n| n - 0.1),
                    max: at.map(|n| n + 0.1),
                },
            ) {
                return false;
            }
            let Some(object) = c.object(
                id,
                ObjectKind::RelayBolt,
                at,
                1.,
                80,
                scale(a.direction, 0.8),
            ) else {
                return false;
            };
            self.bolts.push(Bolt {
                id: object,
                bounces: 0,
            });
            self.core.spend(1);
            return true;
        }
        self.running = false;
        self.core.equip(slot)
    }
    fn fire(&mut self, alt: bool, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Some(id) = owner(a) else {
            return false;
        };
        if self.core.equipped == Some(0) {
            if self.core.slots[0].charges == 0 || !self.walls(id, a, c) {
                return false;
            }
            self.core.spend(0);
            self.core.cancel();
            c.world
                .ability_sound(a.position, "minecraft:block.beacon.activate");
            return true;
        }
        if alt
            && (self.running || self.ultimate > 0)
            && self.slide_charge
            && self.slide == 0
            && a.ground
        {
            self.slide_charge = false;
            self.recharge = self.settings.slide_recharge.clamp(100, 1200);
            self.slide_direction = if length(a.movement) > 0.05 {
                horizontal(a.movement)
            } else {
                horizontal(a.direction)
            };
            self.slide = self.settings.slide_ticks.clamp(1, 10);
            self.energy = (self.energy - 10.).max(0.);
            c.world
                .ability_sound(a.position, "minecraft:entity.player.splash.high_speed");
            return true;
        }
        if !alt && self.ultimate > 0 {
            self.core.firing = true;
            return true;
        }
        false
    }
    fn tick(&mut self, a: &Actor, c: &mut Context<'_, impl World>) {
        let Some(id) = owner(a) else {
            return;
        };
        self.walls = self.walls.saturating_sub(1);
        self.ultimate = self.ultimate.saturating_sub(1);
        self.fire_delay = self.fire_delay.saturating_sub(1);
        self.recharge = self.recharge.saturating_sub(1);
        if self.recharge == 0 {
            self.slide_charge = true;
        }
        self.regen_delay = self.regen_delay.saturating_sub(1);
        if self.running && self.ultimate == 0 && length(a.movement) > 0.05 {
            self.energy = (self.energy - self.settings.energy_drain.clamp(0.1, 5.)).max(0.);
            self.regen_delay = 20;
            if self.energy == 0. {
                self.running = false;
            }
        } else if !self.running && self.ultimate == 0 && self.regen_delay == 0 {
            self.energy = (self.energy + self.settings.energy_regen.clamp(0.1, 5.)).min(100.);
        }
        if self.slide > 0 {
            let delta = sweep(
                c.world,
                a.bounds,
                scale(
                    self.slide_direction,
                    self.settings.slide_speed.clamp(0.1, 1.2),
                ),
            );
            if length(delta) < 0.05 || !c.world.move_player(id, add(a.position, delta)) {
                self.slide = 0;
            } else {
                self.slide -= 1;
            }
            c.world.velocity(id, [0., a.velocity[1], 0.], false);
        } else if self.running || self.ultimate > 0 {
            c.world.effect(a.entity, "minecraft:speed", 3, 2);
            if length(a.movement) > 0.05 {
                let v = scale(
                    horizontal(a.movement),
                    self.settings.sprint_speed.clamp(0.1, 0.6),
                );
                let safe = sweep(c.world, a.bounds, [v[0], 0., v[2]]);
                c.world
                    .velocity(id, [safe[0], a.velocity[1], safe[2]], false);
            }
        }
        if self.ultimate > 0 && self.core.firing && self.fire_delay == 0 {
            let end = c.hitscan(id, a.eye(), add(a.eye(), scale(a.direction, 32.)), 4.);
            if c.beams.len() < recon::MAX_BEAMS {
                c.beams.push(Beam {
                    from: a.eye(),
                    to: end,
                    ticks: 3,
                });
            }
            c.world
                .ability_sound(a.eye(), "minecraft:block.amethyst_block.hit");
            self.fire_delay = 10;
        }
        if self.ultimate == 0 {
            self.core.firing = false;
        }
        self.core.slots[2].charges = u8::from(self.energy > 0.);
        self.core.slots[2].active = u32::from(self.running);
        self.core.slots[3].active = self.ultimate;
        self.bolts(id, c);
        for p in &mut self.pulses {
            if p.delay > 0 {
                p.delay -= 1;
                if p.delay == 0 {
                    for t in c.enemies(id, p.at, 3.5) {
                        if c.visible(add(p.at, [0., 0.3, 0.]), t.eye()) {
                            c.concuss(t.entity, 50);
                        }
                    }
                    c.world
                        .ability_sound(p.at, "minecraft:block.respawn_anchor.deplete");
                }
            } else {
                p.life = p.life.saturating_sub(1);
            }
        }
        self.pulses.retain(|p| p.life > 0);
    }
}
