use super::*;
use crate::abilities::Kind;
pub const NAMES: [&str; 4] = ["Trapwire", "Cyber Cage", "Spycam", "Neural Theft"];
pub const DEFINITIONS: [Ability; 4] = [
    Ability {
        kind: Kind::Active,
        cooldown: 600,
        duration: 0,
        charges: 2,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 600,
        duration: 0,
        charges: 2,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 600,
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
struct Wire {
    id: u64,
    victim: Option<Entity>,
    delay: u32,
    anchor: V3,
}
struct Cage {
    at: V3,
    ticks: u32,
    inside: Vec<Entity>,
}
pub struct Cypher {
    pub core: Core,
    wires: Vec<Wire>,
    devices: Vec<u64>,
    cages: Vec<Cage>,
    cam: Option<u64>,
    view: Option<Camera>,
    dart: u32,
}
impl Default for Cypher {
    fn default() -> Self {
        Self {
            core: Core::new(DEFINITIONS),
            wires: vec![],
            devices: vec![],
            cages: vec![],
            cam: None,
            view: None,
            dart: 0,
        }
    }
}
impl Cypher {
    fn proxy(&self, a: &Actor) -> Actor {
        if let Some(v) = &self.view {
            Actor {
                position: v.remote.position,
                bounds: Box3 {
                    min: v.remote.position,
                    max: v.remote.position,
                },
                direction: a.direction,
                ..a.clone()
            }
        } else {
            a.clone()
        }
    }
}
impl Agent for Cypher {
    fn stats(&self) -> [u32; 4] {
        [
            self.wires.len() as u32,
            self.devices.len() as u32,
            u32::from(self.cam.is_some()),
            u32::from(self.view.is_some()),
        ]
    }
    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }
    fn camera(&self) -> Option<V3> {
        self.view.as_ref().map(|v| v.remote.position)
    }
    fn owns(&self, id: u64) -> bool {
        self.cam == Some(id) || self.wires.iter().any(|w| w.id == id) || self.devices.contains(&id)
    }
    fn cancel(&mut self) {
        self.core.cancel();
        self.view = None;
    }
    fn leave(&mut self) {
        self.cancel();
        self.cam = None;
        self.wires.clear();
        self.devices.clear();
    }
    fn activate(&mut self, slot: usize, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Some(id) = owner(a) else {
            return false;
        };
        if slot >= 4 {
            return false;
        }
        if slot == 3 {
            let index = c
                .corpses
                .iter()
                .enumerate()
                .filter(|(_, corpse)| {
                    corpse.enemy(c.world, id)
                        && distance(a.eye(), corpse.at) < 5.
                        && dot(a.direction, unit(sub(corpse.at, a.eye()))) > 0.8
                })
                .map(|(i, _)| i)
                .next();
            let Some(index) = index else {
                return false;
            };
            if !c.visible(a.eye(), c.corpses[index].at) || !self.core.spend(3) {
                return false;
            }
            c.corpses[index].used = true;
            c.pulse(id, a.position, 128., false, 40);
            c.world
                .ability_sound(a.position, "minecraft:block.beacon.power_select");
            return true;
        }
        if slot == 2
            && let Some(cam) = self.cam.and_then(|id| {
                c.objects
                    .iter()
                    .find(|o| o.id == id && o.health > 0.)
                    .cloned()
            })
        {
            if self.view.is_some() {
                self.view = None;
                return true;
            }
            self.view = Camera::new(c.world, a, add(cam.position, [0., 0.12, 0.]), cam.id);
            return self.view.is_some();
        }
        if slot == 1 {
            let p = self.proxy(a);
            let target = self
                .devices
                .iter()
                .filter_map(|id| c.objects.iter().find(|o| o.id == *id && o.health > 0.))
                .filter(|o| {
                    distance(p.eye(), o.position) < 32.
                        && dot(p.direction, unit(sub(o.position, p.eye()))) > 0.94
                })
                .min_by(|x, y| {
                    distance(p.eye(), x.position).total_cmp(&distance(p.eye(), y.position))
                })
                .cloned();
            if let Some(o) = target {
                if c.zones.len() >= MAX_ZONES || !c.visible(p.eye(), add(o.position, [0., 0.1, 0.]))
                {
                    return false;
                }
                c.objects.retain(|x| x.id != o.id);
                self.devices.retain(|x| *x != o.id);
                self.cages.push(Cage {
                    at: o.position,
                    ticks: 140,
                    inside: vec![],
                });
                c.zones.push(Zone {
                    owner: id,
                    kind: zones::ZoneKind::CyberCage,
                    bounds: Box3 {
                        min: add(o.position, [-3., 0., -3.]),
                        max: add(o.position, [3., 4., 3.]),
                    },
                    ticks: 140,
                });
                return true;
            }
            if self.view.is_some()
                || c.objects.len() >= MAX_OBJECTS
                || self.core.slots[1].charges == 0
            {
                return false;
            }
            let Some(device) = c.object(
                id,
                ObjectKind::CageDevice,
                a.eye(),
                1.,
                600,
                scale(a.direction, 0.6),
            ) else {
                return false;
            };
            self.core.spend(1);
            self.devices.push(device);
            return true;
        }
        if slot == 2 && self.cam.is_some() {
            self.cam = None;
        }
        self.view = None;
        self.core.equip(slot)
    }
    fn fire(&mut self, _alt: bool, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Some(id) = owner(a) else {
            return false;
        };
        if let Some(v) = &mut self.view {
            if self.dart > 0 || c.projectiles.len() >= MAX_PROJECTILES {
                return false;
            }
            v.remote.look(a.direction);
            c.projectiles.push(Projectile {
                owner: id,
                kind: ProjectileKind::ReconDart,
                position: add(v.remote.position, scale(v.remote.direction, 0.25)),
                velocity: scale(v.remote.direction, 1.5),
                ticks: 30,
                guided: false,
                victims: vec![],
            });
            self.dart = 40;
            return true;
        }
        let Some(slot) = self.core.equipped else {
            return false;
        };
        if c.objects.len() >= MAX_OBJECTS {
            return false;
        }
        let end = add(a.eye(), scale(a.direction, 6.));
        let Some(hit) = block_collision(c.world, a.eye(), end, 0.) else {
            return false;
        };
        if !hit.known || hit.time < 0.03 {
            return false;
        }
        let at = add(
            add(a.eye(), scale(sub(end, a.eye()), hit.time)),
            scale(hit.normal, 0.22),
        );
        if slot == 0 {
            let end = add(at, scale(hit.normal, 8.));
            let Some(other) = block_collision(c.world, at, end, 0.) else {
                return false;
            };
            if !other.known || other.time < 0.1 {
                return false;
            }
            let far = add(
                add(at, scale(sub(end, at), other.time)),
                scale(hit.normal, -0.08),
            );
            if self.core.slots[0].charges == 0 {
                return false;
            }
            let Some(wire) = c.object(id, ObjectKind::Trapwire, at, 1., 1200, far) else {
                return false;
            };
            self.core.spend(0);
            self.wires.push(Wire {
                id: wire,
                victim: None,
                delay: 0,
                anchor: at,
            });
        } else if slot == 2 {
            if self.core.slots[2].charges == 0 {
                return false;
            }
            let Some(cam) = c.object(id, ObjectKind::Spycam, at, 1., 1200, [0.; 3]) else {
                return false;
            };
            self.core.spend(2);
            self.cam = Some(cam);
        } else {
            return false;
        }
        self.core.cancel();
        c.world.ability_sound(at, "minecraft:block.tripwire.attach");
        true
    }
    fn recall(&mut self, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let target = c
            .objects
            .iter()
            .filter(|o| {
                self.owns(o.id)
                    && !matches!(o.kind, ObjectKind::CageDevice)
                    && distance(a.eye(), o.position) <= 3.
            })
            .find(|o| dot(a.direction, unit(sub(o.position, a.eye()))) > 0.8)
            .cloned();
        let Some(o) = target else {
            return false;
        };
        if !c.visible(a.eye(), o.position) {
            return false;
        }
        c.objects.retain(|x| x.id != o.id);
        let slot = if self.cam == Some(o.id) {
            self.cam = None;
            self.view = None;
            2
        } else {
            self.wires.retain(|w| w.id != o.id);
            0
        };
        self.core.slots[slot].remaining = self.core.slots[slot].remaining.clamp(1, 200);
        true
    }
    fn tick(&mut self, a: &Actor, c: &mut Context<'_, impl World>) {
        let Some(id) = owner(a) else {
            return;
        };
        self.dart = self.dart.saturating_sub(1);
        if let Some(v) = &mut self.view
            && !v.update(a, c, false)
        {
            self.view = None;
        }
        if !self
            .cam
            .is_some_and(|id| c.objects.iter().any(|o| o.id == id && o.health > 0.))
        {
            self.cam = None;
            self.view = None;
        }
        self.devices
            .retain(|id| c.objects.iter().any(|o| o.id == *id && o.health > 0.));
        for o in c
            .objects
            .iter_mut()
            .filter(|o| self.devices.contains(&o.id))
        {
            landed(o, c.world, 0.03);
        }
        self.wires
            .retain(|wire| c.objects.iter().any(|o| o.id == wire.id && o.health > 0.));
        for wire in &mut self.wires {
            let Some(o) = c.objects.iter().find(|o| o.id == wire.id).cloned() else {
                continue;
            };
            if let Some(victim) = wire.victim {
                if let Some(target) = c.actors.iter().find(|t| t.entity == victim && t.eligible) {
                    if distance(target.position, wire.anchor) > 5. {
                        wire.victim = None;
                        continue;
                    }
                    c.intel.reveal(c.world, id, victim, 8);
                    c.world.effect(victim, "minecraft:slowness", 3, 3);
                    if distance(target.position, wire.anchor) > 1. {
                        c.world.push_entity(
                            victim,
                            scale(unit(sub(wire.anchor, target.position)), 0.5),
                        );
                    }
                    wire.delay = wire.delay.saturating_sub(1);
                    if wire.delay == 0 {
                        c.concuss(victim, 40);
                        c.objects.retain(|x| x.id != wire.id);
                    }
                } else {
                    wire.victim = None;
                }
            } else {
                for t in c.enemies(id, o.position, 12.) {
                    let steps = (length(t.velocity) / 0.2).ceil().clamp(1., 64.) as u32;
                    let crossed = (0..=steps).any(|n| {
                        t.bounds
                            .translated(scale(t.velocity, -f64::from(n) / f64::from(steps)))
                            .expanded(0.1)
                            .segment(o.position, o.velocity)
                            .is_some()
                    });
                    if crossed {
                        wire.victim = Some(t.entity);
                        wire.anchor = t.position;
                        wire.delay = 40;
                        c.intel.reveal(c.world, id, t.entity, 8);
                        break;
                    }
                }
            }
        }
        for cage in &mut self.cages {
            cage.ticks = cage.ticks.saturating_sub(1);
            let mut inside = vec![];
            for t in c.enemies(id, cage.at, 5.) {
                if t.position[1] >= cage.at[1]
                    && t.position[1] <= cage.at[1] + 4.
                    && (t.position[0] - cage.at[0]).hypot(t.position[2] - cage.at[2]) < 3.
                {
                    c.world.effect(t.entity, "minecraft:slowness", 3, 1);
                    if !cage.inside.contains(&t.entity) {
                        c.world
                            .ability_sound(cage.at, "minecraft:block.tripwire.click_on");
                    }
                    inside.push(t.entity);
                }
            }
            cage.inside = inside;
        }
        self.cages.retain(|cage| cage.ticks > 0);
    }
}
