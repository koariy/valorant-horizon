use super::*;
impl Gekko {
    pub(super) fn throw_mosh(
        &mut self,
        alt: bool,
        a: &Actor,
        c: &mut Context<'_, impl World>,
    ) -> bool {
        let Some(id) = owner(a) else { return false };
        let Some(object) = c.object(
            id,
            ObjectKind::Mosh,
            a.eye(),
            1.,
            180,
            scale(unit(a.direction), if alt { 0.35 } else { 0.75 }),
        ) else {
            return false;
        };
        self.moshes.push(Mosh {
            object,
            ground: false,
            left: 100,
        });
        true
    }
    pub(super) fn tick_mosh(&mut self, id: u128, c: &mut Context<'_, impl World>) {
        for mosh in &mut self.moshes {
            let Some(mut o) = c
                .objects
                .iter()
                .find(|o| o.id == mosh.object && o.ticks > 0)
                .cloned()
            else {
                mosh.object = 0;
                continue;
            };
            if !mosh.ground {
                mosh.left = mosh.left.saturating_sub(1);
                match throwable::advance(c.world, &mut o, 0.04, 0.1) {
                    throwable::Flight::Unknown => {
                        mosh.object = 0;
                        continue;
                    }
                    throwable::Flight::Surface(n) if n[1] > 0.5 => {
                        mosh.ground = true;
                        mosh.left = self.settings.mosh_delay.clamp(10, 80);
                        o.velocity = [0.; 3];
                        o.kind = ObjectKind::MoshPatch;
                    }
                    throwable::Flight::Surface(n) => throwable::reflect(&mut o, n, 0.5),
                    _ => {}
                }
                if mosh.left == 0 {
                    mosh.object = 0;
                }
            } else {
                mosh.left = mosh.left.saturating_sub(1);
                if mosh.left % 10 == 0 {
                    let burst = mosh.left == 0;
                    let targets: Vec<_> = c
                        .actors
                        .iter()
                        .filter(|t| {
                            t.eligible
                                && t.health > 0.
                                && distance(
                                    [t.position[0], 0., t.position[2]],
                                    [o.position[0], 0., o.position[2]],
                                ) <= 3.5
                                && (t.position[1] - o.position[1]).abs() < 2.5
                        })
                        .take(32)
                        .cloned()
                        .collect();
                    for target in targets {
                        if c.budget.actors == 0 {
                            break;
                        }
                        c.budget.actors -= 1;
                        if c.world.can_harm(id, target.entity)
                            && !c.world.status_immune(target.entity)
                            && c.visible(add(o.position, [0., 0.1, 0.]), target.eye())
                        {
                            let damage = if burst {
                                12. * (1. - distance(target.position, o.position) as f32 / 3.5)
                                    .clamp(0.25, 1.)
                            } else {
                                1.
                            };
                            c.hurt_with(
                                id,
                                target.entity,
                                damage,
                                if burst {
                                    "minecraft:explosion"
                                } else {
                                    "minecraft:magic"
                                },
                            );
                        }
                    }
                    if burst {
                        c.world.burst_fx(o.position);
                        mosh.object = 0;
                    }
                }
            }
            if mosh.ground {
                o.ticks = mosh.left + 1;
            }
            if let Some(stored) = c.objects.iter_mut().find(|x| x.id == o.id) {
                *stored = o;
            }
        }
        self.moshes.retain(|m| m.object != 0);
    }
}
