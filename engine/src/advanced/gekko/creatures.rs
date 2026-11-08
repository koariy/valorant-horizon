use super::*;
impl Gekko {
    pub(super) fn spawn_wing(&mut self, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Some(id) = owner(a) else { return false };
        if self.wing.is_some() || !a.ground {
            return false;
        }
        let at = add(a.position, scale(horizontal(a.direction), 0.7));
        if !clear_box(
            c.world,
            Box3 {
                min: add(at, [-0.25, 0., -0.25]),
                max: add(at, [0.25, 0.5, 0.25]),
            },
        ) {
            return false;
        }
        let left = self.settings.wing_ticks.clamp(40, 200);
        let Some(object) = c.object(
            id,
            ObjectKind::Wingman,
            at,
            5.,
            left + 1,
            scale(horizontal(a.direction), 0.2),
        ) else {
            return false;
        };
        self.wing = Some(Creature {
            object,
            left,
            next: 0,
            last: at,
        });
        true
    }
    pub(super) fn spawn_dizzy(&mut self, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Some(id) = owner(a) else { return false };
        if self.dizzy.is_some() {
            return false;
        }
        let left = self.settings.dizzy_ticks.clamp(20, 100);
        let at = a.eye();
        let Some(object) = c.object(
            id,
            ObjectKind::Dizzy,
            at,
            2.,
            left + 1,
            scale(unit(a.direction), 0.25),
        ) else {
            return false;
        };
        self.dizzy = Some(Creature {
            object,
            left,
            next: 8,
            last: at,
        });
        true
    }
    pub(super) fn spawn_thrash(&mut self, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Some(id) = owner(a) else { return false };
        if self.thrash.is_some() || !a.ground {
            return false;
        }
        let at = add(a.position, scale(horizontal(a.direction), 0.7));
        if !clear_box(
            c.world,
            Box3 {
                min: add(at, [-0.25, 0., -0.25]),
                max: add(at, [0.25, 0.5, 0.25]),
            },
        ) {
            return false;
        }
        let left = self.settings.thrash_ticks.clamp(40, 240);
        let Some(object) = c.object(id, ObjectKind::Thrash, at, 8., left + 1, [0.; 3]) else {
            return false;
        };
        let Some(camera) = Camera::new(c.world, a, add(at, [0., 0.35, 0.]), object) else {
            c.objects.retain(|o| o.id != object);
            return false;
        };
        self.thrash = Some(companions::Pilot {
            camera,
            left,
            hurt: c.world.protection_state(id).hurt_epoch,
            leap: 0,
        });
        true
    }
    pub(super) fn tick_creatures(&mut self, id: u128, a: &Actor, c: &mut Context<'_, impl World>) {
        self.wing(id, c);
        self.dizzy(id, c);
        self.plasma(id, c);
        if let Some(mut thrash) = self.thrash.take() {
            match thrash.step(id, a, c, 0.28) {
                companions::Drive::Moving => self.thrash = Some(thrash),
                companions::Drive::Ended => {
                    self.queue(3, add(thrash.camera.remote.position, [0., -0.35, 0.]))
                }
                companions::Drive::Impact { at, .. } => {
                    for target in c.enemies(id, at, 3.) {
                        if c.visible(at, target.eye()) {
                            control::detain(
                                c.controls,
                                c.world,
                                target.entity,
                                self.settings.detain_ticks.clamp(20, 160),
                            );
                        }
                    }
                    c.world.burst_fx(at);
                    self.queue(3, at);
                }
            }
        }
    }
    fn wing(&mut self, id: u128, c: &mut Context<'_, impl World>) {
        let Some(mut creature) = self.wing.take() else {
            return;
        };
        creature.left = creature.left.saturating_sub(1);
        let object = c
            .objects
            .iter()
            .find(|o| o.id == creature.object && o.health > 0. && o.ticks > 0)
            .cloned();
        let Some(mut o) = object.filter(|_| creature.left > 0) else {
            self.queue(1, creature.last);
            return;
        };
        let facing = horizontal(o.velocity);
        let target = companions::nearest(c, id, o.position, 10.)
            .into_iter()
            .find(|t| {
                dot(unit(sub(t.eye(), add(o.position, [0., 0.25, 0.]))), facing) > 0.1
                    && c.visible(add(o.position, [0., 0.25, 0.]), t.eye())
            });
        if let Some(target) = target.filter(|t| distance(t.position, o.position) <= 3.5) {
            c.concuss(target.entity, 40);
            if c.beams.len() < recon::MAX_BEAMS {
                c.beams.push(Beam {
                    from: add(o.position, [0., 0.25, 0.]),
                    to: target.eye(),
                    ticks: 4,
                });
            }
            c.world
                .ability_sound(o.position, "minecraft:block.anvil.land");
            self.queue(1, o.position);
            return;
        }
        walking::step(c.world, &mut o, facing, 0.2);
        creature.last = o.position;
        if let Some(stored) = c.objects.iter_mut().find(|x| x.id == o.id) {
            *stored = o;
        }
        self.wing = Some(creature);
    }
    fn dizzy(&mut self, id: u128, c: &mut Context<'_, impl World>) {
        let Some(mut creature) = self.dizzy.take() else {
            return;
        };
        creature.left = creature.left.saturating_sub(1);
        let object = c
            .objects
            .iter()
            .find(|o| o.id == creature.object && o.health > 0. && o.ticks > 0)
            .cloned();
        let Some(mut o) = object.filter(|_| creature.left > 0) else {
            self.queue(2, creature.last);
            return;
        };
        if matches!(
            throwable::advance(c.world, &mut o, 0.015, 0.2),
            throwable::Flight::Surface(_)
        ) {
            o.velocity = [0.; 3];
        }
        if o.health <= 0. {
            self.queue(2, creature.last);
            return;
        }
        creature.next = creature.next.saturating_sub(1);
        if creature.next == 0 {
            for target in companions::nearest(c, id, o.position, 9.) {
                if self.bolts.len() >= 12 || c.objects.len() >= MAX_OBJECTS {
                    break;
                }
                let from = add(o.position, [0., 0.2, 0.]);
                if !c.visible(from, target.eye()) {
                    continue;
                }
                if let Some(object) = c.object(
                    id,
                    ObjectKind::Plasma,
                    from,
                    1.,
                    20,
                    scale(unit(sub(target.eye(), from)), 0.75),
                ) {
                    self.bolts.push(Bolt {
                        object,
                        source: o.id,
                        left: 20,
                    });
                }
            }
            creature.next = 15;
        }
        creature.last = o.position;
        if let Some(stored) = c.objects.iter_mut().find(|x| x.id == o.id) {
            *stored = o;
        }
        self.dizzy = Some(creature);
    }
    fn plasma(&mut self, id: u128, c: &mut Context<'_, impl World>) {
        for bolt in &mut self.bolts {
            bolt.left = bolt.left.saturating_sub(1);
            let Some(mut o) = c
                .objects
                .iter()
                .find(|o| o.id == bolt.object && o.ticks > 0)
                .cloned()
                .filter(|_| bolt.left > 0)
            else {
                bolt.object = 0;
                continue;
            };
            let hit = companions::contact(
                c,
                id,
                o.position,
                add(o.position, o.velocity),
                0.1,
                true,
                Some(bolt.source),
            );
            o.position = hit.at;
            if hit.blocked {
                if let Some(target) = hit.target.filter(|t| c.world.recon_target(id, *t)) {
                    if let Entity::Player(player) = target {
                        if c.world.effect(target, "minecraft:blindness", 30, 0) {
                            let v = c.vision.entry(player).or_default();
                            v.splash = v.splash.max(30);
                        }
                    } else {
                        c.nearsight(target, 30);
                    }
                }
                bolt.object = 0;
            }
            if let Some(stored) = c.objects.iter_mut().find(|x| x.id == o.id) {
                *stored = o;
            }
        }
        self.bolts.retain(|b| b.object != 0);
    }
}
