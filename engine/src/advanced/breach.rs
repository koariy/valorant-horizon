use super::*;
use crate::abilities::Kind;
pub const NAMES: [&str; 4] = ["Aftershock", "Flashpoint", "Fault Line", "Rolling Thunder"];
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
    pub wall_range: f64,
    pub aftershock_delay: u32,
    pub fault_range: f64,
    pub thunder_range: f64,
    pub concuss_ticks: u32,
    pub knock_up: f64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            wall_range: 12.,
            aftershock_delay: 30,
            fault_range: 18.,
            thunder_range: 24.,
            concuss_ticks: 60,
            knock_up: 0.55,
        }
    }
}
struct Charge {
    object: u64,
    flash: bool,
    delay: u32,
    bursts: u8,
}
struct Quake {
    object: u64,
    origin: V3,
    direction: V3,
    distance: f64,
    range: f64,
    next: u32,
    thunder: bool,
    victims: Vec<Entity>,
}
pub struct Breach {
    pub core: Core,
    pub settings: Settings,
    charges: Vec<Charge>,
    quakes: Vec<Quake>,
    affected: u32,
}
impl Default for Breach {
    fn default() -> Self {
        Self {
            core: Core::new(DEFINITIONS),
            settings: Settings::default(),
            charges: vec![],
            quakes: vec![],
            affected: 0,
        }
    }
}
impl Breach {
    fn wall_exit(&self, a: &Actor, c: &mut Context<'_, impl World>) -> Option<V3> {
        let direction = unit(a.direction);
        let from = a.eye();
        let to = add(
            from,
            scale(direction, self.settings.wall_range.clamp(2., 16.)),
        );
        let t = c.terrain_limit(from, to)?;
        if t <= 0. {
            return None;
        }
        let entry = add(from, scale(sub(to, from), t));
        if !block_collision(
            c.world,
            add(entry, scale(direction, -0.1)),
            add(entry, scale(direction, 0.1)),
            0.,
        )
        .is_some_and(|h| h.known)
        {
            return None;
        }
        for n in 1..=16 {
            if c.budget.rays == 0 {
                return None;
            }
            c.budget.rays -= 1;
            let exit = add(entry, scale(direction, f64::from(n) * 0.25 + 0.08));
            if clear_box(
                c.world,
                Box3 {
                    min: add(exit, [-0.08; 3]),
                    max: add(exit, [0.08; 3]),
                },
            ) {
                return Some(exit);
            }
        }
        None
    }
    fn quake(&mut self, a: &Actor, c: &mut Context<'_, impl World>, thunder: bool) -> bool {
        let Some(id) = owner(a) else { return false };
        let slot = if thunder { 3 } else { 2 };
        if self.core.slots[slot].charges == 0 {
            return false;
        }
        let origin = a.position;
        let direction = horizontal(a.direction);
        if length(direction) < 0.9 {
            return false;
        }
        let side = [-direction[2], 0., direction[0]];
        let range = if thunder {
            self.settings.thunder_range.clamp(8., 32.)
        } else {
            4. + (self.settings.fault_range.clamp(4., 24.) - 4.)
                * f64::from(self.core.charge.min(40))
                / 40.
        };
        let Some(object) = c.object(
            id,
            ObjectKind::SeismicFront,
            add(origin, side),
            1.,
            100,
            sub(origin, side),
        ) else {
            return false;
        };
        self.quakes.push(Quake {
            object,
            origin,
            direction,
            distance: 0.,
            range,
            next: if thunder { 12 } else { 8 },
            thunder,
            victims: vec![],
        });
        self.core.spend(slot);
        self.core.cancel();
        c.world.ability_sound(origin, "minecraft:block.anvil.land");
        true
    }
}
impl Agent for Breach {
    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }
    fn owns(&self, id: u64) -> bool {
        self.charges.iter().any(|x| x.object == id) || self.quakes.iter().any(|x| x.object == id)
    }
    fn stats(&self) -> [u32; 4] {
        [
            self.core.charge,
            self.charges.len() as u32,
            self.quakes.len() as u32,
            self.affected,
        ]
    }
    fn busy(&self) -> bool {
        self.core.busy() || !self.charges.is_empty() || !self.quakes.is_empty()
    }
    fn leave(&mut self) {
        self.core.cancel();
        self.charges.clear();
        self.quakes.clear();
    }
    fn activate(&mut self, slot: usize, a: &Actor, _c: &mut Context<'_, impl World>) -> bool {
        a.eligible && a.health > 0. && self.core.equip(slot)
    }
    fn fire(&mut self, alt: bool, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let (Some(id), Some(slot)) = (owner(a), self.core.equipped) else {
            return false;
        };
        if !a.eligible || self.core.slots[slot].charges == 0 {
            return false;
        }
        if slot == 2 {
            if alt {
                return self.quake(a, c, false);
            }
            self.core.firing = true;
            return true;
        }
        if slot == 3 {
            return self.quake(a, c, true);
        }
        let Some(exit) = self.wall_exit(a, c) else {
            return false;
        };
        let delay = if slot == 1 {
            10
        } else {
            self.settings.aftershock_delay.clamp(10, 60)
        };
        let Some(object) = c.object(
            id,
            ObjectKind::WallCharge,
            exit,
            1.,
            delay + 30,
            unit(a.direction),
        ) else {
            return false;
        };
        self.charges.push(Charge {
            object,
            flash: slot == 1,
            delay,
            bursts: 0,
        });
        self.core.spend(slot);
        self.core.cancel();
        c.world
            .ability_sound(exit, "minecraft:block.respawn_anchor.charge");
        true
    }
    fn release(&mut self, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        if self.core.equipped == Some(2) && self.core.firing {
            self.core.firing = false;
            self.quake(a, c, false)
        } else {
            self.core.firing = false;
            true
        }
    }
    fn tick(&mut self, a: &Actor, c: &mut Context<'_, impl World>) {
        let Some(id) = owner(a) else { return };
        for charge in &mut self.charges {
            let Some(o) = c
                .objects
                .iter()
                .find(|o| o.id == charge.object && o.ticks > 0)
                .cloned()
            else {
                charge.object = 0;
                continue;
            };
            charge.delay = charge.delay.saturating_sub(1);
            if charge.delay > 0 {
                continue;
            }
            if !clear_box(
                c.world,
                Box3 {
                    min: add(o.position, [-0.05; 3]),
                    max: add(o.position, [0.05; 3]),
                },
            ) {
                charge.object = 0;
                continue;
            }
            if charge.flash {
                c.flash(o.position, 12., 60);
                charge.object = 0;
                c.world
                    .ability_sound(o.position, "minecraft:entity.firework_rocket.blast");
            } else {
                let far = add(o.position, scale(unit(o.velocity), 4.));
                let limit = c.terrain_limit(o.position, far).unwrap_or(1.);
                let far = add(o.position, scale(sub(far, o.position), limit));
                for target in c.enemies(id, o.position, 6.) {
                    if !c.world.status_immune(target.entity)
                        && target
                            .bounds
                            .expanded(0.9)
                            .segment(o.position, far)
                            .is_some()
                        && c.visible(o.position, target.eye())
                    {
                        c.hurt_with(id, target.entity, 9., "minecraft:explosion");
                        self.affected += 1;
                    }
                }
                charge.bursts += 1;
                charge.delay = 10;
                if charge.bursts >= 2 {
                    charge.object = 0;
                }
                c.world.burst_fx(o.position);
            }
        }
        self.charges.retain(|ch| ch.object != 0);
        for quake in &mut self.quakes {
            if !c
                .objects
                .iter()
                .any(|o| o.id == quake.object && o.ticks > 0)
            {
                quake.object = 0;
                continue;
            }
            quake.next = quake.next.saturating_sub(1);
            if quake.next > 0 {
                continue;
            }
            let old = quake.distance;
            let distance = (old + 2.).min(quake.range);
            let from = add(quake.origin, scale(quake.direction, old));
            let end = recon::loaded_end(c.world, from, quake.direction, distance - old);
            if crate::gameplay::geometry::distance(from, end) + 0.1 < distance - old {
                quake.object = 0;
                continue;
            }
            quake.distance = distance;
            quake.next = if quake.thunder { 4 } else { 1 };
            let width = if quake.thunder {
                (2. + distance * 0.2).min(6.)
            } else {
                1.5
            };
            for target in c.enemies(id, quake.origin, quake.range + 4.) {
                if quake.victims.len() >= 32
                    || quake.victims.contains(&target.entity)
                    || c.world.status_immune(target.entity)
                {
                    continue;
                }
                let offset = sub(target.position, quake.origin);
                let along = dot(offset, quake.direction);
                let lateral = sub(offset, scale(quake.direction, along));
                if along >= old - 0.3
                    && along <= distance + 0.3
                    && offset[1].abs() <= 3.
                    && length([lateral[0], 0., lateral[2]]) <= width + 0.3
                {
                    c.concuss(target.entity, self.settings.concuss_ticks.clamp(20, 160));
                    if quake.thunder {
                        c.world.push_entity(
                            target.entity,
                            add(
                                target.velocity,
                                [0., self.settings.knock_up.clamp(0.2, 1.2), 0.],
                            ),
                        );
                    }
                    quake.victims.push(target.entity);
                    self.affected += 1;
                }
            }
            if let Some(o) = c.objects.iter_mut().find(|o| o.id == quake.object) {
                let side = scale([-quake.direction[2], 0., quake.direction[0]], width);
                o.position = add(end, side);
                o.velocity = sub(end, side);
            }
            c.world.ability_sound(end, "minecraft:block.anvil.land");
            if distance >= quake.range {
                quake.object = 0;
            }
        }
        self.quakes.retain(|q| q.object != 0);
    }
}
