use super::*;
use crate::abilities::Kind;
pub const NAMES: [&str; 4] = ["Regrowth", "Trailblazer", "Guiding Light", "Seekers"];
pub const DEFINITIONS: [Ability; 4] = [
    Ability {
        kind: Kind::Active,
        cooldown: 1,
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
        charges: 2,
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
    pub pool_units: u32,
    pub pool_regen_interval: u32,
    pub tiger_ticks: u32,
    pub hawk_ticks: u32,
    pub seeker_ticks: u32,
    pub nearsight_ticks: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            pool_units: 80,
            pool_regen_interval: 10,
            tiger_ticks: 120,
            hawk_ticks: 60,
            seeker_ticks: 200,
            nearsight_ticks: 60,
        }
    }
}
type Tiger = companions::Pilot;
struct Hawk {
    object: u64,
    left: u32,
    pop: bool,
}
pub struct Skye {
    pub core: Core,
    pub settings: Settings,
    pub pool: u32,
    regen: u32,
    rest: u32,
    tiger: Option<Tiger>,
    hawk: Option<Hawk>,
    seekers: Vec<u64>,
    confirm: u32,
}
impl Default for Skye {
    fn default() -> Self {
        Self {
            core: Core::new(DEFINITIONS),
            settings: Settings::default(),
            pool: 80,
            regen: 0,
            rest: 0,
            tiger: None,
            hawk: None,
            seekers: vec![],
            confirm: 0,
        }
    }
}
impl Skye {
    fn healing(&mut self, id: u128, a: &Actor, c: &mut Context<'_, impl World>) {
        self.pool = self.pool.min(self.settings.pool_units.clamp(1, 160));
        let channel = self.core.equipped == Some(0) && self.core.firing && self.pool > 0;
        let mut healed = false;
        if channel {
            let allies = c.world.intel_viewers(id);
            let targets: Vec<_> = c
                .actors
                .iter()
                .filter(|t| {
                    t.entity != a.entity
                        && t.eligible
                        && t.health > 0.
                        && t.health < t.max_health
                        && matches!(t.entity,Entity::Player(p)if allies.contains(&p))
                        && distance(a.eye(), t.eye()) <= 8.
                })
                .take(16)
                .cloned()
                .collect();
            for target in targets {
                if c.budget.actors == 0 {
                    break;
                }
                c.budget.actors -= 1;
                let Entity::Player(target_id) = target.entity else {
                    continue;
                };
                if !c.world.protection_state(target_id).downed
                    && c.visible(a.eye(), target.eye())
                    && c.world
                        .heal(target.entity, (target.max_health - target.health).min(0.25))
                {
                    healed = true;
                    if c.beams.len() < recon::MAX_BEAMS {
                        c.beams.push(Beam {
                            from: a.eye(),
                            to: target.eye(),
                            ticks: 2,
                        });
                    }
                }
            }
            c.world.effect(a.entity, "minecraft:slowness", 2, 2);
        }
        if healed {
            self.pool -= 1;
            self.rest = 100;
            self.regen = 0;
        } else if !channel {
            self.rest = self.rest.saturating_sub(1);
            if self.rest == 0 && self.pool < self.settings.pool_units.clamp(1, 160) {
                self.regen += 1;
                if self.regen >= self.settings.pool_regen_interval.clamp(2, 40) {
                    self.pool += 1;
                    self.regen = 0;
                }
            }
        }
        if self.pool == 0 {
            self.core.firing = false;
        }
        self.core.slots[0].charges = u8::from(self.pool > 0);
        self.core.slots[0].remaining = if self.pool == 0 {
            self.rest + self.settings.pool_regen_interval.clamp(2, 40) - self.regen
        } else {
            0
        };
        self.core.slots[0].active = u32::from(channel && self.pool > 0);
    }
    fn tiger(&mut self, id: u128, a: &Actor, c: &mut Context<'_, impl World>) {
        let Some(mut tiger) = self.tiger.take() else {
            return;
        };
        match tiger.step(id, a, c, 0.22) {
            companions::Drive::Moving => self.tiger = Some(tiger),
            companions::Drive::Ended => {}
            companions::Drive::Impact { at, target } => {
                if let Some(target) = target.filter(|t| c.world.recon_target(id, *t)) {
                    c.hurt(id, target, 6.);
                }
                for target in c.enemies(id, at, 3.) {
                    if c.visible(at, target.eye()) {
                        c.concuss(target.entity, 40);
                    }
                }
                c.world.burst_fx(at);
            }
        }
    }
    fn clear_hawk_input(&mut self) {
        if self.core.equipped == Some(2) {
            self.core.cancel();
        }
    }
    fn hawk(&mut self, id: u128, a: &Actor, c: &mut Context<'_, impl World>) {
        let Some(mut hawk) = self.hawk.take() else {
            return;
        };
        let Some(mut object) = c
            .objects
            .iter()
            .find(|o| o.id == hawk.object && o.health > 0. && o.ticks > 0)
            .cloned()
        else {
            self.clear_hawk_input();
            return;
        };
        hawk.left = hawk.left.saturating_sub(1);
        if !hawk.pop && hawk.left > 0 {
            if self.core.equipped == Some(2) && self.core.firing {
                object.velocity = scale(unit(a.direction), 0.4);
            }
            if matches!(
                throwable::advance(c.world, &mut object, 0., 0.1),
                throwable::Flight::Unknown
            ) {
                self.clear_hawk_input();
                return;
            }
        }
        if hawk.pop || hawk.left == 0 {
            let mut confirmed = false;
            for enemy in c.enemies(id, object.position, 12.) {
                if c.visible(object.position, enemy.eye()) {
                    confirmed = true;
                    break;
                }
            }
            c.flash(object.position, 12., 60);
            if confirmed {
                self.confirm = 30;
                c.world
                    .ability_sound(a.position, "minecraft:block.note_block.bell");
            }
            c.world
                .ability_sound(object.position, "minecraft:entity.firework_rocket.blast");
            self.clear_hawk_input();
            return;
        }
        if let Some(stored) = c.objects.iter_mut().find(|o| o.id == object.id) {
            *stored = object;
        }
        self.hawk = Some(hawk);
    }
    fn seekers(&mut self, id: u128, c: &mut Context<'_, impl World>) {
        for seeker in &mut self.seekers {
            let Some(mut o) = c
                .objects
                .iter()
                .find(|o| o.id == *seeker && o.health > 0. && o.ticks > 0)
                .cloned()
            else {
                *seeker = 0;
                continue;
            };
            let target = o.target.and_then(|e| {
                c.actors
                    .iter()
                    .find(|a| a.entity == e && a.eligible && a.health > 0.)
                    .cloned()
            });
            let Some(target) = target.filter(|t| {
                c.world.recon_target(id, t.entity)
                    && c.world.bot_target(id, t.entity)
                    && !c.world.status_immune(t.entity)
                    && distance(o.position, t.position) < 40.
            }) else {
                *seeker = 0;
                continue;
            };
            let direction = sub(target.position, o.position);
            walking::step(c.world, &mut o, direction, 0.18);
            if distance(o.position, target.position) < 0.8
                && c.visible(add(o.position, [0., 0.3, 0.]), target.eye())
            {
                c.nearsight(target.entity, self.settings.nearsight_ticks.clamp(20, 120));
                *seeker = 0;
                c.world
                    .ability_sound(o.position, "minecraft:block.amethyst_block.break");
            }
            if let Some(stored) = c.objects.iter_mut().find(|x| x.id == o.id) {
                *stored = o;
            }
        }
        self.seekers.retain(|id| *id != 0);
    }
}
impl Agent for Skye {
    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }
    fn owns(&self, id: u64) -> bool {
        self.tiger.as_ref().is_some_and(|t| t.camera.source == id)
            || self.hawk.as_ref().is_some_and(|h| h.object == id)
            || self.seekers.contains(&id)
    }
    fn busy(&self) -> bool {
        self.core.busy()
            || self.pool < self.settings.pool_units.clamp(1, 160)
            || self.tiger.is_some()
            || self.hawk.is_some()
            || !self.seekers.is_empty()
    }
    fn equipped(&self) -> bool {
        self.core.equipped.is_some() || self.tiger.is_some()
    }
    fn camera(&self) -> Option<V3> {
        self.tiger.as_ref().map(|t| t.camera.remote.position)
    }
    fn stats(&self) -> [u32; 4] {
        [
            self.pool,
            self.tiger.as_ref().map_or(0, |t| t.left),
            self.confirm,
            self.seekers.len() as u32,
        ]
    }
    fn protection(&self, a: &Actor) -> effects::Protection {
        effects::Protection {
            health: a.health,
            disarmed: self.tiger.is_some() || self.core.equipped == Some(0) && self.core.firing,
            ..Default::default()
        }
    }
    fn cancel(&mut self) {
        self.core.cancel();
        self.tiger = None;
    }
    fn leave(&mut self) {
        self.cancel();
        self.hawk = None;
        self.seekers.clear();
    }
    fn activate(&mut self, slot: usize, a: &Actor, _c: &mut Context<'_, impl World>) -> bool {
        if !a.eligible || a.health <= 0. || self.tiger.is_some() {
            return false;
        }
        if slot == 2
            && let Some(h) = &mut self.hawk
        {
            h.pop = true;
            return true;
        }
        if slot == 0 && self.pool == 0 {
            return false;
        }
        self.core.equip(slot)
    }
    fn fire(&mut self, alt: bool, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Some(id) = owner(a).filter(|_| a.eligible && a.health > 0.) else {
            return false;
        };
        if let Some(tiger) = &mut self.tiger {
            return tiger.lunge(a.direction, c, 0.65, 0.35);
        }
        let Some(slot) = self.core.equipped else {
            return false;
        };
        if alt && (slot == 0 || slot == 2) {
            return false;
        }
        if slot == 0 {
            if self.pool == 0 {
                return false;
            }
            self.core.firing = true;
            return true;
        }
        if slot == 2 && self.hawk.is_some() {
            self.core.firing = true;
            return true;
        }
        if self.core.slots[slot].charges == 0 {
            return false;
        }
        if slot == 1 {
            let at = add(a.position, scale(horizontal(a.direction), 0.7));
            if !a.ground
                || !clear_box(
                    c.world,
                    Box3 {
                        min: add(at, [-0.25, 0., -0.25]),
                        max: add(at, [0.25, 0.6, 0.25]),
                    },
                )
            {
                return false;
            }
            let ticks = self.settings.tiger_ticks.clamp(40, 200);
            let Some(object) = c.object(id, ObjectKind::SkyeTiger, at, 6., ticks + 1, [0.; 3])
            else {
                return false;
            };
            let Some(camera) = Camera::new(c.world, a, add(at, [0., 0.35, 0.]), object) else {
                c.objects.retain(|o| o.id != object);
                return false;
            };
            self.tiger = Some(Tiger {
                camera,
                left: ticks,
                hurt: c.world.protection_state(id).hurt_epoch,
                leap: 0,
            });
        } else if slot == 2 {
            let ticks = self.settings.hawk_ticks.clamp(20, 100);
            let Some(object) = c.object(
                id,
                ObjectKind::SkyeHawk,
                a.eye(),
                2.,
                ticks + 2,
                scale(unit(a.direction), 0.4),
            ) else {
                return false;
            };
            self.hawk = Some(Hawk {
                object,
                left: ticks,
                pop: false,
            });
        } else if slot == 3 {
            let targets = companions::nearest(c, id, a.position, 32.)
                .into_iter()
                .take(3)
                .collect::<Vec<_>>();
            if targets.is_empty() || c.objects.len() + targets.len() > MAX_OBJECTS {
                return false;
            }
            let mut created = vec![];
            for target in targets {
                let Some(object) = c.object(
                    id,
                    ObjectKind::SkyeSeeker,
                    a.position,
                    5.,
                    self.settings.seeker_ticks.clamp(40, 300),
                    [0.; 3],
                ) else {
                    c.objects.retain(|o| !created.contains(&o.id));
                    return false;
                };
                if let Some(o) = c.objects.iter_mut().find(|o| o.id == object) {
                    o.target = Some(target.entity);
                }
                created.push(object);
            }
            self.seekers.extend(created);
        } else {
            return false;
        }
        self.core.spend(slot);
        if slot == 2 {
            self.core.firing = true;
        } else {
            self.core.cancel();
        }
        c.world
            .ability_sound(a.position, "minecraft:block.azalea_leaves.place");
        true
    }
    fn tick(&mut self, a: &Actor, c: &mut Context<'_, impl World>) {
        let Some(id) = owner(a) else { return };
        self.confirm = self.confirm.saturating_sub(1);
        self.healing(id, a, c);
        self.tiger(id, a, c);
        self.hawk(id, a, c);
        self.seekers(id, c);
        self.core.slots[1].active = self.tiger.as_ref().map_or(0, |t| t.left);
        self.core.slots[2].active = self.hawk.as_ref().map_or(0, |h| h.left);
    }
}
