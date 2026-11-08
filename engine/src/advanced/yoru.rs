use super::*;
use crate::abilities::Kind;
pub const NAMES: [&str; 4] = ["Fakeout", "Blindside", "Gatecrash", "Dimensional Drift"];
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
    pub echo_ticks: u32,
    pub echo_speed: f64,
    pub gate_ticks: u32,
    pub gate_speed: f64,
    pub phase_ticks: u32,
    pub teleport_ticks: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            echo_ticks: 100,
            echo_speed: 0.2,
            gate_ticks: 400,
            gate_speed: 0.2,
            phase_ticks: 200,
            teleport_ticks: 8,
        }
    }
}
struct Echo {
    id: u64,
    moving: bool,
    direction: V3,
}
struct Fragment {
    id: u64,
    armed: Option<u32>,
}
struct Gate {
    id: u64,
    moving: bool,
}
struct Cast {
    at: V3,
    origin: V3,
    ticks: u32,
}
struct Detonation {
    enemy_only: bool,
    at: V3,
    direction: V3,
    ticks: u32,
}
pub struct Yoru {
    pub core: Core,
    pub settings: Settings,
    pub phase: u32,
    echoes: Vec<Echo>,
    fragments: Vec<Fragment>,
    gate: Option<Gate>,
    cast: Option<Cast>,
    flashes: Vec<Detonation>,
}
impl Default for Yoru {
    fn default() -> Self {
        Self {
            core: Core::new(DEFINITIONS),
            settings: Settings::default(),
            phase: 0,
            echoes: vec![],
            fragments: vec![],
            gate: None,
            cast: None,
            flashes: vec![],
        }
    }
}
fn cue(at: V3, c: &mut Context<'_, impl World>) {
    c.world
        .ability_sound(at, "minecraft:entity.enderman.teleport");
    if c.beams.len() < recon::MAX_BEAMS {
        c.beams.push(Beam {
            from: at,
            to: add(at, [0., 1.8, 0.]),
            ticks: 4,
        });
    }
}
impl Yoru {
    fn echoes(&mut self, id: u128, c: &mut Context<'_, impl World>) {
        for echo in &mut self.echoes {
            let Some(mut o) = c.objects.iter().find(|o| o.id == echo.id).cloned() else {
                echo.id = 0;
                continue;
            };
            if o.ticks == 0 {
                echo.id = 0;
                continue;
            }
            if o.health <= 0. {
                if echo.moving
                    && let Some(mark) = &o.last_hit
                    && let Some(attacker) = mark.attacker
                    && attacker != Entity::Player(id)
                    && !matches!(attacker,Entity::Player(p) if c.world.intel_viewers(id).contains(&p))
                {
                    let at = add(o.position, [0., 1.3, 0.]);
                    let from = c
                        .actors
                        .iter()
                        .find(|a| a.entity == attacker)
                        .map_or(mark.from, Actor::eye);
                    let direction = unit(sub(from, at));
                    self.flashes.push(Detonation {
                        at,
                        direction: if length(direction) > 0.5 {
                            direction
                        } else {
                            scale(echo.direction, -1.)
                        },
                        ticks: 12,
                        enemy_only: true,
                    });
                }
                echo.id = 0;
                continue;
            }
            if echo.moving {
                let previous = o.position;
                walking::step(
                    c.world,
                    &mut o,
                    echo.direction,
                    self.settings.echo_speed.clamp(0.05, 0.3),
                );
                if distance(previous, o.position) > 0.05 && o.ticks.is_multiple_of(8) {
                    c.world
                        .ability_sound(o.position, "minecraft:block.stone.step");
                }
            } else {
                walking::step(c.world, &mut o, [0.; 3], 0.);
            }
            if let Some(stored) = c.objects.iter_mut().find(|o| o.id == echo.id) {
                *stored = o;
            }
        }
        self.echoes.retain(|e| e.id != 0);
    }
    fn fragments(&mut self, c: &mut Context<'_, impl World>) {
        for f in &mut self.fragments {
            let Some(o) = c
                .objects
                .iter_mut()
                .find(|o| o.id == f.id && o.health > 0. && o.ticks > 0)
            else {
                f.id = 0;
                continue;
            };
            let end = add(o.position, o.velocity);
            if let Some(hit) = block_collision(c.world, o.position, end, 0.08) {
                if !hit.known {
                    o.health = 0.;
                    f.id = 0;
                    continue;
                }
                o.position = add(
                    add(o.position, scale(o.velocity, hit.time)),
                    scale(hit.normal, 0.12),
                );
                if f.armed.is_none() {
                    o.velocity = scale(
                        sub(
                            o.velocity,
                            scale(hit.normal, 2. * dot(o.velocity, hit.normal)),
                        ),
                        0.6,
                    );
                    f.armed = Some(12);
                } else {
                    o.velocity = [0.; 3];
                }
            } else {
                o.position = end;
            }
            if let Some(timer) = &mut f.armed {
                *timer = timer.saturating_sub(1);
                if *timer == 0 {
                    self.flashes.push(Detonation {
                        at: o.position,
                        direction: [0.; 3],
                        ticks: 0,
                        enemy_only: false,
                    });
                    o.health = 0.;
                    f.id = 0;
                }
            }
        }
        self.fragments.retain(|f| f.id != 0);
    }
    fn gate(&mut self, id: u128, a: &Actor, c: &mut Context<'_, impl World>) {
        if self.gate.is_none() {
            self.cast = None;
            return;
        }
        if let Some(gate) = &mut self.gate {
            let Some(mut o) = c
                .objects
                .iter()
                .find(|o| o.id == gate.id && o.health > 0. && o.ticks > 0)
                .cloned()
            else {
                self.gate = None;
                self.cast = None;
                return;
            };
            if distance(a.position, o.position) > 64. {
                self.gate = None;
                self.cast = None;
                return;
            }
            if gate.moving && self.cast.is_none() {
                let dir = horizontal(o.velocity);
                walking::step(
                    c.world,
                    &mut o,
                    dir,
                    self.settings.gate_speed.clamp(0.05, 0.3),
                );
            } else if self.cast.is_none() {
                walking::step(c.world, &mut o, [0.; 3], 0.);
            }
            if let Some(stored) = c.objects.iter_mut().find(|o| o.id == gate.id) {
                *stored = o;
            }
        }
        if let Some(cast) = &mut self.cast {
            if distance(a.position, cast.origin) > 1. {
                self.cast = None;
                return;
            }
            c.world.velocity(id, [0., a.velocity[1], 0.], false);
            cast.ticks = cast.ticks.saturating_sub(1);
            if cast.ticks == 0 {
                let at = cast.at;
                self.cast = None;
                if teleport::safe(c.world, a, at) {
                    cue(a.position, c);
                    cue(at, c);
                    if let Some(gate) = self.gate.take() {
                        c.objects.retain(|o| o.id != gate.id);
                    }
                }
            }
        }
    }
}
impl Agent for Yoru {
    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }
    fn busy(&self) -> bool {
        self.core.busy()
            || self.phase > 0
            || self.cast.is_some()
            || !self.echoes.is_empty()
            || self.gate.is_some()
    }
    fn equipped(&self) -> bool {
        self.core.equipped.is_some() || self.phase > 0 || self.cast.is_some()
    }
    fn phased(&self) -> bool {
        self.phase > 0
    }
    fn hidden(&self) -> bool {
        self.phase > 0
    }
    fn protection(&self, a: &Actor) -> effects::Protection {
        effects::Protection {
            health: a.health,
            invulnerable: self.phase > 0,
            disarmed: self.phase > 0 || self.cast.is_some(),
            ..Default::default()
        }
    }
    fn owns(&self, id: u64) -> bool {
        self.echoes.iter().any(|e| e.id == id)
            || self.fragments.iter().any(|f| f.id == id)
            || self.gate.as_ref().is_some_and(|g| g.id == id)
    }
    fn stats(&self) -> [u32; 4] {
        [
            self.echoes.len() as u32,
            u32::from(self.gate.is_some()),
            self.cast.as_ref().map_or(0, |c| c.ticks),
            self.phase,
        ]
    }
    fn cancel(&mut self) {
        self.core.cancel();
        self.phase = 0;
        self.cast = None;
    }
    fn leave(&mut self) {
        self.cancel();
        self.echoes.clear();
        self.fragments.clear();
        self.gate = None;
        self.flashes.clear();
    }
    fn activate(&mut self, slot: usize, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Some(id) = owner(a) else {
            return false;
        };
        if !a.eligible || slot >= 4 {
            return false;
        }
        if slot == 3 && self.phase > 0 {
            self.phase = 0;
            self.core.cancel();
            c.world.protect(id, self.protection(a));
            return true;
        }
        if self.cast.is_some() {
            return false;
        }
        if slot == 2
            && let Some(g) = &self.gate
        {
            let Some(o) = c
                .objects
                .iter()
                .find(|o| o.id == g.id && o.health > 0. && o.ticks > 0)
            else {
                return false;
            };
            if distance(a.position, o.position) > 64. {
                return false;
            }
            self.core.cancel();
            self.cast = Some(Cast {
                at: o.position,
                origin: a.position,
                ticks: self.settings.teleport_ticks.clamp(1, 20),
            });
            c.world.protect(id, self.protection(a));
            return true;
        }
        self.core.equip(slot)
    }
    fn fire(&mut self, alt: bool, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let (Some(id), Some(slot)) = (owner(a), self.core.equipped) else {
            return false;
        };
        if !a.eligible || self.cast.is_some() || self.core.slots[slot].charges == 0 {
            return false;
        }
        if slot == 3 {
            if self.phase > 0 || !self.core.spend(3) {
                return false;
            }
            self.core.cancel();
            self.phase = self.settings.phase_ticks.clamp(20, 300);
            c.world.protect(id, self.protection(a));
            c.world
                .quiet_effect(a.entity, "minecraft:invisibility", 3, 0);
            return true;
        }
        let dir = horizontal(a.direction);
        let at = if slot == 1 {
            add(a.eye(), scale(a.direction, 0.4))
        } else {
            add(a.position, scale(dir, 0.8))
        };
        let kind = match slot {
            0 => {
                if alt {
                    ObjectKind::DormantEcho
                } else {
                    ObjectKind::Echo
                }
            }
            1 => ObjectKind::Blindside,
            _ => ObjectKind::Gatecrash,
        };
        let shape = Box3 {
            min: add(at, [-0.3, 0., -0.3]),
            max: add(at, [0.3, if slot == 0 && !alt { 1.8 } else { 0.2 }, 0.3]),
        };
        if !clear_box(c.world, shape) || c.objects.len() >= MAX_OBJECTS {
            return false;
        }
        let ticks = match slot {
            0 => {
                if alt {
                    600
                } else {
                    self.settings.echo_ticks.clamp(20, 200)
                }
            }
            1 => 80,
            _ => self.settings.gate_ticks.clamp(40, 600),
        };
        let Some(object) = c.object(
            id,
            kind,
            at,
            if slot == 2 { 8. } else { 1. },
            ticks,
            if slot == 1 {
                scale(a.direction, 0.7)
            } else {
                scale(dir, if alt { 0. } else { 0.2 })
            },
        ) else {
            return false;
        };
        match slot {
            0 => self.echoes.push(Echo {
                id: object,
                moving: !alt,
                direction: dir,
            }),
            1 => self.fragments.push(Fragment {
                id: object,
                armed: None,
            }),
            _ => {
                self.gate = Some(Gate {
                    id: object,
                    moving: !alt,
                })
            }
        }
        self.core.spend(slot);
        self.core.cancel();
        true
    }
    fn recall(&mut self, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let target = self
            .echoes
            .iter()
            .filter(|e| !e.moving)
            .find(|e| {
                c.objects.iter().find(|o| o.id == e.id).is_some_and(|o| {
                    distance(a.eye(), o.position) < 32.
                        && dot(a.direction, unit(sub(o.position, a.eye()))) > 0.85
                })
            })
            .map(|e| e.id);
        if let Some(id) = target {
            let Some(mut o) = c
                .objects
                .iter()
                .find(|o| o.id == id && o.health > 0.)
                .cloned()
            else {
                return false;
            };
            if !c.visible(a.eye(), add(o.position, [0., 0.1, 0.])) {
                return false;
            }
            o.kind = ObjectKind::Echo;
            if !clear_box(c.world, o.bounds()) {
                return false;
            }
            o.ticks = self.settings.echo_ticks.clamp(20, 200);
            if let Some(e) = self.echoes.iter_mut().find(|e| e.id == id) {
                e.moving = true;
                o.velocity = scale(e.direction, self.settings.echo_speed.clamp(0.05, 0.3));
            }
            if let Some(stored) = c.objects.iter_mut().find(|o| o.id == id) {
                *stored = o;
            }
            return true;
        }
        if let Some(gate) = self.gate.take()
            && let Some(o) = c
                .objects
                .iter()
                .find(|o| o.id == gate.id && o.health > 0. && o.ticks > 0)
                .cloned()
        {
            cue(a.position, c);
            cue(o.position, c);
            c.objects.retain(|o| o.id != gate.id);
            self.cast = None;
            return true;
        }
        false
    }
    fn tick(&mut self, a: &Actor, c: &mut Context<'_, impl World>) {
        let Some(id) = owner(a) else {
            return;
        };
        self.phase = self.phase.saturating_sub(1);
        self.core.slots[3].active = self.phase;
        c.world.protect(id, self.protection(a));
        if self.phase > 0 {
            c.world
                .quiet_effect(a.entity, "minecraft:invisibility", 3, 0);
            c.world.quiet_effect(a.entity, "minecraft:speed", 3, 1);
        }
        self.echoes(id, c);
        self.fragments(c);
        self.gate(id, a, c);
        for f in &mut self.flashes {
            f.ticks = f.ticks.saturating_sub(1);
            if f.ticks == 0 {
                if f.enemy_only {
                    c.enemy_flash_cone(id, f.at, f.direction, 0.3, 16., 50);
                } else {
                    c.flash(f.at, 16., 50);
                }
                c.world
                    .ability_sound(f.at, "minecraft:entity.enderman.scream");
            }
        }
        self.flashes.retain(|f| f.ticks > 0);
    }
}
