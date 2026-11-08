use super::*;
impl Gekko {
    pub(super) fn queue(&mut self, slot: usize, at: V3) {
        if self.pending.len() < 4 && (slot != 3 || self.thrash_reclaims == 0) {
            self.pending.push(Pending {
                slot,
                at,
                left: self.settings.globule_ticks.clamp(40, 400),
            });
        }
    }
    pub(super) fn begin_reclaim(&mut self, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Some(id) = owner(a).filter(|_| a.eligible && a.health > 0.) else {
            return false;
        };
        if self.thrash.is_some() {
            return false;
        }
        if let Some(r) = &mut self.reclaim {
            r.lease = 12;
            return true;
        }
        let chosen = self
            .globules
            .iter()
            .filter_map(|g| {
                c.objects
                    .iter()
                    .find(|o| {
                        o.id == g.object
                            && o.owner == id
                            && o.ticks > 0
                            && distance(a.eye(), o.position) <= 2.5
                            && self.core.slots[g.slot].charges == 0
                    })
                    .map(|o| (g.object, g.slot, o.position))
            })
            .min_by(|x, y| distance(a.eye(), x.2).total_cmp(&distance(a.eye(), y.2)));
        let Some((object, slot, at)) = chosen else {
            return false;
        };
        if !c.visible(a.eye(), at) {
            return false;
        }
        self.core.cancel();
        self.reclaim = Some(Reclaim {
            object,
            slot,
            origin: a.position,
            hurt: c.world.protection_state(id).hurt_epoch,
            progress: 0,
            lease: 12,
        });
        true
    }
    pub(super) fn tick_recovery(&mut self, id: u128, a: &Actor, c: &mut Context<'_, impl World>) {
        for pending in &mut self.pending {
            pending.left = pending.left.saturating_sub(1);
            if pending.left == 0 {
                continue;
            }
            let kind = match pending.slot {
                1 => ObjectKind::WingmanGlobule,
                2 => ObjectKind::DizzyGlobule,
                _ => ObjectKind::ThrashGlobule,
            };
            if let Some(object) = c.object(id, kind, pending.at, 1., pending.left, [0.; 3]) {
                self.globules.push(Globule {
                    object,
                    slot: pending.slot,
                });
                pending.left = 0;
            }
        }
        self.pending.retain(|p| p.left > 0);
        self.globules
            .retain(|g| c.objects.iter().any(|o| o.id == g.object && o.ticks > 0));
        for globule in &self.globules {
            if let Some(o) = c.objects.iter_mut().find(|o| o.id == globule.object) {
                match throwable::advance(c.world, o, 0.04, 0.12) {
                    throwable::Flight::Surface(_) => o.velocity = [0.; 3],
                    throwable::Flight::Unknown => o.ticks = 0,
                    _ => {}
                }
            }
        }
        let Some(mut reclaim) = self.reclaim.take() else {
            return;
        };
        reclaim.lease = reclaim.lease.saturating_sub(1);
        let Some(at) = c
            .objects
            .iter()
            .find(|o| o.id == reclaim.object && o.owner == id && o.ticks > 0)
            .map(|o| o.position)
        else {
            return;
        };
        if reclaim.lease == 0
            || distance(reclaim.origin, a.position) > 0.3
            || c.world.protection_state(id).hurt_epoch != reclaim.hurt
            || distance(a.eye(), at) > 2.5
            || !c.visible(a.eye(), at)
            || self.core.slots[reclaim.slot].charges > 0
        {
            return;
        }
        reclaim.progress += 1;
        if reclaim.progress >= self.settings.reclaim_ticks.clamp(10, 40) {
            self.globules.retain(|g| g.object != reclaim.object);
            self.core.slots[reclaim.slot].remaining = self.settings.recovery_ticks.clamp(20, 200);
            if reclaim.slot == 3 {
                self.thrash_reclaims += 1;
                self.ult_recovering = true;
            }
            c.world
                .ability_sound(at, "minecraft:block.amethyst_block.resonate");
        } else {
            self.reclaim = Some(reclaim);
        }
    }
}
