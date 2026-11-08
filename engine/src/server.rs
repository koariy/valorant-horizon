use crate::{
    agents::jett::Jett,
    gameplay::{geometry::*, projectiles::*, smoke::Smoke, *},
};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Mutex, OnceLock},
};
pub const CHANNEL: &str = "valorant_horizon:abilities";
#[derive(Default)]
pub struct Player {
    pub agent: usize,
    pub extra: crate::advanced::Expansion,
    pub sova: crate::agents::sova::Sova,
    pub clove: crate::agents::clove::Clove,
    pub harbor: crate::agents::harbor::Harbor,
    pub astra: crate::agents::astra::Astra,
    pub viper: crate::agents::viper::Viper,
    pub brimstone: crate::agents::brimstone::Brimstone,
    pub phoenix: crate::agents::phoenix::Phoenix,
    pub sage: crate::agents::sage::Sage,
    pub omen: crate::agents::omen::Omen,
    pub reyna: crate::agents::reyna::Reyna,
    pub raze: crate::agents::raze::Raze,
    pub strike_delay: u32,
    pub jett: Jett,
    pub sequence: u64,
    pub connection: u32,
    pub error: Option<String>,
}
impl Player {
    pub fn change_dimension(&mut self) {
        self.cancel();
        self.phoenix.anchor = None;
        self.viper.cloud = None;
        self.viper.screen = None;
        self.viper.placed_ticks = 0;
        self.reyna.souls.clear();
        self.astra
            .returns
            .extend(self.astra.stars.drain(..).map(|_| 80));
    }

    pub fn busy(&self) -> bool {
        self.extra.busy()
            || self.sova.busy()
            || self.clove.busy()
            || self.harbor.busy()
            || self.astra.busy()
            || self.viper.busy()
            || self.brimstone.busy()
            || self.jett.busy()
            || self.phoenix.busy()
            || self.sage.busy()
            || self.omen.busy()
            || self.raze.busy()
            || self.reyna.busy()
    }
    pub fn cancel(&mut self) {
        self.extra.leave();
        self.sova.cancel();
        self.clove.leave();
        self.harbor.cancel();
        self.astra.cancel();
        self.viper.cancel();
        self.brimstone.cancel();
        self.jett.cancel();
        self.phoenix.cancel();
        self.sage.cancel();
        self.omen.cancel();
        self.raze.cancel();
        self.reyna.cancel();
    }
}
#[derive(Default)]
pub struct Session {
    pub players: HashMap<u128, Player>,
    pub corpses: Vec<crate::advanced::Corpse>,
    pub budget: crate::advanced::Budget,
    pub extra_kills: Vec<(u128, Entity)>,
    pub hidden: Vec<(u128, u32)>,
    pub viewers: HashMap<u128, Vec<u128>>,
    pub intel: recon::Intel,
    pub beams: Vec<recon::Beam>,
    pub toxins: Vec<toxin::Field>,
    pub shields: Vec<shields::Shield>,
    pub divides: Vec<crate::gameplay::divide::Divide>,
    pub vulnerable: HashMap<Entity, u32>,
    pub decay: HashMap<Entity, toxin::Decay>,
    pub decay_loaded: bool,
    pub projectiles: Vec<Projectile>,
    pub smokes: Vec<Smoke>,
    pub zones: Vec<crate::gameplay::zones::Zone>,
    pub objects: Vec<objects::Object>,
    pub next_object: u64,
    pub bursts: Vec<explosions::Explosion>,
    pub vision: HashMap<u128, effects::Vision>,
    pub blocks: Vec<barrier::OwnedBlock>,
    pub blocks_loaded: bool,
    pub blocks_dirty: bool,
    pub requests: VecDeque<(u128, Request)>,
    pub positions: HashMap<Entity, V3>,
    pub pending_deaths: Vec<(u128, Entity, V3, bool)>,
    pub credited: VecDeque<Entity>,
}
pub type Key = (u64, String);
pub fn sessions() -> &'static Mutex<HashMap<Key, Session>> {
    static S: OnceLock<Mutex<HashMap<Key, Session>>> = OnceLock::new();
    S.get_or_init(Default::default)
}
impl Session {
    pub fn enqueue(&mut self, owner: u128, request: Request) {
        if self.requests.len() < 256
            && self.requests.iter().filter(|(id, _)| *id == owner).count() < 8
        {
            self.requests.push_back((owner, request));
        }
    }
    pub fn tick(&mut self, w: &mut impl World) {
        self.budget = Default::default();
        for corpse in &mut self.corpses {
            corpse.ticks = corpse.ticks.saturating_sub(1);
        }
        self.corpses.retain(|x| x.ticks > 0);
        self.intel.tick(w);
        for b in &mut self.beams {
            b.ticks = b.ticks.saturating_sub(1);
        }
        self.beams.retain(|b| b.ticks > 0);
        self.decay_loaded = false;
        if let Ok(entries) = w.saved_decay() {
            self.decay = entries.into_iter().collect();
            self.decay_loaded = true;
        }
        let actors = w.actors();
        for (owner, target) in w.hit_credits() {
            if let Some(p) = self.players.get_mut(&owner).filter(|p| p.agent == 10) {
                p.clove.damage(target);
            }
        }
        self.positions = actors.iter().map(|a| (a.entity, a.eye())).collect();
        self.credited
            .retain(|entity| !actors.iter().any(|a| a.entity == *entity && a.health > 0.));
        for (owner, target, at, primary) in std::mem::take(&mut self.pending_deaths) {
            if actors.iter().any(|a| a.entity == target && a.health > 0.) {
                continue;
            }
            if !self.credited.contains(&target) {
                for (id, p) in &mut self.players {
                    if p.agent == 10 {
                        p.clove.fallen(target, *id == owner);
                    }
                }
                if self.corpses.len() >= crate::advanced::MAX_CORPSES {
                    self.corpses.remove(0);
                }
                self.corpses.push(crate::advanced::Corpse {
                    target,
                    killer: owner,
                    at,
                    ticks: 200,
                    used: false,
                });
                self.credit_kill(owner, target);
                if let Some(p) = self.players.get_mut(&owner) {
                    if p.agent == 5 {
                        p.reyna.kill(at);
                    }
                    if primary && p.agent == 0 && p.jett.slots[3].active > 0 {
                        p.jett.knives = 5;
                    }
                }
            }
        }

        if !self.blocks_loaded
            && let Ok(blocks) = w.saved_blocks()
        {
            self.blocks = blocks;
            self.blocks_loaded = true;
        }
        if self.blocks_dirty && w.save_blocks(&self.blocks) {
            self.blocks_dirty = false;
        }
        barrier::cleanup(w, &mut self.blocks);
        // Retain spent charges across disconnects and dimension changes.
        self.players
            .retain(|id, p| actors.iter().any(|a| a.entity == Entity::Player(*id)) || p.busy());
        for (id, p) in &mut self.players {
            if !actors.iter().any(|a| a.entity == Entity::Player(*id)) {
                p.cancel();
                let offline = Actor {
                    entity: Entity::Player(*id),
                    position: [0.; 3],
                    bounds: Box3 {
                        min: [0.; 3],
                        max: [0.; 3],
                    },
                    velocity: [0.; 3],
                    direction: [0.; 3],
                    movement: [0.; 3],
                    health: 0.,
                    max_health: 20.,
                    ground: false,
                    jump: false,
                    eligible: false,
                };
                if p.agent >= 12 {
                    p.extra.tick(
                        p.agent,
                        &offline,
                        &mut crate::advanced::Context {
                            world: w,
                            actors: &actors,
                            objects: &mut self.objects,
                            next: &mut self.next_object,
                            projectiles: &mut self.projectiles,
                            zones: &mut self.zones,
                            smokes: &mut self.smokes,
                            intel: &mut self.intel,
                            beams: &mut self.beams,
                            bursts: &mut self.bursts,
                            vision: &mut self.vision,
                            corpses: &mut self.corpses,
                            kills: &mut self.extra_kills,
                            shields: &mut self.shields,
                            divides: &self.divides,
                            decay: self.decay_loaded.then_some(&mut self.decay),
                            budget: &mut self.budget,
                        },
                    );
                }
                p.sova.offline();
                p.clove.offline();
                p.viper.tick(&offline, 0);
                p.astra.tick(&offline, w);
                p.harbor.tick(&offline, w, &mut self.zones);
                p.brimstone.tick(false);
                for (r, d) in p
                    .jett
                    .slots
                    .iter_mut()
                    .zip(crate::agents::jett::DEFINITIONS)
                {
                    r.tick(d);
                }
                for (r, d) in p
                    .omen
                    .slots
                    .iter_mut()
                    .zip(crate::agents::omen::DEFINITIONS)
                {
                    r.tick(d);
                }
                for (r, d) in p
                    .raze
                    .slots
                    .iter_mut()
                    .zip(crate::agents::raze::DEFINITIONS)
                {
                    r.tick(d);
                }
                for (r, d) in p
                    .reyna
                    .slots
                    .iter_mut()
                    .zip(crate::agents::reyna::DEFINITIONS)
                {
                    r.tick(d);
                }
                p.sage.cancel();
                for (r, d) in p
                    .sage
                    .slots
                    .iter_mut()
                    .zip(crate::agents::sage::DEFINITIONS)
                {
                    r.tick(d);
                }
                for (r, d) in p
                    .phoenix
                    .slots
                    .iter_mut()
                    .zip(crate::agents::phoenix::DEFINITIONS)
                {
                    r.tick(d);
                }
                p.phoenix.anchor = None;
            }
        }
        while let Some((id, r)) = self.requests.pop_front() {
            let Some(a) = actors.iter().find(|a| {
                a.entity == Entity::Player(id)
                    && (a.eligible
                        || (a.health <= 0.
                            && self.players.get(&id).is_some_and(|p| p.agent == 10)
                            && matches!(
                                r.action,
                                Action::Activate(2)
                                    | Action::Fire(_)
                                    | Action::Cancel
                                    | Action::Aim(_, _)
                                    | Action::Release(_)
                            )))
            }) else {
                continue;
            };
            if self.players.len() >= 64 && !self.players.contains_key(&id) {
                continue;
            }
            let star_total: usize = self.players.values().map(|p| p.astra.stars.len()).sum();
            let p = self.players.entry(id).or_default();
            if r.sequence <= p.sequence {
                continue;
            }
            p.sequence = r.sequence;
            if !matches!(r.action, Action::Select(_) | Action::Cancel)
                && r.expected_agent != p.agent
            {
                p.error = Some("Agent changed: stale ability request rejected".into());
                continue;
            }

            if !a.eligible
                && (!p.clove.dead_allowed(a)
                    || matches!(r.action,Action::Fire(_) if p.clove.equipped!=Some(2)))
            {
                continue;
            }
            let okay = match r.action {
                Action::Select(index) if index < 12 + crate::advanced::COUNT => {
                    if p.agent == index {
                        true
                    } else if p.busy() {
                        false
                    } else {
                        p.agent = index;
                        p.extra = Default::default();
                        p.sova = Default::default();
                        p.clove = Default::default();
                        p.harbor = Default::default();
                        p.astra = Default::default();
                        p.viper = Default::default();
                        p.brimstone = Default::default();
                        p.jett = Default::default();
                        p.phoenix = Default::default();
                        p.sage = Default::default();
                        p.omen = Default::default();
                        p.raze = Default::default();
                        p.reyna = Default::default();
                        true
                    }
                }
                Action::Select(_) => false,
                Action::Activate(slot) => {
                    if p.agent >= 12 {
                        p.extra.activate(
                            p.agent,
                            slot,
                            a,
                            &mut crate::advanced::Context {
                                world: w,
                                actors: &actors,
                                objects: &mut self.objects,
                                next: &mut self.next_object,
                                projectiles: &mut self.projectiles,
                                zones: &mut self.zones,
                                smokes: &mut self.smokes,
                                intel: &mut self.intel,
                                beams: &mut self.beams,
                                bursts: &mut self.bursts,
                                vision: &mut self.vision,
                                corpses: &mut self.corpses,
                                kills: &mut self.extra_kills,
                                shields: &mut self.shields,
                                divides: &self.divides,
                                decay: self.decay_loaded.then_some(&mut self.decay),
                                budget: &mut self.budget,
                            },
                        )
                    } else if p.agent == 11 {
                        p.sova.activate(slot, a)
                    } else if p.agent == 10 {
                        p.clove.activate(slot, a, w)
                    } else if p.agent == 9 {
                        p.harbor.activate(
                            slot,
                            a,
                            &mut crate::agents::harbor::Context {
                                world: w,
                                projectiles: &mut self.projectiles,
                                shields: &mut self.shields,
                            },
                        )
                    } else if p.agent == 8 {
                        p.astra.activate(
                            slot,
                            a,
                            &mut crate::agents::astra::Context {
                                world: w,
                                smokes: &mut self.smokes,
                                zones: &mut self.zones,
                                divides: &mut self.divides,
                            },
                        )
                    } else if p.agent == 7 {
                        p.viper.activate(slot, a)
                    } else if p.agent == 6 {
                        p.brimstone.activate(slot, a)
                    } else if p.agent == 0 {
                        p.jett.activate(slot, a, w, &mut self.projectiles)
                    } else if p.agent == 1 {
                        p.sage.activate(slot, a)
                    } else if p.agent == 3 {
                        p.omen.activate(slot, a, w, &mut self.projectiles)
                    } else if p.agent == 4 {
                        p.raze.activate(
                            slot,
                            a,
                            &mut crate::agents::raze::Context {
                                world: w,
                                projectiles: &mut self.projectiles,
                                objects: &mut self.objects,
                                bursts: &mut self.bursts,
                                next: &mut self.next_object,
                            },
                        )
                    } else if p.agent == 5 {
                        p.reyna.activate(slot, a, w)
                    } else {
                        p.phoenix.activate(slot, a)
                    }
                }
                Action::Release(0) => {
                    p.jett.guiding = false;
                    true
                }
                Action::Release(4) if p.agent >= 12 => p.extra.release(
                    p.agent,
                    a,
                    &mut crate::advanced::Context {
                        world: w,
                        actors: &actors,
                        objects: &mut self.objects,
                        next: &mut self.next_object,
                        projectiles: &mut self.projectiles,
                        zones: &mut self.zones,
                        smokes: &mut self.smokes,
                        intel: &mut self.intel,
                        beams: &mut self.beams,
                        bursts: &mut self.bursts,
                        vision: &mut self.vision,
                        corpses: &mut self.corpses,
                        kills: &mut self.extra_kills,
                        shields: &mut self.shields,
                        divides: &self.divides,
                        decay: self.decay_loaded.then_some(&mut self.decay),
                        budget: &mut self.budget,
                    },
                ),
                Action::Release(4) if p.agent == 11 => p.sova.release(
                    a,
                    &mut crate::agents::sova::Context {
                        world: w,
                        projectiles: &mut self.projectiles,
                        objects: &mut self.objects,
                        next: &mut self.next_object,
                        intel: &mut self.intel,
                        beams: &mut self.beams,
                        bursts: &mut self.bursts,
                    },
                ),
                Action::Release(4) => {
                    p.phoenix.guiding = false;
                    p.harbor.guiding = false;
                    true
                }
                Action::Release(_) => true,
                Action::Fire(_) if p.omen.cast.is_some() => false,
                Action::Fire(alt)
                    if (p.agent >= 12 && !p.extra.equipped(p.agent))
                        || (p.agent == 0 && p.jett.slots[3].active == 0)
                        || (p.agent == 1 && p.sage.equipped.is_none())
                        || (p.agent == 2 && p.phoenix.equipped.is_none())
                        || (p.agent == 3 && p.omen.equipped.is_none())
                        || (p.agent == 4 && p.raze.equipped.is_none())
                        || (p.agent == 5 && !p.reyna.equipped)
                        || (p.agent == 6 && p.brimstone.equipped.is_none())
                        || (p.agent == 7 && p.viper.equipped.is_none())
                        || (p.agent == 8 && p.astra.view.is_none())
                        || (p.agent == 10 && p.clove.equipped.is_none())
                        || (p.agent == 11
                            && p.sova.equipped.is_none()
                            && p.sova.view.is_none())
                        || (p.agent == 9
                            && p.harbor.equipped.is_none()
                            && p.harbor.tide.is_none()) =>
                {
                    if !alt && p.strike_delay == 0 && objects::strike(&mut self.objects, a, w) {
                        p.strike_delay = 10;
                        true
                    } else {
                        false
                    }
                }
                Action::Fire(alt) => {
                    if p.agent >= 12 {
                        p.extra.fire(
                            p.agent,
                            alt,
                            a,
                            &mut crate::advanced::Context {
                                world: w,
                                actors: &actors,
                                objects: &mut self.objects,
                                next: &mut self.next_object,
                                projectiles: &mut self.projectiles,
                                zones: &mut self.zones,
                                smokes: &mut self.smokes,
                                intel: &mut self.intel,
                                beams: &mut self.beams,
                                bursts: &mut self.bursts,
                                vision: &mut self.vision,
                                corpses: &mut self.corpses,
                                kills: &mut self.extra_kills,
                                shields: &mut self.shields,
                                divides: &self.divides,
                                decay: self.decay_loaded.then_some(&mut self.decay),
                                budget: &mut self.budget,
                            },
                        )
                    } else if p.agent == 11 {
                        p.sova.fire(
                            alt,
                            a,
                            &mut crate::agents::sova::Context {
                                world: w,
                                projectiles: &mut self.projectiles,
                                objects: &mut self.objects,
                                next: &mut self.next_object,
                                intel: &mut self.intel,
                                beams: &mut self.beams,
                                bursts: &mut self.bursts,
                            },
                        )
                    } else if p.agent == 10 {
                        p.clove.fire(
                            alt,
                            a,
                            &mut crate::agents::clove::Context {
                                world: w,
                                projectiles: &mut self.projectiles,
                                smokes: &mut self.smokes,
                            },
                        )
                    } else if p.agent == 9 {
                        p.harbor.fire(
                            alt,
                            a,
                            &mut crate::agents::harbor::Context {
                                world: w,
                                projectiles: &mut self.projectiles,
                                shields: &mut self.shields,
                            },
                        )
                    } else if p.agent == 8 {
                        if !alt
                            && !p.astra.divide_aim
                            && star_total >= crate::agents::astra::MAX_STARS
                        {
                            false
                        } else {
                            p.astra.fire(
                                alt,
                                a,
                                &mut crate::agents::astra::Context {
                                    world: w,
                                    smokes: &mut self.smokes,
                                    zones: &mut self.zones,
                                    divides: &mut self.divides,
                                },
                            )
                        }
                    } else if p.agent == 7 {
                        if self.toxins.len() >= toxin::MAX_FIELDS {
                            false
                        } else {
                            p.viper.fire(a, w, &mut self.projectiles)
                        }
                    } else if p.agent == 6 {
                        p.brimstone.fire(
                            alt,
                            a,
                            &mut crate::agents::brimstone::Context {
                                world: w,
                                projectiles: &mut self.projectiles,
                                zones: &mut self.zones,
                                smokes: &mut self.smokes,
                            },
                        )
                    } else if p.agent == 0 {
                        p.jett.fire(alt, a, &mut self.projectiles)
                    } else if p.agent == 1 {
                        p.sage
                            .fire(alt, a, w, &mut self.projectiles, &mut self.blocks)
                    } else if p.agent == 3 {
                        p.omen
                            .fire(alt, a, w, &mut self.objects, &mut self.next_object)
                    } else if p.agent == 4 {
                        p.raze.fire(
                            alt,
                            a,
                            &mut crate::agents::raze::Context {
                                world: w,
                                projectiles: &mut self.projectiles,
                                objects: &mut self.objects,
                                bursts: &mut self.bursts,
                                next: &mut self.next_object,
                            },
                        )
                    } else if p.agent == 5 {
                        p.reyna.fire(a, w, &mut self.objects, &mut self.next_object)
                    } else {
                        p.phoenix.fire(alt, a, &mut self.projectiles)
                    }
                }
                Action::Aim(x, z) => p.agent == 10 && p.clove.nudge(x, z, a),
                Action::Recall if p.agent >= 12 => p.extra.recall(
                    p.agent,
                    a,
                    &mut crate::advanced::Context {
                        world: w,
                        actors: &actors,
                        objects: &mut self.objects,
                        next: &mut self.next_object,
                        projectiles: &mut self.projectiles,
                        zones: &mut self.zones,
                        smokes: &mut self.smokes,
                        intel: &mut self.intel,
                        beams: &mut self.beams,
                        bursts: &mut self.bursts,
                        vision: &mut self.vision,
                        corpses: &mut self.corpses,
                        kills: &mut self.extra_kills,
                        shields: &mut self.shields,
                        divides: &self.divides,
                        decay: self.decay_loaded.then_some(&mut self.decay),
                        budget: &mut self.budget,
                    },
                ),
                Action::Recall => p.agent == 8 && p.astra.dissipate(a, &mut self.smokes),
                Action::Cancel => {
                    p.extra.cancel();
                    p.sova.cancel();
                    p.clove.cancel();
                    p.harbor.cancel();
                    p.astra.cancel();
                    p.viper.cancel();
                    p.brimstone.cancel();
                    p.jett.cancel();
                    p.phoenix.cancel();
                    p.sage.cancel();
                    p.omen.cancel();
                    p.raze.cancel();
                    p.reyna.cancel();
                    true
                }
            };
            p.error = (!okay).then(|| {
                if p.agent == 10 && matches!(r.action, Action::Activate(3)) {
                    "Not Dead Yet unavailable: respawn/inventory/Hardcore integration required"
                        .into()
                } else if p.agent == 1 && matches!(r.action, Action::Activate(3)) {
                    "Resurrection unavailable: Minecraft respawn/inventory integration required"
                        .into()
                } else {
                    "Ability unavailable, obstructed, or resource limit reached".into()
                }
            });
        }
        for t in self.vulnerable.values_mut() {
            *t = t.saturating_sub(1);
        }
        for (entity, t) in &self.vulnerable {
            if *t == 0 {
                w.vulnerability(*entity, false);
            }
        }
        self.vulnerable.retain(|_, t| *t > 0);
        for a in &actors {
            w.vulnerability(a.entity, self.vulnerable.contains_key(&a.entity));
        }
        for d in &mut self.divides {
            d.ticks = d.ticks.saturating_sub(1);
        }
        self.divides.retain(|d| d.ticks > 0);
        for shield in &mut self.shields {
            shield.ticks = shield.ticks.saturating_sub(1);
        }
        self.shields.retain(|s| s.ticks > 0);
        self.toxins.clear();
        let mut sova_kills = vec![];
        for a in &actors {
            if let Entity::Player(id) = a.entity
                && let Some(p) = self.players.get_mut(&id)
            {
                p.strike_delay = p.strike_delay.saturating_sub(1);
                if p.agent >= 12 {
                    p.extra.tick(
                        p.agent,
                        a,
                        &mut crate::advanced::Context {
                            world: w,
                            actors: &actors,
                            objects: &mut self.objects,
                            next: &mut self.next_object,
                            projectiles: &mut self.projectiles,
                            zones: &mut self.zones,
                            smokes: &mut self.smokes,
                            intel: &mut self.intel,
                            beams: &mut self.beams,
                            bursts: &mut self.bursts,
                            vision: &mut self.vision,
                            corpses: &mut self.corpses,
                            kills: &mut self.extra_kills,
                            shields: &mut self.shields,
                            divides: &self.divides,
                            decay: self.decay_loaded.then_some(&mut self.decay),
                            budget: &mut self.budget,
                        },
                    );
                } else if p.agent == 11 {
                    for target in p.sova.tick(
                        a,
                        &mut crate::agents::sova::Context {
                            world: w,
                            projectiles: &mut self.projectiles,
                            objects: &mut self.objects,
                            next: &mut self.next_object,
                            intel: &mut self.intel,
                            beams: &mut self.beams,
                            bursts: &mut self.bursts,
                        },
                    ) {
                        sova_kills.push((id, target));
                    }
                } else if p.agent == 10 {
                    p.clove.tick(a, w);
                } else if p.agent == 9 {
                    p.harbor.tick(a, w, &mut self.zones);
                } else if p.agent == 8 {
                    p.astra.tick(a, w);
                } else if p.agent == 7 {
                    self.toxins.extend(
                        p.viper
                            .tick(a, toxin::MAX_FIELDS.saturating_sub(self.toxins.len())),
                    );
                } else if p.agent == 6 {
                    p.brimstone.tick(a.eligible);
                } else if p.agent == 0 {
                    p.jett.tick(a, w);
                } else if p.agent == 1 {
                    p.sage.tick(a, w);
                } else if p.agent == 3 {
                    p.omen.tick(a, w, &self.objects);
                } else if p.agent == 4 {
                    p.raze.tick(a);
                } else if p.agent == 5 {
                    p.reyna.tick(a, w);
                    self.vision.entry(id).or_default().phased = p.reyna.phase > 0;
                } else {
                    p.phoenix.tick(a, w, &mut self.zones);
                }
            }
        }
        for (owner, target) in sova_kills {
            self.defer_kill(w, owner, target, false);
        }
        for o in &mut self.objects {
            if o.health <= 0. || o.ticks == 0 {
                continue;
            }
            if matches!(o.kind, objects::ObjectKind::ReconBolt) && [60, 40, 20].contains(&o.ticks) {
                self.intel.scan(
                    w,
                    o.owner,
                    add(o.position, [0., 0.12, 0.]),
                    self.players
                        .get(&o.owner)
                        .map_or(24., |p| p.sova.settings.scan_range.clamp(1., 32.)),
                );
                w.ability_sound(o.position, "minecraft:block.amethyst_block.chime");
            }
            if matches!(o.kind, objects::ObjectKind::Leer) {
                let eye = add(o.position, [0., 0.35, 0.]);
                for t in actors
                    .iter()
                    .filter(|t| t.eligible && t.entity != Entity::Player(o.owner))
                {
                    if w.can_harm(o.owner, t.entity)
                        && distance(t.eye(), eye) <= 20.
                        && dot(t.direction, unit(sub(eye, t.eye()))) > 0.65
                        && block_hit(w, t.eye(), eye, 0.).is_none()
                    {
                        if let Entity::Player(id) = t.entity {
                            self.vision.entry(id).or_default().nearsight = 3;
                        }
                        w.effect(t.entity, "minecraft:blindness", 3, 0);
                    }
                }
            }

            o.ticks = o.ticks.saturating_sub(1);
            if matches!(o.kind, objects::ObjectKind::BlastPack) && o.ticks == 0 {
                self.bursts
                    .push(crate::agents::raze::pack(o.owner, o.position));
            }
            if matches!(o.kind, objects::ObjectKind::BoomBot) {
                let target = actors
                    .iter()
                    .filter(|a| {
                        a.eligible
                            && a.entity != Entity::Player(o.owner)
                            && w.can_harm(o.owner, a.entity)
                    })
                    .filter(|a| {
                        distance(a.position, o.position) < 12.0
                            && (o.target == Some(a.entity) || {
                                let dir = unit(sub(a.position, o.position));
                                let facing = unit(o.velocity);
                                let dot = (0..3).map(|i| dir[i] * facing[i]).sum::<f64>();
                                dot > 0.2
                                    && block_hit(w, add(o.position, [0.0, 0.3, 0.0]), a.eye(), 0.0)
                                        .is_none()
                            })
                    })
                    .min_by(|a, b| {
                        distance(a.position, o.position)
                            .total_cmp(&distance(b.position, o.position))
                    });
                if let Some(a) = target {
                    o.target = Some(a.entity);
                    o.velocity = scale(horizontal(sub(a.position, o.position)), 0.28);
                    if distance(a.position, o.position) < 0.95 {
                        self.bursts.push(explosions::Explosion {
                            owner: o.owner,
                            position: add(o.position, [0.0, 0.2, 0.0]),
                            radius: 3.0,
                            damage: 10.0,
                            push: 0.7,
                            self_damage: false,
                        });
                        o.health = 0.0;
                    }
                }
                let v = o.velocity;
                for delta in [v, [v[2], 0.0, -v[0]], [-v[2], 0.0, v[0]], scale(v, -1.0)] {
                    if length(sweep(w, o.bounds(), delta)) < length(delta) * 0.9 {
                        continue;
                    }
                    let next = add(o.position, delta);
                    let top = add(next, [0.0, 0.25, 0.0]);
                    let down = add(next, [0.0, -0.5, 0.0]);
                    if let Some(hit) = block_hit(w, top, down, 0.0) {
                        o.position = add(add(top, scale(sub(down, top), hit)), [0.0, 0.02, 0.0]);
                        o.velocity = delta;
                        break;
                    }
                }
            }
        }
        self.objects.retain(|o| {
            o.ticks > 0
                && (o.health > 0.0 || matches!(o.kind, objects::ObjectKind::Echo))
                && (!o.kind.is_advanced()
                    || self
                        .players
                        .get(&o.owner)
                        .is_some_and(|p| p.extra.owns(p.agent, o.id)))
                && (!matches!(o.kind, objects::ObjectKind::Drone)
                    || self.players.get(&o.owner).is_some_and(|p| {
                        p.agent == 11 && p.sova.view.is_some() && p.sova.drone == Some(o.id)
                    }))
                && (!matches!(o.kind, objects::ObjectKind::Shade)
                    || self
                        .players
                        .get(&o.owner)
                        .is_some_and(|p| p.omen.shade == Some(o.id)))
        });
        for (id, p) in &self.players {
            self.vision.entry(*id).or_default().phased = p.omen.cast.is_some()
                || p.reyna.phase > 0
                || p.astra.view.is_some()
                || p.extra.phased(p.agent);
        }
        let actors = w.actors();
        for smoke in &mut self.smokes {
            smoke.ticks = smoke.ticks.saturating_sub(1);
        }
        self.smokes.retain(|s| s.ticks > 0);
        for v in self.vision.values_mut() {
            v.flash = v.flash.saturating_sub(1);
            v.nearsight = v.nearsight.saturating_sub(1);
            v.deafen = v.deafen.saturating_sub(1);
        }
        if self.decay_loaded {
            toxin::tick(w, &self.toxins, &mut self.decay);
        }
        let mut affected = std::collections::HashSet::new();
        let mut zone_kills = Vec::new();
        for zone in &mut self.zones {
            zone.ticks = zone.ticks.saturating_sub(1);
            if matches!(
                zone.kind,
                zones::ZoneKind::CyberCage
                    | zones::ZoneKind::Seize
                    | zones::ZoneKind::Nightfall
                    | zones::ZoneKind::ElectricPulse
            ) {
                continue;
            }
            if matches!(zone.kind, zones::ZoneKind::ElectricWall) {
                if zone.ticks.is_multiple_of(10) {
                    for a in actors
                        .iter()
                        .filter(|a| a.eligible && zone.touches(a.bounds))
                    {
                        if w.recon_target(zone.owner, a.entity)
                            && affected.insert((zone.owner, a.entity))
                        {
                            let hit = w.hurt(zone.owner, a.entity, 2.);
                            if hit.killed {
                                zone_kills.push((zone.owner, a.entity));
                            }
                        }
                    }
                }
                continue;
            }
            if matches!(
                zone.kind,
                zones::ZoneKind::WaterWall | zones::ZoneKind::Whirlpool
            ) {
                for a in actors
                    .iter()
                    .filter(|a| a.eligible && zone.touches(a.bounds))
                {
                    if matches!(zone.kind, zones::ZoneKind::WaterWall) {
                        w.effect(a.entity, "minecraft:slowness", 12, 1);
                    } else if zone.ticks == 0
                        && a.entity != Entity::Player(zone.owner)
                        && w.can_harm(zone.owner, a.entity)
                    {
                        w.effect(a.entity, "minecraft:slowness", 60, 2);
                        if let Entity::Player(id) = a.entity {
                            let v = self.vision.entry(id).or_default();
                            v.nearsight = v.nearsight.max(60);
                        }
                    }
                }
                continue;
            }
            if matches!(zone.kind, zones::ZoneKind::Gravity | zones::ZoneKind::Nova) {
                let center = scale(add(zone.bounds.min, zone.bounds.max), 0.5);
                for a in actors
                    .iter()
                    .filter(|a| a.eligible && zone.touches(a.bounds))
                {
                    if w.status_immune(a.entity) {
                        continue;
                    }
                    if matches!(zone.kind, zones::ZoneKind::Gravity) {
                        w.push_entity(
                            a.entity,
                            add(
                                scale(horizontal(sub(center, a.position)), 0.16),
                                [0., a.velocity[1], 0.],
                            ),
                        );
                        if zone.ticks == 0 {
                            self.vulnerable.insert(a.entity, 60);
                            w.vulnerability(a.entity, true);
                        }
                    } else if zone.ticks == 0 {
                        w.effect(a.entity, "minecraft:slowness", 60, 3);
                        w.effect(a.entity, "minecraft:weakness", 60, 1);
                        if let Entity::Player(id) = a.entity {
                            self.vision.entry(id).or_default().nearsight = 60;
                        }
                    }
                }
                continue;
            }
            if zone.ticks % 10 == 0 || matches!(zone.kind, zones::ZoneKind::Slow) {
                for a in actors
                    .iter()
                    .filter(|a| a.eligible && zone.touches(a.bounds))
                {
                    if matches!(zone.kind, zones::ZoneKind::Acid) {
                        w.effect(a.entity, "minecraft:slowness", 12, 1);
                    }
                    if matches!(zone.kind, zones::ZoneKind::Stim) {
                        w.effect(a.entity, "minecraft:speed", 30, 0);
                        w.effect(a.entity, "minecraft:haste", 30, 1);
                        continue;
                    }
                    if matches!(zone.kind, zones::ZoneKind::Slow) {
                        w.effect(a.entity, "minecraft:slowness", 8, 2);
                        continue;
                    }
                    if !affected.insert((zone.owner, a.entity)) {
                        continue;
                    }
                    if a.entity == Entity::Player(zone.owner)
                        && matches!(
                            zone.kind,
                            zones::ZoneKind::FirePool | zones::ZoneKind::FireWall
                        )
                    {
                        w.heal(a.entity, 1.0);
                    } else {
                        let hit = w.hurt_with(
                            zone.owner,
                            a.entity,
                            if matches!(zone.kind, zones::ZoneKind::Orbital) {
                                8.
                            } else {
                                2.
                            },
                            "minecraft:in_fire",
                        );
                        if hit.killed {
                            zone_kills.push((zone.owner, a.entity));
                        }
                    }
                }
            }
        }
        for (owner, target) in zone_kills {
            self.defer_kill(w, owner, target, false);
        }
        self.zones.retain(|z| {
            z.ticks > 0
                && (!matches!(
                    z.kind,
                    zones::ZoneKind::CyberCage
                        | zones::ZoneKind::Seize
                        | zones::ZoneKind::Nightfall
                        | zones::ZoneKind::ElectricPulse
                        | zones::ZoneKind::ElectricWall
                ) || self
                    .players
                    .get(&z.owner)
                    .is_some_and(|p| p.extra.zone_active(p.agent, z.kind)))
        });
        w.block_arrows(&self.divides, &mut self.shields, &mut self.objects);
        let mut alive = Vec::new();
        for mut p in std::mem::take(&mut self.projectiles) {
            if matches!(p.kind, ProjectileKind::Knife { .. })
                && projectiles::intercept(
                    w,
                    &mut self.shields,
                    &self.divides,
                    &mut self.objects,
                    projectiles::Shot {
                        attacker: Some(Entity::Player(p.owner)),
                        from: p.position,
                        to: add(p.position, p.velocity),
                        damage: 10.,
                        padding: shields::CollisionPadding {
                            actors: 0.08,
                            terrain: 0.08,
                        },
                    },
                )
            {
                continue;
            }
            if matches!(p.kind, ProjectileKind::SovaBolt { .. }) {
                if crate::agents::sova::arrow(
                    &mut p,
                    &mut crate::agents::sova::Context {
                        world: w,
                        projectiles: &mut self.projectiles,
                        objects: &mut self.objects,
                        next: &mut self.next_object,
                        intel: &mut self.intel,
                        beams: &mut self.beams,
                        bursts: &mut self.bursts,
                    },
                ) {
                    alive.push(p);
                }
                continue;
            }
            if matches!(p.kind, ProjectileKind::ReconDart) {
                let hit = p.advance(w);
                match hit {
                    Some(Impact::Entity(target, _, _)) => {
                        if w.recon_target(p.owner, target) {
                            self.intel.tag(p.owner, target);
                        }
                    }
                    None if p.ticks > 0 => alive.push(p),
                    _ => {}
                }
                continue;
            }
            if matches!(p.kind, ProjectileKind::Meddle) {
                p.velocity[1] -= 0.03;
                let end = add(p.position, p.velocity);
                if let Some(t) = block_hit(w, p.position, end, 0.1) {
                    p.position = add(p.position, scale(p.velocity, (t - 0.01).max(0.)));
                    p.velocity = [0.; 3];
                } else {
                    p.position = end;
                }
                p.ticks = p.ticks.saturating_sub(1);
                if p.ticks == 0 {
                    if self.decay_loaded {
                        for a in actors
                            .iter()
                            .filter(|a| {
                                a.eligible
                                    && distance(a.eye(), p.position) <= 4.5
                                    && w.can_harm(p.owner, a.entity)
                            })
                            .cloned()
                            .collect::<Vec<_>>()
                        {
                            toxin::pulse(w, &a, &mut self.decay, 8., 100);
                        }
                    }
                    w.burst_fx(p.position);
                    w.ability_sound(p.position, "minecraft:block.amethyst_cluster.break");
                } else {
                    alive.push(p);
                }
                continue;
            }
            if matches!(p.kind, ProjectileKind::StormSurge) {
                p.velocity[1] -= 0.04;
                let end = add(p.position, p.velocity);
                let hit = block_hit(w, p.position, end, 0.1);
                p.ticks = p.ticks.saturating_sub(1);
                if hit.is_some() || p.ticks == 0 {
                    let at = hit.map_or(p.position, |t| {
                        add(p.position, scale(p.velocity, (t - 0.01).max(0.)))
                    });
                    if self.zones.len() < MAX_ZONES {
                        self.zones.push(zones::Zone {
                            owner: p.owner,
                            kind: zones::ZoneKind::Whirlpool,
                            bounds: Box3 {
                                min: add(at, [-3., -0.5, -3.]),
                                max: add(at, [3., 3., 3.]),
                            },
                            ticks: 20,
                        });
                    }
                    w.ability_sound(at, "minecraft:entity.generic.splash");
                } else {
                    p.position = end;
                    alive.push(p);
                }
                continue;
            }
            if matches!(p.kind, ProjectileKind::Reckoning) {
                let end = add(p.position, p.velocity);
                if w.boxes(end.map(|v| v.floor() as i32)).is_none() {
                    continue;
                }
                for a in actors
                    .iter()
                    .filter(|a| a.eligible && a.entity != Entity::Player(p.owner))
                {
                    if !p.victims.contains(&a.entity)
                        && w.can_harm(p.owner, a.entity)
                        && a.bounds.expanded(2.5).segment(p.position, end).is_some()
                    {
                        p.victims.push(a.entity);
                        w.effect(a.entity, "minecraft:slowness", 60, 2);
                        if let Entity::Player(id) = a.entity {
                            let v = self.vision.entry(id).or_default();
                            v.nearsight = v.nearsight.max(60);
                        }
                    }
                }
                p.position = end;
                p.ticks = p.ticks.saturating_sub(1);
                if p.ticks > 0 {
                    alive.push(p);
                }
                continue;
            }

            if matches!(
                p.kind,
                ProjectileKind::SnakeBite | ProjectileKind::PoisonEmitter
            ) {
                p.velocity[1] -= 0.04;
                let next = add(p.position, p.velocity);
                if let Some(t) = block_hit(w, p.position, next, 0.1) {
                    let at = add(p.position, scale(p.velocity, (t - 0.02).max(0.)));
                    if matches!(p.kind, ProjectileKind::PoisonEmitter) {
                        if let Some(owner) = self.players.get_mut(&p.owner).filter(|x| x.agent == 7)
                        {
                            owner.viper.cloud = Some(at);
                            owner.viper.placed_ticks = 1200;
                        }
                    } else if self.zones.len() < MAX_ZONES {
                        self.zones.push(zones::Zone {
                            owner: p.owner,
                            kind: zones::ZoneKind::Acid,
                            bounds: Box3 {
                                min: add(at, [-3., 0., -3.]),
                                max: add(at, [3., 0.8, 3.]),
                            },
                            ticks: 120,
                        });
                    }
                    w.ability_sound(at, "minecraft:block.fire.extinguish");
                    continue;
                }
                p.position = next;
                p.ticks = p.ticks.saturating_sub(1);
                if p.ticks > 0 {
                    alive.push(p);
                }
                continue;
            }
            if let ProjectileKind::Incendiary { duration } = p.kind {
                p.velocity[1] -= 0.04;
                let end = add(p.position, p.velocity);
                if let Some(t) = block_hit(w, p.position, end, 0.1) {
                    let at = add(p.position, scale(p.velocity, (t - 0.02).max(0.)));
                    let grounded = block_hit(w, at, add(at, [0., -0.25, 0.]), 0.).is_some();
                    if grounded && p.velocity[1] < 0. {
                        if self.zones.len() < MAX_ZONES {
                            self.zones.push(zones::Zone {
                                owner: p.owner,
                                kind: zones::ZoneKind::Incendiary,
                                bounds: Box3 {
                                    min: add(at, [-3., 0., -3.]),
                                    max: add(at, [3., 0.8, 3.]),
                                },
                                ticks: duration,
                            });
                            w.ability_sound(at, "minecraft:item.firecharge.use");
                        }
                        continue;
                    }
                    p.position = at;
                    p.velocity[0] *= -0.6;
                    p.velocity[2] *= -0.6;
                } else {
                    p.position = end;
                }
                p.ticks = p.ticks.saturating_sub(1);
                if p.ticks > 0 {
                    alive.push(p);
                }
                continue;
            }
            if p.guided
                && matches!(p.kind, ProjectileKind::Cloudburst)
                && self.players.get(&p.owner).is_some_and(|x| x.jett.guiding)
                && let Some(a) = actors.iter().find(|a| a.entity == Entity::Player(p.owner))
            {
                p.velocity = scale(a.direction, 0.9);
            }
            if let ProjectileKind::Curveball { right } = p.kind {
                let angle: f64 = if right { 0.14 } else { -0.14 };
                let [x, y, z] = p.velocity;
                p.velocity = [
                    x * angle.cos() - z * angle.sin(),
                    y,
                    x * angle.sin() + z * angle.cos(),
                ];
            }
            if matches!(
                p.kind,
                ProjectileKind::HotHands
                    | ProjectileKind::SlowOrb
                    | ProjectileKind::BlastPack
                    | ProjectileKind::Grenade { .. }
            ) {
                p.velocity[1] -= 0.04;
            }
            if let ProjectileKind::DarkCover { target } = p.kind {
                let delta = sub(target, p.position);
                if length(delta) <= 2.0 {
                    if self.smokes.len() < MAX_SMOKES {
                        self.smokes.push(Smoke {
                            owner: p.owner,
                            center: target,
                            radius: 2.8,
                            ticks: 260,
                            color: [65, 45, 100, 255],
                        });
                    }
                } else {
                    p.position = add(p.position, scale(unit(delta), 2.0));
                    p.ticks = p.ticks.saturating_sub(1);
                    if p.ticks > 0 {
                        alive.push(p);
                    }
                }
                continue;
            }
            if matches!(p.kind, ProjectileKind::Paranoia) {
                let end = add(p.position, p.velocity);
                for a in actors
                    .iter()
                    .filter(|a| a.eligible && a.entity != Entity::Player(p.owner))
                {
                    if !w.can_harm(p.owner, a.entity) || p.victims.contains(&a.entity) {
                        continue;
                    }
                    if a.bounds.expanded(0.8).segment(p.position, end).is_some() {
                        w.effect(a.entity, "minecraft:blindness", 60, 0);
                        if let Entity::Player(id) = a.entity {
                            self.vision.entry(id).or_default().nearsight = 60;
                        }
                        p.victims.push(a.entity);
                    }
                }
                p.position = end;
                p.ticks = p.ticks.saturating_sub(1);
                if p.ticks > 0 {
                    alive.push(p);
                }
                continue;
            }
            let before = p.position;
            let impact = p.advance(w);
            match (p.kind, impact) {
                (ProjectileKind::BlastPack, impact) if impact.is_some() || p.ticks == 0 => {
                    let at = match impact {
                        Some(Impact::Block(at)) => at,
                        _ => p.position,
                    };
                    if self.objects.len() < MAX_OBJECTS {
                        self.next_object = self.next_object.saturating_add(1);
                        self.objects.push(objects::Object {
                            last_hit: None,
                            id: self.next_object,
                            owner: p.owner,
                            kind: objects::ObjectKind::BlastPack,
                            position: at,
                            health: 4.0,
                            ticks: 100,
                            velocity: [0.0; 3],
                            target: None,
                        });
                    }
                }
                (ProjectileKind::Rocket, impact) if impact.is_some() || p.ticks == 0 => {
                    let at = match impact {
                        Some(Impact::Block(at) | Impact::Entity(_, at, _)) => at,
                        _ => p.position,
                    };
                    self.bursts.push(explosions::Explosion {
                        owner: p.owner,
                        position: at,
                        radius: 5.0,
                        damage: 26.0,
                        push: 1.0,
                        self_damage: true,
                    });
                }
                (ProjectileKind::Grenade { cluster }, impact) => {
                    if p.ticks == 0 {
                        self.bursts.push(explosions::Explosion {
                            owner: p.owner,
                            position: p.position,
                            radius: if cluster { 2.5 } else { 3.0 },
                            damage: if cluster { 8.0 } else { 6.0 },
                            push: 0.4,
                            self_damage: true,
                        });
                        if !cluster {
                            for direction in [
                                [0.3, 0.25, 0.0],
                                [-0.3, 0.25, 0.0],
                                [0.0, 0.25, 0.3],
                                [0.0, 0.25, -0.3],
                            ] {
                                if alive.len() < MAX_PROJECTILES {
                                    alive.push(Projectile {
                                        owner: p.owner,
                                        kind: ProjectileKind::Grenade { cluster: true },
                                        position: add(p.position, [0.0, 0.1, 0.0]),
                                        velocity: direction,
                                        ticks: 12,
                                        guided: false,
                                        victims: vec![],
                                    });
                                }
                            }
                        }
                    } else {
                        if impact.is_some() {
                            let mut v = scale(p.velocity, 0.65);
                            for i in 0..3 {
                                let mut axis = [0.0; 3];
                                axis[i] = p.velocity[i];
                                if axis[i].abs() > 1e-9
                                    && block_hit(w, before, add(before, axis), 0.08).is_some()
                                {
                                    v[i] = -v[i];
                                }
                            }
                            p.position = before;
                            p.velocity = v;
                        }
                        alive.push(p);
                    }
                }

                (ProjectileKind::SlowOrb, impact) if impact.is_some() || p.ticks == 0 => {
                    let at = match impact {
                        Some(Impact::Block(at)) => at,
                        _ => p.position,
                    };
                    if self.zones.len() < MAX_ZONES {
                        self.zones.push(zones::Zone {
                            owner: p.owner,
                            kind: zones::ZoneKind::Slow,
                            bounds: Box3 {
                                min: add(at, [-3.0, 0.0, -3.0]),
                                max: add(at, [3.0, 0.6, 3.0]),
                            },
                            ticks: 120,
                        });
                    }
                }

                (ProjectileKind::HotHands, impact) if impact.is_some() || p.ticks == 0 => {
                    let at = match impact {
                        Some(Impact::Block(at)) => at,
                        _ => p.position,
                    };
                    if self.zones.len() < MAX_ZONES {
                        self.zones.push(zones::Zone {
                            owner: p.owner,
                            kind: zones::ZoneKind::FirePool,
                            bounds: Box3 {
                                min: add(at, [-2.5, 0.0, -2.5]),
                                max: add(at, [2.5, 0.7, 2.5]),
                            },
                            ticks: 120,
                        });
                    }
                }
                (ProjectileKind::Curveball { .. }, impact) if impact.is_some() || p.ticks == 0 => {
                    for a in actors
                        .iter()
                        .filter(|a| a.eligible && distance(a.eye(), p.position) < 15.0)
                    {
                        if !w.status_immune(a.entity)
                            && block_hit(w, a.eye(), p.position, 0.0).is_none()
                        {
                            let facing = unit(sub(p.position, a.eye()));
                            let dot = (0..3).map(|i| facing[i] * a.direction[i]).sum::<f64>();
                            let ticks = if dot > 0.15 { 40 } else { 6 };
                            w.effect(a.entity, "minecraft:blindness", ticks, 0);
                            if let Entity::Player(id) = a.entity {
                                let v = self.vision.entry(id).or_default();
                                v.flash = v.flash.max(ticks);
                            }
                        }
                    }
                }

                (ProjectileKind::Cloudburst, Some(Impact::Block(at))) => {
                    self.add_smoke(p.owner, at)
                }
                (ProjectileKind::Cloudburst, _) if p.ticks == 0 => {
                    self.add_smoke(p.owner, p.position)
                }
                (
                    ProjectileKind::Knife { alternate },
                    Some(Impact::Entity(target, point, head)),
                ) => {
                    let legs = actors.iter().find(|a| a.entity == target).is_some_and(|a| {
                        point[1] < a.bounds.min[1] + (a.bounds.max[1] - a.bounds.min[1]) * 0.35
                    });
                    let hit = w.hurt(
                        p.owner,
                        target,
                        if head {
                            30.0
                        } else if legs {
                            8.5
                        } else {
                            10.0
                        },
                    );
                    if hit.accepted && hit.killed {
                        self.defer_kill(w, p.owner, target, !alternate);
                    }
                }
                (_, Some(_)) => {}
                (_, None) if p.ticks > 0 => alive.push(p),
                _ => {}
            }
        }
        alive.truncate(MAX_PROJECTILES);
        self.projectiles = alive;
        for e in std::mem::take(&mut self.bursts) {
            for (owner, target) in explosions::apply(w, &mut self.objects, e) {
                self.defer_kill(w, owner, target, false);
            }
        }

        for (owner, target) in std::mem::take(&mut self.extra_kills) {
            self.defer_kill(w, owner, target, false);
        }
        self.hidden = self
            .players
            .iter()
            .filter(|(_, p)| {
                p.extra.hidden(p.agent) || p.agent == 5 && p.reyna.phase > 0 && p.reyna.frenzy > 0
            })
            .filter_map(|(id, _)| {
                w.network_id(Entity::Player(*id))
                    .map(|entity| (*id, entity))
            })
            .take(64)
            .collect();
        self.viewers = self
            .players
            .keys()
            .map(|id| (*id, w.intel_viewers(*id)))
            .collect();
        if self.decay_loaded {
            w.save_decay(
                &self
                    .decay
                    .iter()
                    .map(|(e, d)| (*e, d.clone()))
                    .collect::<Vec<_>>(),
            );
        }
        for (id, p) in &mut self.players {
            if p.agent == 0
                && p.jett.knives == 0
                && !self
                    .pending_deaths
                    .iter()
                    .any(|(owner, _, _, primary)| *owner == *id && *primary)
                && !self
                    .projectiles
                    .iter()
                    .any(|b| b.owner == *id && matches!(b.kind, ProjectileKind::Knife { .. }))
            {
                p.jett.slots[3].active = 0;
            }
        }
    }
    pub fn defer_kill(&mut self, w: &impl World, owner: u128, target: Entity, primary: bool) {
        let at = w
            .actors()
            .into_iter()
            .find(|a| a.entity == target)
            .map(|a| a.eye())
            .or_else(|| self.positions.get(&target).copied());
        let Some(at) = at else {
            return;
        };
        self.queue_death(owner, target, at, primary);
    }
    pub fn queue_death(&mut self, owner: u128, target: Entity, at: V3, primary: bool) {
        if self.pending_deaths.len() < 256 {
            self.pending_deaths.push((owner, target, at, primary));
        }
    }
    pub fn credit_kill(&mut self, owner: u128, target: Entity) {
        if self.credited.contains(&target) {
            return;
        }
        self.credited.push_back(target);
        if self.credited.len() > 256 {
            self.credited.pop_front();
        }
        if let Some(p) = self.players.get_mut(&owner) {
            p.extra.kill(p.agent);
            if p.agent == 4 {
                p.raze.kill();
            } else if p.agent == 0 {
                p.jett.kill();
            }
        }
    }
    fn add_smoke(&mut self, owner: u128, center: V3) {
        if self.smokes.len() < MAX_SMOKES {
            self.smokes.push(Smoke {
                center,
                color: [192, 224, 238, 255],
                radius: 2.2,
                ticks: 50,
                owner,
            });
        }
    }
    pub fn snapshot(&self, id: u128) -> Option<Snapshot> {
        let p = self.players.get(&id)?;
        let slots = if p.agent >= 12 {
            p.extra.slots(p.agent)
        } else if p.agent == 11 {
            p.sova.slots
        } else if p.agent == 10 {
            p.clove.slots
        } else if p.agent == 9 {
            p.harbor.slots
        } else if p.agent == 8 {
            p.astra.slots
        } else if p.agent == 7 {
            p.viper.slots
        } else if p.agent == 6 {
            p.brimstone.slots
        } else if p.agent == 0 {
            p.jett.slots
        } else if p.agent == 1 {
            p.sage.slots
        } else if p.agent == 3 {
            p.omen.slots
        } else if p.agent == 4 {
            p.raze.slots
        } else if p.agent == 5 {
            p.reyna.slots
        } else {
            p.phoenix.slots
        };
        Some(Snapshot {
            agent: p.agent,
            kit: p.extra.stats(p.agent),
            hidden_entities: self
                .hidden
                .iter()
                .filter(|(owner, _)| {
                    *owner != id && !self.viewers.get(owner).is_some_and(|v| v.contains(&id))
                })
                .map(|(_, entity)| *entity)
                .collect(),
            contacts: self.intel.snapshot(id),
            beams: self.beams.clone(),
            trails: self
                .players
                .iter()
                .filter(|(owner, _)| {
                    **owner == id || self.viewers.get(owner).is_some_and(|v| v.contains(&id))
                })
                .flat_map(|(_, p)| p.extra.trails(p.agent))
                .take(2)
                .collect(),
            bow_charge: p.sova.charging,
            bounces: p.sova.bounces,
            fury_shots: p.sova.fury_shots,
            clove_marks: p.clove.marks.len() as u8,
            after_death: p.agent == 10 && p.clove.death.is_some(),
            sequence: p.sequence,
            equipped: if p.agent >= 12 {
                p.extra.equipped(p.agent)
            } else if p.agent == 11 {
                p.sova.equipped.is_some() || p.sova.view.is_some()
            } else if p.agent == 10 {
                p.clove.equipped.is_some()
            } else if p.agent == 9 {
                p.harbor.equipped.is_some() || p.harbor.tide.is_some()
            } else if p.agent == 8 {
                p.astra.view.is_some()
            } else if p.agent == 7 {
                p.viper.equipped.is_some()
            } else if p.agent == 6 {
                p.brimstone.equipped.is_some()
            } else if p.agent == 0 {
                p.jett.slots[3].active > 0
            } else if p.agent == 1 {
                p.sage.equipped.is_some()
            } else if p.agent == 3 {
                p.omen.equipped.is_some()
            } else if p.agent == 4 {
                p.raze.equipped.is_some()
            } else if p.agent == 5 {
                p.reyna.equipped
            } else {
                p.phoenix.equipped.is_some()
            },
            charges: slots.map(|s| s.charges),
            cooldowns: slots.map(|s| s.remaining),
            active: slots.map(|s| s.active),
            knives: if p.agent == 0 { p.jett.knives } else { 0 },
            camera: p.extra.camera(p.agent).or(p
                .sova
                .view
                .as_ref()
                .or(p.astra.view.as_ref())
                .map(|v| v.position)),
            stars: self
                .players
                .values()
                .flat_map(|p| p.astra.stars.clone())
                .take(crate::agents::astra::MAX_STARS)
                .collect(),
            star_count: p.astra.available,
            divides: self.divides.clone(),
            shields: self.shields.clone(),
            fuel: (p.agent == 7).then_some(p.viper.fuel),
            toxins: self.toxins.clone(),
            smokes: self.smokes.clone(),
            projectiles: self.projectiles.clone(),
            error: p.error.clone(),
            zones: self.zones.clone(),
            objects: self
                .objects
                .iter()
                .filter(|o| {
                    !matches!(
                        o.kind,
                        objects::ObjectKind::Trapwire | objects::ObjectKind::DormantEcho
                    ) || o.owner == id
                        || self.viewers.get(&o.owner).is_some_and(|v| v.contains(&id))
                        || self.positions.get(&Entity::Player(id)).is_some_and(|p| {
                            distance(
                                *p,
                                if matches!(o.kind, objects::ObjectKind::Trapwire) {
                                    scale(add(o.position, o.velocity), 0.5)
                                } else {
                                    o.position
                                },
                            ) <= 3.
                        })
                })
                .cloned()
                .chain(
                    p.reyna
                        .souls
                        .iter()
                        .enumerate()
                        .map(|(i, s)| objects::Object {
                            last_hit: None,
                            id: u64::MAX - i as u64,
                            owner: id,
                            kind: objects::ObjectKind::Soul,
                            position: s.position,
                            health: 0.,
                            ticks: s.ticks,
                            velocity: [0.; 3],
                            target: None,
                        }),
                )
                .collect(),
            preview: if p.agent == 10 {
                p.clove.preview.map(|at| Smoke {
                    owner: id,
                    center: add(at, [0., 1., 0.]),
                    radius: 0.4,
                    ticks: 0,
                    color: [170, 110, 210, 255],
                })
            } else if p.agent == 9 {
                p.harbor.preview.map(|center| Smoke {
                    center,
                    radius: 3.,
                    ticks: 0,
                    owner: id,
                    color: [35, 160, 215, 80],
                })
            } else {
                p.omen.preview.map(|center| Smoke {
                    center,
                    radius: if p.omen.equipped == Some(2) { 2.8 } else { 0.4 },
                    ticks: 0,
                    owner: id,
                    color: [100, 80, 180, 45],
                })
            },
            vision: self.vision.get(&id).copied().unwrap_or_default(),
        })
    }
}

pub static ACTIVE_EPOCH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
