use super::*;
use crate::abilities::Kind;
pub const NAMES: [&str; 4] = [
    "Contingency",
    "Undercut",
    "Double Tap",
    "Kill Contract (unavailable)",
];
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
        charges: 0,
    },
];
#[derive(Clone, Copy)]
pub struct Settings {
    pub focus_ticks: u32,
    pub flow_ticks: u32,
    pub shield_ticks: u32,
    pub wall_ticks: u32,
    pub fragile_ticks: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            focus_ticks: 20,
            flow_ticks: 300,
            shield_ticks: 200,
            wall_ticks: 80,
            fragile_ticks: 90,
        }
    }
}
struct Bolt {
    id: u64,
    victims: Vec<Entity>,
}
pub struct Iso {
    pub core: Core,
    pub settings: Settings,
    pub focus: u32,
    pub flow: u32,
    pub shield: bool,
    shield_left: u32,
    shield_epoch: u64,
    contributions: std::collections::HashMap<Entity, u32>,
    pending: Vec<(V3, u32)>,
    orbs: Vec<u64>,
    walls: Vec<u64>,
    bolts: Vec<Bolt>,
    aura: Option<u64>,
}
impl Default for Iso {
    fn default() -> Self {
        Self {
            core: Core::new(DEFINITIONS),
            settings: Settings::default(),
            focus: 0,
            flow: 0,
            shield: false,
            shield_left: 0,
            shield_epoch: 0,
            contributions: Default::default(),
            pending: vec![],
            orbs: vec![],
            walls: vec![],
            bolts: vec![],
            aura: None,
        }
    }
}
impl Agent for Iso {
    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }
    fn owns(&self, id: u64) -> bool {
        self.orbs.contains(&id)
            || self.walls.contains(&id)
            || self.bolts.iter().any(|b| b.id == id)
            || self.aura == Some(id)
    }
    fn busy(&self) -> bool {
        self.core.busy() || self.focus > 0 || self.flow > 0 || self.shield || !self.orbs.is_empty()
    }
    fn stats(&self) -> [u32; 4] {
        [
            self.focus,
            self.flow,
            u32::from(self.shield),
            self.orbs.len() as u32,
        ]
    }
    fn protection(&self, a: &Actor) -> effects::Protection {
        effects::Protection {
            health: a.health,
            one_hit: self.shield,
            shield_epoch: self.shield_epoch,
            ..Default::default()
        }
    }
    fn damage(&mut self, target: Entity) {
        if self.flow > 0
            && (self.contributions.len() < 32 || self.contributions.contains_key(&target))
        {
            self.contributions.insert(target, 200);
        }
    }
    fn fallen(&mut self, target: Entity, at: V3, own: bool) {
        if self.flow > 0
            && (own || self.contributions.contains_key(&target))
            && self.pending.len() < 4
        {
            self.pending.push((add(at, [0., 0.4, 0.]), 100));
        }
        self.contributions.remove(&target);
    }
    fn suppress(&mut self) {
        self.core.cancel();
    }
    fn cancel(&mut self) {
        self.core.cancel();
        self.focus = 0;
        self.flow = 0;
        self.shield = false;
        self.shield_left = 0;
        self.pending.clear();
    }
    fn leave(&mut self) {
        self.cancel();
        self.orbs.clear();
        self.walls.clear();
        self.bolts.clear();
        self.contributions.clear();
        self.aura = None;
    }
    fn activate(&mut self, slot: usize, a: &Actor, _c: &mut Context<'_, impl World>) -> bool {
        if !a.eligible || slot >= 4 {
            return false;
        }
        if slot == 2 {
            if self.focus > 0 || self.flow > 0 || !self.core.spend(2) {
                return false;
            }
            self.core.cancel();
            self.focus = self.settings.focus_ticks.clamp(1, 60);
            return true;
        }
        self.core.equip(slot)
    }
    fn fire(&mut self, _alt: bool, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let (Some(id), Some(slot)) = (owner(a), self.core.equipped) else {
            return false;
        };
        if slot > 1 || self.core.slots[slot].charges == 0 {
            return false;
        }
        let dir = if slot == 0 {
            horizontal(a.direction)
        } else {
            unit(a.direction)
        };
        let at = if slot == 0 {
            add(a.position, scale(dir, 1.))
        } else {
            add(a.eye(), scale(dir, 0.8))
        };
        let kind = if slot == 0 {
            ObjectKind::EnergyWall
        } else {
            ObjectKind::Undercut
        };
        if slot == 0 {
            let temp = Object {
                last_hit: None,
                id: 0,
                owner: id,
                kind,
                position: at,
                health: 1.,
                ticks: 1,
                velocity: dir,
                target: None,
            };
            if !clear_box(c.world, temp.bounds()) {
                return false;
            }
        } else if c.world.boxes(at.map(|v| v.floor() as i32)).is_none() {
            return false;
        }
        let Some(object) = c.object(
            id,
            kind,
            at,
            1.,
            if slot == 0 {
                self.settings.wall_ticks.clamp(20, 160)
            } else {
                32
            },
            dir,
        ) else {
            return false;
        };
        if slot == 0 {
            self.walls.push(object);
        } else {
            self.bolts.push(Bolt {
                id: object,
                victims: vec![],
            });
        }
        self.core.spend(slot);
        self.core.cancel();
        c.world
            .ability_sound(at, "minecraft:block.amethyst_block.chime");
        true
    }
    fn tick(&mut self, a: &Actor, c: &mut Context<'_, impl World>) {
        let Some(id) = owner(a) else {
            return;
        };
        self.shield_left = self.shield_left.saturating_sub(1);
        if self.shield && (!c.world.single_hit_shield(id) || self.shield_left == 0) {
            self.shield = false;
            c.world
                .ability_sound(a.position, "minecraft:block.amethyst_block.break");
        }
        self.flow = self.flow.saturating_sub(1);
        if self.focus > 0 {
            self.focus -= 1;
            if self.focus == 0 {
                self.flow = self.settings.flow_ticks.clamp(20, 400);
                c.world
                    .ability_sound(a.position, "minecraft:block.beacon.activate");
            }
        }
        self.contributions.retain(|_, ticks| {
            *ticks = ticks.saturating_sub(1);
            *ticks > 0
        });
        for (at, ticks) in &mut self.pending {
            *ticks = ticks.saturating_sub(1);
            if *ticks > 0
                && let Some(object) = c.object(id, ObjectKind::IsoOrb, *at, 1., *ticks, [0.; 3])
            {
                self.orbs.push(object);
                *ticks = 0;
            }
        }
        self.pending.retain(|(_, t)| *t > 0);
        self.orbs.retain(|orb| {
            let Some(o) = c.objects.iter().find(|o| o.id == *orb && o.ticks > 0) else {
                return false;
            };
            if o.health <= 0. {
                if o.last_hit.as_ref().is_some_and(|hit| {
                    hit.attacker == Some(a.entity) && hit.cause == objects::UtilityCause::Shot
                }) {
                    self.shield_epoch = self.shield_epoch.wrapping_add(1);
                    self.shield = true;
                    self.shield_left = self.settings.shield_ticks.clamp(20, 300);
                    c.world
                        .ability_sound(a.position, "minecraft:block.amethyst_block.resonate");
                }
                false
            } else {
                true
            }
        });
        if self.shield {
            if let Some(marker) = self
                .aura
                .and_then(|marker| c.objects.iter_mut().find(|o| o.id == marker && o.ticks > 0))
            {
                marker.position = a.position;
                marker.ticks = self.shield_left.max(1);
            } else {
                self.aura = c.object(
                    id,
                    ObjectKind::IsoAura,
                    a.position,
                    1.,
                    self.shield_left.max(1),
                    [0.; 3],
                );
            }
        } else {
            self.aura = None;
        }
        c.world.protect(id, self.protection(a));
        for wall in &self.walls {
            if let Some(o) = c.objects.iter_mut().find(|o| o.id == *wall && o.ticks > 0) {
                let delta = sweep(c.world, o.bounds(), scale(horizontal(o.velocity), 0.12));
                o.position = add(o.position, delta);
            }
        }
        self.walls.retain(|wall| {
            c.objects
                .iter()
                .any(|o| o.id == *wall && o.health > 0. && o.ticks > 0)
        });
        for bolt in &mut self.bolts {
            let Some(mut o) = c
                .objects
                .iter()
                .find(|o| o.id == bolt.id && o.ticks > 0)
                .cloned()
            else {
                bolt.id = 0;
                continue;
            };
            let end = recon::loaded_end(c.world, o.position, o.velocity, 1.);
            if distance(o.position, end) < 0.9 {
                bolt.id = 0;
                continue;
            }
            for t in c.actors {
                if c.budget.actors == 0 || bolt.victims.len() == 32 {
                    break;
                }
                c.budget.actors -= 1;
                if !t.eligible
                    || t.health <= 0.
                    || t.entity == a.entity
                    || c.world.status_immune(t.entity)
                    || bolt.victims.contains(&t.entity)
                {
                    continue;
                }
                if t.bounds.expanded(0.4).segment(o.position, end).is_some() {
                    let ticks = self.settings.fragile_ticks.clamp(10, 160);
                    let status = c.vulnerable.entry(t.entity).or_default();
                    *status = (*status).max(ticks);
                    c.world.vulnerability(t.entity, true);
                    bolt.victims.push(t.entity);
                }
            }
            o.position = end;
            if let Some(stored) = c.objects.iter_mut().find(|o| o.id == bolt.id) {
                *stored = o;
            }
        }
        self.bolts.retain(|b| b.id != 0);
        self.core.slots[2].active =
            self.focus
                .max(self.flow)
                .max(if self.shield { self.shield_left } else { 0 });
    }
}
