use super::*;
use crate::abilities::Kind;
mod creatures;
mod mosh;
mod recovery;
pub const NAMES: [&str; 4] = ["Mosh Pit", "Wingman", "Dizzy", "Thrash"];
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
    pub wing_ticks: u32,
    pub dizzy_ticks: u32,
    pub thrash_ticks: u32,
    pub globule_ticks: u32,
    pub reclaim_ticks: u32,
    pub recovery_ticks: u32,
    pub detain_ticks: u32,
    pub mosh_delay: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            wing_ticks: 120,
            dizzy_ticks: 60,
            thrash_ticks: 160,
            globule_ticks: 200,
            reclaim_ticks: 20,
            recovery_ticks: 80,
            detain_ticks: 80,
            mosh_delay: 40,
        }
    }
}
struct Creature {
    object: u64,
    left: u32,
    next: u32,
    last: V3,
}
struct Bolt {
    object: u64,
    source: u64,
    left: u32,
}
struct Mosh {
    object: u64,
    ground: bool,
    left: u32,
}
struct Globule {
    object: u64,
    slot: usize,
}
struct Pending {
    slot: usize,
    at: V3,
    left: u32,
}
struct Reclaim {
    object: u64,
    slot: usize,
    origin: V3,
    hurt: u64,
    progress: u32,
    lease: u32,
}
pub struct Gekko {
    pub core: Core,
    pub settings: Settings,
    wing: Option<Creature>,
    dizzy: Option<Creature>,
    thrash: Option<companions::Pilot>,
    bolts: Vec<Bolt>,
    moshes: Vec<Mosh>,
    globules: Vec<Globule>,
    pending: Vec<Pending>,
    reclaim: Option<Reclaim>,
    pub thrash_reclaims: u32,
    ult_recovering: bool,
    last_ult_charge: u8,
}
impl Default for Gekko {
    fn default() -> Self {
        Self {
            core: Core::new(DEFINITIONS),
            settings: Settings::default(),
            wing: None,
            dizzy: None,
            thrash: None,
            bolts: vec![],
            moshes: vec![],
            globules: vec![],
            pending: vec![],
            reclaim: None,
            thrash_reclaims: 0,
            ult_recovering: false,
            last_ult_charge: 1,
        }
    }
}
impl Agent for Gekko {
    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }
    fn owns(&self, id: u64) -> bool {
        self.wing.as_ref().is_some_and(|c| c.object == id)
            || self.dizzy.as_ref().is_some_and(|c| c.object == id)
            || self.thrash.as_ref().is_some_and(|c| c.camera.source == id)
            || self.bolts.iter().any(|c| c.object == id)
            || self.moshes.iter().any(|c| c.object == id)
            || self.globules.iter().any(|c| c.object == id)
    }
    fn busy(&self) -> bool {
        self.core.busy()
            || self.wing.is_some()
            || self.dizzy.is_some()
            || self.thrash.is_some()
            || !self.bolts.is_empty()
            || !self.moshes.is_empty()
            || !self.globules.is_empty()
            || !self.pending.is_empty()
    }
    fn equipped(&self) -> bool {
        self.core.equipped.is_some() || self.thrash.is_some() || self.reclaim.is_some()
    }
    fn camera(&self) -> Option<V3> {
        self.thrash.as_ref().map(|t| t.camera.remote.position)
    }
    fn protection(&self, a: &Actor) -> effects::Protection {
        effects::Protection {
            health: a.health,
            disarmed: self.thrash.is_some() || self.reclaim.is_some(),
            ..Default::default()
        }
    }
    fn stats(&self) -> [u32; 4] {
        [
            self.globules.len() as u32,
            self.reclaim.as_ref().map_or(0, |r| r.progress),
            self.thrash_reclaims,
            self.thrash.as_ref().map_or(0, |t| t.left),
        ]
    }
    fn cancel(&mut self) {
        self.core.cancel();
        self.reclaim = None;
        if let Some(t) = self.thrash.take() {
            self.queue(3, add(t.camera.remote.position, [0., -0.35, 0.]));
        }
    }
    fn leave(&mut self) {
        self.cancel();
        self.wing = None;
        self.dizzy = None;
        self.bolts.clear();
        self.moshes.clear();
        self.globules.clear();
        self.pending.clear();
    }
    fn recall_release(&mut self) {
        self.reclaim = None;
    }
    fn recall(&mut self, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        self.begin_reclaim(a, c)
    }
    fn activate(&mut self, slot: usize, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        if !a.eligible || a.health <= 0. || self.reclaim.is_some() {
            return false;
        }
        if let Some(t) = &mut self.thrash {
            return slot == 3 && t.lunge(a.direction, c, 0.75, 0.3);
        }
        self.core.equip(slot)
    }
    fn fire(&mut self, alt: bool, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        if !a.eligible || a.health <= 0. || self.reclaim.is_some() {
            return false;
        }
        if let Some(t) = &mut self.thrash {
            return t.lunge(a.direction, c, 0.75, 0.3);
        }
        let Some(slot) = self.core.equipped else {
            return false;
        };
        if self.core.slots[slot].charges == 0 {
            return false;
        }
        let okay = match slot {
            0 => self.throw_mosh(alt, a, c),
            1 => !alt && self.spawn_wing(a, c),
            2 => self.spawn_dizzy(a, c),
            3 => self.spawn_thrash(a, c),
            _ => false,
        };
        if okay {
            self.core.spend(slot);
            self.core.cancel();
            if slot == 3 {
                self.last_ult_charge = 0;
            }
            c.world
                .ability_sound(a.position, "minecraft:block.amethyst_block.chime");
        }
        okay
    }
    fn tick(&mut self, a: &Actor, c: &mut Context<'_, impl World>) {
        let Some(id) = owner(a) else { return };
        if self.last_ult_charge == 0 && self.core.slots[3].charges > 0 {
            if self.ult_recovering {
                self.ult_recovering = false;
            } else {
                self.thrash_reclaims = 0;
            }
        }
        self.last_ult_charge = self.core.slots[3].charges;
        self.tick_mosh(id, c);
        self.tick_creatures(id, a, c);
        self.tick_recovery(id, a, c);
        self.core.slots[1].active = self.wing.as_ref().map_or(0, |w| w.left);
        self.core.slots[2].active = self.dizzy.as_ref().map_or(0, |d| d.left);
        self.core.slots[3].active = self.thrash.as_ref().map_or(0, |t| t.left);
    }
}
