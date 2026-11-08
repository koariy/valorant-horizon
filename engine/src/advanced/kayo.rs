use super::*;
use crate::abilities::Kind;
pub const NAMES: [&str; 4] = ["FRAG/MENT", "FLASH/DRIVE", "ZERO/POINT", "NULL/CMD"];
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
    pub suppression_ticks: u32,
    pub ultimate_ticks: u32,
    pub pulse_interval: u32,
    pub downed_ticks: u32,
    pub stabilize_ticks: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            suppression_ticks: 120,
            ultimate_ticks: 240,
            pulse_interval: 40,
            downed_ticks: 200,
            stabilize_ticks: 60,
        }
    }
}
enum Payload {
    Flash { fuse: u32, alt: bool },
    Blade { windup: Option<u32> },
    Fragment { ground: bool, next: u32, pulses: u8 },
}
struct Grenade {
    object: u64,
    payload: Payload,
}
pub enum Overload {
    Off,
    Active { ticks: u32, next: u32 },
    Downed { ticks: u32, anchor: V3 },
    PendingFatal,
}
pub struct Kayo {
    pub core: Core,
    pub settings: Settings,
    pub overload: Overload,
    pub pool: f32,
    epoch: u64,
    owner: Option<u128>,
    native_id: Option<i32>,
    grenades: Vec<Grenade>,
    marker: Option<u64>,
    affected: u32,
}
impl Default for Kayo {
    fn default() -> Self {
        Self {
            core: Core::new(DEFINITIONS),
            settings: Settings::default(),
            overload: Overload::Off,
            pool: 0.,
            epoch: 0,
            owner: None,
            native_id: None,
            grenades: vec![],
            marker: None,
            affected: 0,
        }
    }
}
impl Kayo {
    pub fn downed(&self) -> bool {
        matches!(self.overload, Overload::Downed { .. }) && self.pool > 0.
    }
    pub fn stabilize(&mut self) -> bool {
        if !self.downed() {
            return false;
        }
        self.overload = Overload::Off;
        self.pool = 0.;
        self.native_id = None;
        self.marker = None;
        self.core.cancel();
        true
    }
    fn pulse(&mut self, id: u128, at: V3, range: f64, c: &mut Context<'_, impl World>) {
        self.affected = 0;
        for t in c.enemies(id, at, range) {
            if matches!(t.entity, Entity::Player(_)) && !c.world.status_immune(t.entity) {
                control::suppress(
                    c.controls,
                    c.world,
                    t.entity,
                    self.settings.suppression_ticks.clamp(20, 200),
                );
                self.affected += 1;
            }
        }
        c.world
            .ability_sound(at, "minecraft:block.beacon.deactivate");
        c.world.burst_fx(at);
    }
    fn utilities(&mut self, id: u128, c: &mut Context<'_, impl World>) {
        let mut pulses = vec![];
        for grenade in &mut self.grenades {
            let Some(mut o) = c
                .objects
                .iter()
                .find(|o| o.id == grenade.object && o.health > 0. && o.ticks > 0)
                .cloned()
            else {
                grenade.object = 0;
                continue;
            };
            match &mut grenade.payload {
                Payload::Flash { fuse, alt } => {
                    match throwable::advance(c.world, &mut o, 0.035, 0.08) {
                        throwable::Flight::Unknown => {
                            grenade.object = 0;
                            continue;
                        }
                        throwable::Flight::Surface(n) => throwable::reflect(&mut o, n, 0.55),
                        _ => {}
                    }
                    *fuse = fuse.saturating_sub(1);
                    if *fuse == 0 {
                        c.flash(o.position, 12., if *alt { 25 } else { 60 });
                        c.world
                            .ability_sound(o.position, "minecraft:entity.firework_rocket.blast");
                        o.health = 0.;
                        grenade.object = 0;
                    }
                }
                Payload::Blade { windup } => {
                    if let Some(left) = windup {
                        *left = left.saturating_sub(1);
                        if *left == 0 {
                            pulses.push(o.position);
                            o.health = 0.;
                            grenade.object = 0;
                        }
                    } else {
                        match throwable::advance(c.world, &mut o, 0.015, 0.07) {
                            throwable::Flight::Unknown => {
                                grenade.object = 0;
                                continue;
                            }
                            throwable::Flight::Surface(_) => {
                                o.velocity = [0.; 3];
                                *windup = Some(20);
                            }
                            _ => {}
                        }
                    }
                }
                Payload::Fragment {
                    ground,
                    next,
                    pulses,
                } => {
                    if *ground
                        && !block_collision(
                            c.world,
                            o.position,
                            add(o.position, [0., -0.4, 0.]),
                            0.,
                        )
                        .is_some_and(|h| h.known && h.normal[1] > 0.5)
                    {
                        *ground = false;
                    }
                    if !*ground {
                        match throwable::advance(c.world, &mut o, 0.04, 0.1) {
                            throwable::Flight::Unknown => {
                                grenade.object = 0;
                                continue;
                            }
                            throwable::Flight::Surface(n) if n[1] > 0.5 => {
                                *ground = true;
                                o.velocity = [0.; 3];
                            }
                            throwable::Flight::Surface(n) => throwable::reflect(&mut o, n, 0.5),
                            _ => {}
                        }
                    }
                    if *ground {
                        *next = next.saturating_sub(1);
                        if *next == 0 {
                            let targets: Vec<_> = c
                                .actors
                                .iter()
                                .filter(|a| {
                                    a.eligible
                                        && a.health > 0.
                                        && distance(a.position, o.position) < 3.5
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
                                    let damage = (14.
                                        - distance(target.position, o.position) as f32 * 3.)
                                        .max(3.);
                                    c.hurt_with(id, target.entity, damage, "minecraft:explosion");
                                }
                            }
                            c.world.burst_fx(o.position);
                            c.world
                                .ability_sound(o.position, "minecraft:entity.generic.explode");
                            *pulses += 1;
                            *next = 15;
                            if *pulses >= 4 {
                                o.health = 0.;
                                grenade.object = 0;
                            }
                        }
                    }
                }
            }
            if let Some(stored) = c.objects.iter_mut().find(|x| x.id == o.id) {
                *stored = o;
            }
        }
        self.grenades.retain(|g| g.object != 0);
        for at in pulses {
            self.pulse(id, at, 10., c);
        }
    }
}
impl Agent for Kayo {
    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }
    fn owns(&self, id: u64) -> bool {
        self.marker == Some(id) || self.grenades.iter().any(|g| g.object == id)
    }
    fn busy(&self) -> bool {
        self.core.busy() || !matches!(self.overload, Overload::Off) || !self.grenades.is_empty()
    }
    fn equipped(&self) -> bool {
        self.core.equipped.is_some()
            || matches!(
                self.overload,
                Overload::Downed { .. } | Overload::PendingFatal
            )
    }
    fn cancel(&mut self) {
        self.core.cancel();
    }
    fn leave(&mut self) {
        // A lethal native hit and disconnect can occur before the next kit tick.
        // Match the unique ultimate grant, so a reused raw entity id cannot inherit it.
        if let Some(native) = self.native_id.and_then(|id| {
            effects::protections()
                .lock()
                .ok()
                .and_then(|all| all.get(&id).copied())
        }) && native.down_epoch == self.epoch
            && native.downed
        {
            self.pool = native.downed_pool;
            self.overload = Overload::PendingFatal;
        }
        self.core.cancel();
        self.grenades.clear();
        self.marker = None;
        self.overload = if matches!(
            self.overload,
            Overload::Downed { .. } | Overload::PendingFatal
        ) {
            Overload::PendingFatal
        } else {
            Overload::Off
        };
    }
    fn fallen(&mut self, target: Entity, _at: V3, _own: bool) {
        if self.owner.is_some_and(|id| target == Entity::Player(id)) {
            self.overload = Overload::Off;
            self.pool = 0.;
            self.marker = None;
            self.native_id = None;
        }
    }
    fn stats(&self) -> [u32; 4] {
        [
            match self.overload {
                Overload::Active { ticks, .. } => ticks,
                _ => 0,
            },
            self.affected,
            match self.overload {
                Overload::Downed { ticks, .. } => ticks,
                Overload::PendingFatal => 1,
                _ => 0,
            },
            self.pool.max(0.) as u32,
        ]
    }
    fn protection(&self, a: &Actor) -> effects::Protection {
        effects::Protection {
            health: a.health,
            down_epoch: if matches!(
                self.overload,
                Overload::Active { .. } | Overload::Downed { .. }
            ) {
                self.epoch
            } else {
                0
            },
            downable: matches!(self.overload, Overload::Active { .. }),
            downed: matches!(self.overload, Overload::Downed { .. }),
            downed_pool: self.pool,
            disarmed: matches!(
                self.overload,
                Overload::Downed { .. } | Overload::PendingFatal
            ),
            ..Default::default()
        }
    }
    fn activate(&mut self, slot: usize, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Some(id) = owner(a).filter(|_| a.eligible && a.health > 0.) else {
            return false;
        };
        if slot >= 4 || self.downed() || matches!(self.overload, Overload::PendingFatal) {
            return false;
        }
        if slot != 3 {
            return self.core.equip(slot);
        }
        if !matches!(self.overload, Overload::Off) || self.core.slots[3].charges == 0 {
            return false;
        }
        let ticks = self.settings.ultimate_ticks.clamp(40, 400);
        let Some(marker) = c.object(id, ObjectKind::KayoCore, a.position, 1., ticks + 1, [0.; 3])
        else {
            return false;
        };
        self.marker = Some(marker);
        self.owner = Some(id);
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        self.epoch = NEXT
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .max(1);
        self.native_id = c.world.network_id(a.entity).map(|id| id as i32);
        self.overload = Overload::Active { ticks, next: 0 };
        self.core.spend(3);
        self.core.cancel();
        c.world.protect(id, self.protection(a));
        true
    }
    fn fire(&mut self, alt: bool, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let (Some(id), Some(slot)) = (owner(a), self.core.equipped) else {
            return false;
        };
        if !a.eligible
            || slot > 2
            || self.downed()
            || matches!(self.overload, Overload::PendingFatal)
            || self.core.slots[slot].charges == 0
        {
            return false;
        }
        let kind = match slot {
            0 => ObjectKind::Fragment,
            1 => ObjectKind::FlashDrive,
            _ => ObjectKind::SuppressionBlade,
        };
        let velocity = scale(unit(a.direction), if alt { 0.35 } else { 0.8 });
        let Some(object) = c.object(
            id,
            kind,
            a.eye(),
            if slot == 2 { 2. } else { 1. },
            160,
            velocity,
        ) else {
            return false;
        };
        self.grenades.push(Grenade {
            object,
            payload: match slot {
                0 => Payload::Fragment {
                    ground: false,
                    next: 0,
                    pulses: 0,
                },
                1 => Payload::Flash {
                    fuse: if alt { 15 } else { 30 },
                    alt,
                },
                _ => Payload::Blade { windup: None },
            },
        });
        self.core.spend(slot);
        self.core.cancel();
        c.world
            .ability_sound(a.eye(), "minecraft:entity.snowball.throw");
        true
    }
    fn tick(&mut self, a: &Actor, c: &mut Context<'_, impl World>) {
        let Some(id) = owner(a) else { return };
        self.utilities(id, c);
        let native = c.world.protection_state(id);
        if matches!(self.overload, Overload::Active { .. }) && native.downed {
            self.pool = native.downed_pool;
            self.overload = Overload::Downed {
                ticks: self.settings.downed_ticks.clamp(40, 400),
                anchor: a.position,
            };
            self.core.cancel();
            c.world
                .ability_sound(a.position, "minecraft:block.anvil.land");
        }
        let mut pulse = false;
        match &mut self.overload {
            Overload::Active { ticks, next } => {
                *ticks = ticks.saturating_sub(1);
                *next = next.saturating_sub(1);
                if *next == 0 {
                    pulse = true;
                    *next = self.settings.pulse_interval.clamp(10, 80);
                }
                c.world.effect(a.entity, "minecraft:speed", 2, 1);
                c.world.effect(a.entity, "minecraft:haste", 2, 1);
                if *ticks == 0 {
                    self.overload = Overload::Off;
                    self.marker = None;
                }
            }
            Overload::Downed { ticks, anchor } => {
                self.pool = native.downed_pool;
                *ticks = ticks.saturating_sub(1);
                c.world.move_player(id, *anchor);
                c.world.velocity(id, [0.; 3], false);
                if *ticks == 0 || self.pool <= 0. {
                    self.overload = Overload::PendingFatal;
                }
            }
            _ => {}
        }
        if pulse {
            self.pulse(id, a.position, 12., c);
        }
        if matches!(self.overload, Overload::PendingFatal) {
            c.world.protect(id, effects::Protection::default());
            let hit = c
                .world
                .hurt_with(id, a.entity, 10000., "minecraft:generic_kill");
            if hit.killed {
                self.overload = Overload::Off;
                self.pool = 0.;
                self.marker = None;
            }
        }
        if let Some(marker) = self
            .marker
            .and_then(|n| c.objects.iter_mut().find(|o| o.id == n))
        {
            marker.position = a.position;
            marker.ticks = match self.overload {
                Overload::Active { ticks, .. } | Overload::Downed { ticks, .. } => ticks + 1,
                _ => 0,
            };
        }
        self.core.slots[3].active = self.stats()[0].max(self.stats()[2]);
    }
}
