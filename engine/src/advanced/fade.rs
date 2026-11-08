use super::*;
use crate::abilities::Kind;
pub const NAMES: [&str; 4] = ["Prowler", "Seize", "Haunt", "Nightfall"];
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
    pub drop_after: u32,
    pub haunt_ticks: u32,
    pub trail_ticks: u32,
    pub scan_range: f64,
    pub prowler_ticks: u32,
    pub prowler_speed: f64,
    pub seize_ticks: u32,
    pub wave_range: f64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            drop_after: 20,
            haunt_ticks: 100,
            trail_ticks: 200,
            scan_range: 24.,
            prowler_ticks: 120,
            prowler_speed: 0.3,
            seize_ticks: 80,
            wave_range: 48.,
        }
    }
}
struct Orb {
    id: u64,
    slot: usize,
    age: u32,
    falling: bool,
}
struct Tether {
    teleport: u64,
    target: Entity,
    safe: V3,
}
struct Catch {
    at: V3,
    ticks: u32,
    targets: Vec<Tether>,
}
struct Trail {
    target: Entity,
    origin: V3,
    ticks: u32,
    points: Vec<V3>,
}
struct Wave {
    origin: V3,
    direction: V3,
    front: f64,
    victims: Vec<Entity>,
}
pub struct Fade {
    pub core: Core,
    pub settings: Settings,
    orbs: Vec<Orb>,
    eyes: Vec<u64>,
    prowlers: Vec<u64>,
    catches: Vec<Catch>,
    tracks: Vec<Trail>,
    wave: Option<Wave>,
    clock: u32,
}
impl Default for Fade {
    fn default() -> Self {
        Self {
            core: Core::new(DEFINITIONS),
            settings: Settings::default(),
            orbs: vec![],
            eyes: vec![],
            prowlers: vec![],
            catches: vec![],
            tracks: vec![],
            wave: None,
            clock: 0,
        }
    }
}
impl Fade {
    fn mark(&mut self, target: &Actor, origin: V3) {
        let ticks = self.settings.trail_ticks.clamp(1, 240);
        if let Some(t) = self.tracks.iter_mut().find(|t| t.target == target.entity) {
            t.ticks = ticks;
        } else if self.tracks.len() < 16 {
            self.tracks.push(Trail {
                target: target.entity,
                origin,
                ticks,
                points: vec![target.position],
            });
        }
    }
    fn orbs(&mut self, id: u128, c: &mut Context<'_, impl World>) {
        let mut impacts = vec![];
        for orb in &mut self.orbs {
            let Some(o) = c
                .objects
                .iter_mut()
                .find(|o| o.id == orb.id && o.health > 0. && o.ticks > 0)
            else {
                continue;
            };
            orb.age += 1;
            orb.falling |= orb.age >= self.settings.drop_after.clamp(1, 40);
            if orb.falling {
                o.velocity[0] *= 0.8;
                o.velocity[2] *= 0.8;
            }
            o.velocity[1] = (o.velocity[1] - if orb.falling { 0.12 } else { 0.025 }).max(-0.9);
            let end = add(o.position, o.velocity);
            if let Some(hit) = block_collision(c.world, o.position, end, 0.12) {
                if !hit.known {
                    o.health = 0.;
                    continue;
                }
                o.position = add(
                    add(o.position, scale(o.velocity, (hit.time - 0.01).max(0.))),
                    scale(hit.normal, 0.02),
                );
                if hit.normal[1] > 0.5 {
                    impacts.push((o.id, orb.slot, o.position));
                } else {
                    orb.falling = true;
                    for (i, normal) in hit.normal.iter().enumerate() {
                        if normal.abs() > 0.5 {
                            o.velocity[i] = 0.;
                        }
                    }
                }
            } else {
                o.position = end;
            }
        }
        for (object, slot, at) in impacts {
            if slot == 2 {
                if let Some(o) = c.objects.iter_mut().find(|o| o.id == object) {
                    o.kind = ObjectKind::Haunt;
                    o.health = 8.;
                    o.ticks = self.settings.haunt_ticks.clamp(1, 200);
                    o.velocity = [0.; 3];
                    self.eyes.push(object);
                }
            } else {
                c.objects.retain(|o| o.id != object);
                if c.zones.len() < MAX_ZONES {
                    let targets = c
                        .enemies(id, at, 3.5)
                        .into_iter()
                        .filter_map(|a| {
                            c.visible(add(at, [0., 0.3, 0.]), add(a.position, [0., 0.5, 0.]))
                                .then(|| Tether {
                                    target: a.entity,
                                    safe: a.position,
                                    teleport: c.world.teleport_stamp(a.entity),
                                })
                        })
                        .collect();
                    let ticks = self.settings.seize_ticks.clamp(1, 160);
                    self.catches.push(Catch { at, ticks, targets });
                    c.zones.push(Zone {
                        owner: id,
                        kind: zones::ZoneKind::Seize,
                        ticks,
                        bounds: Box3 {
                            min: add(at, [-3.5, -0.2, -3.5]),
                            max: add(at, [3.5, 4., 3.5]),
                        },
                    });
                }
            }
            c.world
                .ability_sound(at, "minecraft:entity.warden.tendril_clicks");
            self.orbs.retain(|o| o.id != object);
        }
        self.orbs.retain(|orb| {
            c.objects
                .iter()
                .any(|o| o.id == orb.id && o.health > 0. && o.ticks > 0)
        });
    }
    fn eyes(&mut self, id: u128, c: &mut Context<'_, impl World>) {
        self.eyes.retain(|eye| {
            c.objects
                .iter()
                .any(|o| o.id == *eye && o.health > 0. && o.ticks > 0)
        });
        if !self.clock.is_multiple_of(5) {
            return;
        }
        let eyes: Vec<_> = c
            .objects
            .iter()
            .filter(|o| self.eyes.contains(&o.id))
            .map(|o| o.position)
            .collect();
        for at in eyes {
            let at = add(at, [0., 0.4, 0.]);
            for a in c.enemies(id, at, self.settings.scan_range.clamp(1., 32.)) {
                if c.visible(at, a.eye()) {
                    c.intel.reveal(c.world, id, a.entity, 8);
                    self.mark(&a, at);
                }
            }
            c.world
                .ability_sound(at, "minecraft:entity.warden.listening");
        }
    }
    fn tethers(&mut self, id: u128, c: &mut Context<'_, impl World>) {
        for catch in &mut self.catches {
            catch.ticks = catch.ticks.saturating_sub(1);
            catch.targets.retain_mut(|t| {
                let Some(a) = c
                    .actors
                    .iter()
                    .find(|a| a.entity == t.target && a.eligible && a.health > 0.)
                else {
                    return false;
                };
                if !c.world.recon_target(id, a.entity)
                    || c.world.teleport_stamp(a.entity) != t.teleport
                    || distance(a.position, t.safe) > 8.
                {
                    return false;
                }
                if distance(a.position, catch.at) <= 3.5 {
                    t.safe = a.position;
                } else if let Entity::Player(player) = a.entity {
                    let last = a.bounds.translated(sub(t.safe, a.position));
                    if !clear_box(c.world, last) || !c.world.move_player(player, t.safe) {
                        return false;
                    }
                    c.world.velocity(player, [0.; 3], false);
                } else {
                    c.world
                        .push_entity(a.entity, scale(unit(sub(catch.at, a.position)), 0.6));
                }
                true
            });
        }
        self.catches.retain(|s| s.ticks > 0);
    }
    fn prowlers(&mut self, id: u128, a: &Actor, c: &mut Context<'_, impl World>) {
        self.prowlers.retain(|id| {
            c.objects
                .iter()
                .any(|o| o.id == *id && o.health > 0. && o.ticks > 0)
        });
        for object in &self.prowlers {
            let Some(mut o) = c.objects.iter().find(|o| o.id == *object).cloned() else {
                continue;
            };
            let mut targets = c.enemies(id, o.position, 24.);
            targets.sort_by(|x, y| {
                distance(x.position, o.position).total_cmp(&distance(y.position, o.position))
            });
            let facing = if length(horizontal(o.velocity)) < 0.01 {
                horizontal(a.direction)
            } else {
                horizontal(o.velocity)
            };
            let target = targets.into_iter().find(|t| {
                if o.target == Some(t.entity) {
                    return true;
                }
                distance(t.position, o.position) < 16.
                    && dot(facing, horizontal(sub(t.position, o.position))) > 0.35
                    && (self.tracks.iter().any(|p| p.target == t.entity)
                        || c.visible(add(o.position, [0., 0.3, 0.]), t.eye()))
            });
            let mut direction = facing;
            if let Some(t) = target {
                o.target = Some(t.entity);
                direction = horizontal(sub(t.position, o.position));
                if !c.visible(add(o.position, [0., 0.3, 0.]), t.eye())
                    && let Some(track) = self.tracks.iter().find(|p| p.target == t.entity)
                    && let Some((index, _)) =
                        track.points.iter().enumerate().min_by(|(_, x), (_, y)| {
                            distance(o.position, **x).total_cmp(&distance(o.position, **y))
                        })
                {
                    let next = track.points[(index + 1).min(track.points.len() - 1)];
                    direction = horizontal(sub(next, o.position));
                }
                if o.bounds().expanded(0.4).intersects(t.bounds)
                    && c.visible(add(o.position, [0., 0.3, 0.]), t.eye())
                {
                    if let Entity::Player(player) = t.entity {
                        let v = c.vision.entry(player).or_default();
                        v.nearsight = v.nearsight.max(60);
                    }
                    c.world.effect(t.entity, "minecraft:blindness", 60, 0);
                    c.world
                        .ability_sound(o.position, "minecraft:entity.warden.sonic_charge");
                    o.health = 0.;
                }
            } else {
                o.target = None;
                if self.core.firing && self.prowlers.last() == Some(object) {
                    direction = horizontal(a.direction);
                }
            }
            ground_step(
                c.world,
                &mut o,
                direction,
                self.settings.prowler_speed.clamp(0.05, 0.5),
            );
            if let Some(stored) = c.objects.iter_mut().find(|o| o.id == *object) {
                *stored = o;
            }
        }
        if self.prowlers.is_empty() {
            self.core.firing = false;
        }
    }
    fn wave(&mut self, id: u128, c: &mut Context<'_, impl World>) {
        let Some(mut wave) = self.wave.take() else {
            return;
        };
        c.zones
            .retain(|z| z.owner != id || !matches!(z.kind, zones::ZoneKind::Nightfall));
        if c.decay.is_none() {
            return;
        }
        let before = wave.front;
        wave.front += 1.2;
        let at = add(wave.origin, scale(wave.direction, wave.front));
        let side = [-wave.direction[2], 0., wave.direction[0]];
        if [-10., 0., 10.].into_iter().any(|n| {
            c.world
                .boxes(add(at, scale(side, n)).map(|v| v.floor() as i32))
                .is_none()
        }) {
            return;
        }
        let targets: Vec<_> = c
            .enemies(id, at, 12.)
            .into_iter()
            .filter(|a| {
                let rel = sub(a.position, wave.origin);
                let along = dot(rel, wave.direction);
                along >= before - 1.
                    && along <= wave.front + 1.
                    && dot(rel, side).abs() <= 10.
                    && rel[1].abs() < 5.
                    && !wave.victims.contains(&a.entity)
            })
            .collect();
        // Refresh once for this hit batch: another kit may have changed HP earlier
        // in the same server tick; using cached health could accidentally heal.
        let live = if targets.is_empty() {
            vec![]
        } else {
            c.world.actors()
        };
        for t in targets {
            if wave.victims.len() == 32 {
                break;
            }
            let Some(current) = live
                .iter()
                .find(|a| a.entity == t.entity && a.eligible && a.health > 0.)
            else {
                continue;
            };
            if let Some(debts) = c.decay.as_deref_mut() {
                toxin::pulse(c.world, current, debts, 8., 160);
            }
            if let Entity::Player(player) = t.entity {
                let v = c.vision.entry(player).or_default();
                v.deafen = v.deafen.max(160);
            }
            self.mark(&t, at);
            wave.victims.push(t.entity);
        }
        if c.zones.len() < MAX_ZONES {
            let half = [
                side[0].abs() * 10. + wave.direction[0].abs() * 0.7,
                4.,
                side[2].abs() * 10. + wave.direction[2].abs() * 0.7,
            ];
            c.zones.push(Zone {
                owner: id,
                kind: zones::ZoneKind::Nightfall,
                ticks: 2,
                bounds: Box3 {
                    min: sub(at, half),
                    max: add(at, half),
                },
            });
        }
        if wave.front < self.settings.wave_range.clamp(4., 64.) {
            self.wave = Some(wave);
        }
    }
}
/// Constant-work local obstacle avoidance. No global search or chunk generation.
fn ground_step(w: &impl World, o: &mut Object, direction: V3, speed: f64) {
    o.velocity[1] = (o.velocity[1] - 0.06).max(-0.6);
    let down = sweep(w, o.bounds(), [0., o.velocity[1], 0.]);
    o.position = add(o.position, down);
    if down[1].abs() < 0.01 {
        o.velocity[1] = 0.;
    }
    let d = horizontal(direction);
    for angle in [0_f64, 0.785, -0.785, 1.57, -1.57] {
        let v = scale(
            [
                d[0] * angle.cos() - d[2] * angle.sin(),
                0.,
                d[0] * angle.sin() + d[2] * angle.cos(),
            ],
            speed,
        );
        let delta = sweep(w, o.bounds(), v);
        if length(delta) >= speed * 0.9 {
            o.position = add(o.position, delta);
            o.velocity[0] = v[0];
            o.velocity[2] = v[2];
            return;
        }
        let up = [0., 0.5, 0.];
        if o.velocity[1] == 0.
            && clear_box(w, o.bounds().translated(up))
            && length(sweep(w, o.bounds().translated(up), v)) >= speed * 0.9
            && block_hit(
                w,
                add(add(o.position, up), v),
                add(add(o.position, v), [0., -0.3, 0.]),
                0.,
            )
            .is_some()
        {
            o.position = add(add(o.position, up), v);
            o.velocity[0] = v[0];
            o.velocity[2] = v[2];
            return;
        }
    }
}
impl Agent for Fade {
    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }
    fn owns(&self, id: u64) -> bool {
        self.orbs.iter().any(|o| o.id == id)
            || self.eyes.contains(&id)
            || self.prowlers.contains(&id)
    }
    fn equipped(&self) -> bool {
        self.core.equipped.is_some() || self.core.firing && !self.prowlers.is_empty()
    }
    fn zone_active(&self, kind: zones::ZoneKind) -> bool {
        match kind {
            zones::ZoneKind::Seize => !self.catches.is_empty(),
            zones::ZoneKind::Nightfall => self.wave.is_some(),
            _ => false,
        }
    }
    fn stats(&self) -> [u32; 4] {
        [
            self.prowlers.len() as u32,
            self.catches.len() as u32,
            self.eyes.len() as u32,
            u32::from(self.wave.is_some()),
        ]
    }
    fn trails(&self) -> Vec<Beam> {
        self.tracks
            .iter()
            .take(2)
            .map(|t| Beam {
                from: if t.points.len() > 1 {
                    t.points[t.points.len() - 2]
                } else {
                    t.origin
                },
                to: *t.points.last().expect("track has an initial point"),
                ticks: t.ticks,
            })
            .collect()
    }
    fn activate(&mut self, slot: usize, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        if !a.eligible || slot >= 4 || slot == 3 && (self.wave.is_some() || c.decay.is_none()) {
            return false;
        }
        if let Some(orb) = self.orbs.iter_mut().find(|o| o.slot == slot)
            && let Some(o) = c
                .objects
                .iter_mut()
                .find(|o| o.id == orb.id && o.health > 0. && o.ticks > 0)
        {
            orb.falling = true;
            o.velocity = [0., -0.8, 0.];
            return true;
        }
        self.core.equip(slot)
    }
    fn fire(&mut self, _alt: bool, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let (Some(id), Some(slot)) = (owner(a), self.core.equipped) else {
            return false;
        };
        if !a.eligible || self.core.slots[slot].charges == 0 {
            return false;
        }
        if slot == 3 {
            if self.wave.is_some() || c.decay.is_none() || c.zones.len() >= MAX_ZONES {
                return false;
            }
            self.wave = Some(Wave {
                origin: a.position,
                direction: horizontal(a.direction),
                front: 0.,
                victims: vec![],
            });
            c.world
                .ability_sound(a.position, "minecraft:entity.warden.roar");
        } else {
            if slot == 1 && c.zones.len() >= MAX_ZONES {
                return false;
            }
            let at = add(a.eye(), scale(a.direction, 0.4));
            let kind = match slot {
                0 => ObjectKind::Prowler,
                1 => ObjectKind::SeizeOrb,
                _ => ObjectKind::HauntOrb,
            };
            if !clear_box(
                c.world,
                Box3 {
                    min: add(at, [-0.25, 0., -0.25]),
                    max: add(at, [0.25, 0.5, 0.25]),
                },
            ) {
                return false;
            }
            let Some(object) = c.object(
                id,
                kind,
                at,
                if slot == 0 { 6. } else { 1. },
                if slot == 0 {
                    self.settings.prowler_ticks.clamp(1, 240)
                } else {
                    120
                },
                scale(a.direction, if slot == 0 { 0.3 } else { 0.7 }),
            ) else {
                return false;
            };
            if slot == 0 {
                self.prowlers.push(object);
            } else {
                self.orbs.push(Orb {
                    id: object,
                    slot,
                    age: 0,
                    falling: false,
                });
            }
            c.world
                .ability_sound(at, "minecraft:entity.warden.tendril_clicks");
        }
        self.core.spend(slot);
        self.core.cancel();
        if slot == 0 {
            self.core.firing = true;
        }
        true
    }
    fn leave(&mut self) {
        self.core.cancel();
        self.orbs.clear();
        self.eyes.clear();
        self.prowlers.clear();
        self.catches.clear();
        self.tracks.clear();
        self.wave = None;
    }
    fn tick(&mut self, a: &Actor, c: &mut Context<'_, impl World>) {
        let Some(id) = owner(a) else {
            return;
        };
        self.clock = self.clock.wrapping_add(1);
        self.tracks.retain_mut(|t| {
            t.ticks = t.ticks.saturating_sub(1);
            let Some(target) = c
                .actors
                .iter()
                .find(|a| a.entity == t.target && a.eligible && a.health > 0.)
            else {
                return false;
            };
            if !c.world.recon_target(id, t.target) {
                return false;
            }
            if self.clock.is_multiple_of(4)
                && t.points
                    .last()
                    .is_some_and(|p| distance(*p, target.position) > 0.4)
            {
                if t.points.len() == 16 {
                    t.points.remove(0);
                }
                t.points.push(target.position);
            }
            t.ticks > 0
        });
        self.orbs(id, c);
        self.eyes(id, c);
        self.tethers(id, c);
        self.prowlers(id, a, c);
        self.wave(id, c);
    }
}
