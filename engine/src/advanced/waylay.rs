use super::*;
use crate::abilities::Kind;
pub const NAMES: [&str; 4] = ["Saturate", "Lightspeed", "Refract", "Convergent Paths"];
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
        charges: 1,
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
#[derive(Clone, Copy)]
pub struct Settings {
    pub hinder_ticks: u32,
    pub beacon_ticks: u32,
    pub dash_speed: f64,
    pub return_speed: f64,
    pub beam_delay: u32,
    pub boost_ticks: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            hinder_ticks: 60,
            beacon_ticks: 160,
            dash_speed: 0.85,
            return_speed: 1.5,
            beam_delay: 20,
            boost_ticks: 80,
        }
    }
}
struct Beacon {
    object: u64,
    ticks: u32,
    stamp: u64,
    path: Vec<V3>,
    returning: bool,
}
struct Dash {
    legs: u8,
    ticks: u8,
    pause: u8,
    direction: V3,
}
struct Path {
    object: u64,
    ticks: u32,
    boosted: bool,
    victims: Vec<Entity>,
}
pub struct Waylay {
    pub core: Core,
    pub settings: Settings,
    beacon: Option<Beacon>,
    dash: Option<Dash>,
    clusters: Vec<u64>,
    paths: Vec<Path>,
    boost: u32,
}
impl Default for Waylay {
    fn default() -> Self {
        Self {
            core: Core::new(DEFINITIONS),
            settings: Settings::default(),
            beacon: None,
            dash: None,
            clusters: vec![],
            paths: vec![],
            boost: 0,
        }
    }
}
impl Waylay {
    fn occupied(&self) -> bool {
        self.dash.is_some() || self.beacon.as_ref().is_some_and(|b| b.returning)
    }
    fn clusters(&mut self, id: u128, c: &mut Context<'_, impl World>) {
        let mut impacts = vec![];
        for cluster in &self.clusters {
            let Some(o) = c
                .objects
                .iter_mut()
                .find(|o| o.id == *cluster && o.ticks > 0)
            else {
                continue;
            };
            o.velocity[1] -= 0.04;
            let end = add(o.position, o.velocity);
            if let Some(h) = block_collision(c.world, o.position, end, 0.1) {
                if !h.known {
                    o.health = 0.;
                    continue;
                }
                o.position = add(
                    add(o.position, scale(o.velocity, h.time)),
                    scale(h.normal, 0.03),
                );
                if h.normal[1] > 0.5 {
                    impacts.push(o.position);
                    o.health = 0.;
                } else {
                    o.velocity = scale(
                        sub(o.velocity, scale(h.normal, 2. * dot(o.velocity, h.normal))),
                        0.5,
                    );
                }
            } else {
                o.position = end;
            }
        }
        self.clusters.retain(|id| {
            c.objects
                .iter()
                .any(|o| o.id == *id && o.health > 0. && o.ticks > 0)
        });
        for at in impacts {
            for target in c.enemies(id, at, 3.5) {
                if !c.world.status_immune(target.entity) && c.visible(at, target.eye()) {
                    c.hinder(target.entity, self.settings.hinder_ticks.clamp(10, 120));
                }
            }
            c.world.burst_fx(at);
            c.world
                .ability_sound(at, "minecraft:block.amethyst_cluster.break");
        }
    }
    fn movement(&mut self, id: u128, a: &Actor, c: &mut Context<'_, impl World>) {
        if let Some(mut b) = self.beacon.take() {
            b.ticks = b.ticks.saturating_sub(1);
            let valid = b.ticks > 0
                && b.stamp == c.world.teleport_stamp(a.entity)
                && c.objects.iter().any(|o| o.id == b.object && o.ticks > 0);
            if valid {
                if b.returning {
                    self.beacon = Some(b);
                } else if let Some(last) = b.path.last() {
                    let dist = distance(*last, a.position);
                    if dist <= 4. && (dist <= 0.2 || b.path.len() < 160) {
                        if dist > 0.2 {
                            b.path.push(a.position);
                        }
                        self.beacon = Some(b);
                    }
                }
            }
        }
        if let Some(d) = &mut self.dash {
            if d.pause > 0 {
                d.pause -= 1;
                if d.pause == 0 {
                    d.direction = horizontal(a.direction);
                }
                return;
            }
            let wanted = scale(d.direction, self.settings.dash_speed.clamp(0.3, 1.2));
            let delta = sweep(c.world, a.bounds, wanted);
            let blocked_actor = c.body_blocked(a, delta);
            if blocked_actor
                || length(delta) < 0.05
                || !c.world.move_player(id, add(a.position, delta))
            {
                self.dash = None;
                return;
            }
            c.world.velocity(id, [0.; 3], false);
            d.ticks -= 1;
            if length(delta) + 0.02 < length(wanted) || d.ticks == 0 {
                d.legs -= 1;
                if d.legs == 0 {
                    self.dash = None;
                } else {
                    d.ticks = 6;
                    d.pause = 3;
                }
            }
            return;
        }
        let Some(mut b) = self.beacon.take() else {
            return;
        };
        if !b.returning {
            self.beacon = Some(b);
            return;
        }
        let Some(&target) = b.path.last() else { return };
        let requested = sub(target, a.position);
        let desired = scale(
            unit(requested),
            length(requested).min(self.settings.return_speed.clamp(0.5, 2.)),
        );
        let delta = sweep(c.world, a.bounds, desired);
        if length(sub(delta, desired)) > 0.02 || c.body_blocked(a, delta) {
            return;
        }
        if !c.world.move_player(id, add(a.position, delta)) {
            return;
        }
        c.world.velocity(id, [0.; 3], false);
        if length(requested) <= self.settings.return_speed.clamp(0.5, 2.) + 0.02 {
            b.path.pop();
        }
        if !b.path.is_empty() {
            self.beacon = Some(b);
        } else {
            c.world
                .ability_sound(a.position, "minecraft:entity.enderman.teleport");
        }
    }
}
impl Agent for Waylay {
    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }
    fn busy(&self) -> bool {
        self.core.busy()
            || self.occupied()
            || self.beacon.is_some()
            || !self.paths.is_empty()
            || !self.clusters.is_empty()
    }
    fn equipped(&self) -> bool {
        self.core.equipped.is_some() || self.occupied()
    }
    fn phased(&self) -> bool {
        self.beacon.as_ref().is_some_and(|b| b.returning)
    }
    fn hidden(&self) -> bool {
        self.phased()
    }
    fn protection(&self, a: &Actor) -> effects::Protection {
        effects::Protection {
            health: a.health,
            invulnerable: self.phased(),
            disarmed: self.occupied(),
            ..Default::default()
        }
    }
    fn owns(&self, id: u64) -> bool {
        self.clusters.contains(&id)
            || self.beacon.as_ref().is_some_and(|b| b.object == id)
            || self.paths.iter().any(|p| p.object == id)
    }
    fn stats(&self) -> [u32; 4] {
        [
            self.dash.as_ref().map_or(0, |d| u32::from(d.legs)),
            self.beacon.as_ref().map_or(0, |b| b.ticks),
            u32::from(self.phased()),
            self.boost,
        ]
    }
    fn cancel(&mut self) {
        self.core.cancel();
        self.dash = None;
        self.beacon = None;
    }
    fn leave(&mut self) {
        self.cancel();
        self.clusters.clear();
        self.paths.clear();
        self.boost = 0;
    }
    fn activate(&mut self, slot: usize, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Some(id) = owner(a).filter(|_| a.eligible && a.health > 0.) else {
            return false;
        };
        if slot >= 4 || self.occupied() {
            return false;
        }
        if slot == 0 {
            if self.core.slots[0].charges == 0 {
                return false;
            }
            let Some(object) = c.object(
                id,
                ObjectKind::LightCluster,
                a.eye(),
                1.,
                80,
                scale(unit(a.direction), 0.65),
            ) else {
                return false;
            };
            self.clusters.push(object);
            self.core.spend(0);
            self.core.cancel();
            c.world
                .ability_sound(a.eye(), "minecraft:entity.snowball.throw");
            return true;
        }
        if slot != 2 {
            return self.core.equip(slot);
        }
        if let Some(b) = &mut self.beacon {
            if b.stamp != c.world.teleport_stamp(a.entity)
                || b.path.last().is_none_or(|p| distance(*p, a.position) > 4.)
            {
                self.beacon = None;
                return false;
            }
            b.returning = true;
            self.core.cancel();
            c.world.protect(id, self.protection(a));
            return true;
        }
        if self.core.slots[2].charges == 0 || !a.ground {
            return false;
        }
        let top = add(a.position, [0., 0.15, 0.]);
        if !block_collision(c.world, top, add(top, [0., -0.4, 0.]), 0.)
            .is_some_and(|h| h.known && h.normal[1] > 0.5)
        {
            return false;
        }
        let ticks = self.settings.beacon_ticks.clamp(20, 240);
        let Some(object) = c.object(
            id,
            ObjectKind::LightBeacon,
            a.position,
            1.,
            ticks + 1,
            [0.; 3],
        ) else {
            return false;
        };
        self.beacon = Some(Beacon {
            object,
            ticks,
            stamp: c.world.teleport_stamp(a.entity),
            path: vec![a.position],
            returning: false,
        });
        self.core.spend(2);
        self.core.cancel();
        c.world
            .ability_sound(a.position, "minecraft:block.beacon.activate");
        true
    }
    fn fire(&mut self, alt: bool, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let (Some(id), Some(slot)) = (owner(a), self.core.equipped) else {
            return false;
        };
        if !a.eligible || self.occupied() || self.core.slots[slot].charges == 0 {
            return false;
        }
        if slot == 1 {
            self.dash = Some(Dash {
                legs: if alt { 1 } else { 2 },
                ticks: 6,
                pause: 0,
                direction: unit(a.direction),
            });
        } else if slot == 3 {
            let from = add(a.position, [0., 0.8, 0.]);
            let direction = horizontal(a.direction);
            let end = add(from, scale(direction, 20.));
            let t = c.terrain_limit(from, end).unwrap_or(1.);
            if t < 0.05 {
                return false;
            }
            let to = add(from, scale(sub(end, from), t));
            let ticks = self.settings.beam_delay.clamp(5, 60) + 30;
            let Some(object) = c.object(id, ObjectKind::LightPath, from, 1., ticks + 1, to) else {
                return false;
            };
            self.paths.push(Path {
                object,
                ticks,
                boosted: false,
                victims: vec![],
            });
        } else {
            return false;
        }
        self.core.spend(slot);
        self.core.cancel();
        c.world
            .ability_sound(a.position, "minecraft:block.amethyst_block.resonate");
        true
    }
    fn tick(&mut self, a: &Actor, c: &mut Context<'_, impl World>) {
        let Some(id) = owner(a) else { return };
        self.clusters(id, c);
        self.boost = self.boost.saturating_sub(1);
        self.movement(id, a, c);
        for path in &mut self.paths {
            path.ticks = path.ticks.saturating_sub(1);
            let Some(o) = c
                .objects
                .iter_mut()
                .find(|o| o.id == path.object && o.ticks > 0)
            else {
                path.ticks = 0;
                continue;
            };
            o.ticks = path.ticks + 1;
            let (from, to) = (o.position, o.velocity);
            if path.ticks > 30 || path.ticks == 0 {
                continue;
            }
            if !path.boosted {
                path.boosted = true;
                self.boost = self.settings.boost_ticks.clamp(20, 160);
                c.world.effect(a.entity, "minecraft:speed", self.boost, 2);
                c.world
                    .ability_sound(from, "minecraft:block.beacon.power_select");
            }
            let width = ((30 - path.ticks) as f64 / 5.).clamp(0.1, 3.);
            let direction = unit(sub(to, from));
            for target in c.enemies(id, from, distance(from, to) + 4.) {
                if path.victims.len() >= 32
                    || path.victims.contains(&target.entity)
                    || c.world.status_immune(target.entity)
                {
                    continue;
                }
                let offset = sub(target.eye(), from);
                let along = dot(offset, direction);
                let side = sub(offset, scale(direction, along));
                if along >= 0.
                    && along <= distance(from, to)
                    && side[1].abs() <= 3.
                    && length([side[0], 0., side[2]]) <= width + 0.3
                    && c.visible(from, target.eye())
                {
                    c.hinder(target.entity, self.settings.hinder_ticks.clamp(10, 120));
                    path.victims.push(target.entity);
                }
            }
        }
        self.paths.retain(|p| p.ticks > 0);
        self.core.slots[1].active = self
            .dash
            .as_ref()
            .map_or(0, |d| u32::from(d.ticks + d.pause));
        self.core.slots[2].active = self.beacon.as_ref().map_or(0, |b| b.ticks);
        self.core.slots[3].active = self
            .boost
            .max(self.paths.iter().map(|p| p.ticks).max().unwrap_or(0));
    }
}
