use crate::{
    abilities::{Ability, Kind, Runtime},
    gameplay::{
        geometry::*,
        objects::*,
        projectiles::*,
        recon::{self, Beam, Intel},
        remote_view::RemoteView,
        *,
    },
};
pub const NAMES: [&str; 4] = ["Owl Drone", "Shock Bolt", "Recon Bolt", "Hunter's Fury"];
pub const DEFINITIONS: [Ability; 4] = [
    Ability {
        kind: Kind::Active,
        cooldown: 600,
        duration: 160,
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
        cooldown: 800,
        duration: 0,
        charges: 1,
    },
    Ability {
        kind: Kind::Ultimate,
        cooldown: 2400,
        duration: 160,
        charges: 1,
    },
];
#[derive(Clone, Copy)]
pub struct Settings {
    pub drone_ticks: u32,
    pub drone_range: f64,
    pub scan_range: f64,
    pub fury_range: f64,
    pub fury_damage: f32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            drone_ticks: 160,
            drone_range: 32.,
            scan_range: 24.,
            fury_range: 64.,
            fury_damage: 16.,
        }
    }
}
pub struct Sova {
    pub slots: [Runtime; 4],
    pub settings: Settings,
    pub equipped: Option<usize>,
    pub charging: Option<u32>,
    pub bounces: u8,
    pub view: Option<RemoteView>,
    pub anchor: Option<V3>,
    pub drone: Option<u64>,
    pub life: u32,
    pub dart_delay: u32,
    pub fury_time: u32,
    pub fury_shots: u8,
    pub windup: u32,
    pub shot_delay: u32,
}
impl Default for Sova {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|i| Runtime {
                charges: DEFINITIONS[i].charges,
                ..Default::default()
            }),
            settings: Settings::default(),
            equipped: None,
            charging: None,
            bounces: 0,
            view: None,
            anchor: None,
            drone: None,
            life: 0,
            dart_delay: 0,
            fury_time: 0,
            fury_shots: 0,
            windup: 0,
            shot_delay: 0,
        }
    }
}
pub struct Context<'a, W: World> {
    pub world: &'a mut W,
    pub projectiles: &'a mut Vec<Projectile>,
    pub objects: &'a mut Vec<Object>,
    pub next: &'a mut u64,
    pub intel: &'a mut Intel,
    pub beams: &'a mut Vec<Beam>,
    pub bursts: &'a mut Vec<explosions::Explosion>,
}
impl Sova {
    pub fn busy(&self) -> bool {
        self.view.is_some() || self.fury_time > 0 || self.slots.iter().any(|s| s.remaining > 0)
    }
    pub fn cancel(&mut self) {
        self.equipped = None;
        self.charging = None;
        self.view = None;
        self.anchor = None;
        self.drone = None;
        self.fury_time = 0;
        self.fury_shots = 0;
        self.windup = 0;
        self.life = 0;
    }
    pub fn offline(&mut self) {
        self.cancel();
        for (r, d) in self.slots.iter_mut().zip(DEFINITIONS) {
            r.tick(d);
        }
    }
    pub fn activate(&mut self, slot: usize, a: &Actor) -> bool {
        if slot >= 4 || !a.eligible {
            return false;
        }
        if slot == 0 && self.view.is_some() {
            self.cancel();
            return true;
        }
        if slot == 3 && self.fury_time > 0 {
            self.equipped = Some(3);
            return true;
        }
        if self.slots[slot].charges == 0 {
            return false;
        }
        self.cancel();
        self.equipped = Some(slot);
        self.bounces = 0;
        if slot == 3 {
            self.fury_shots = 3;
        }
        true
    }
    pub fn fire(&mut self, alt: bool, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Entity::Player(owner) = a.entity else {
            return false;
        };
        if !a.eligible {
            return false;
        }
        if let Some(v) = &self.view {
            if alt
                || self.dart_delay > 0
                || c.projectiles.len() >= MAX_PROJECTILES
                || c.intel.tags.len() >= recon::MAX_TAGS
            {
                return false;
            }
            c.projectiles.push(Projectile {
                owner,
                kind: ProjectileKind::ReconDart,
                position: add(v.position, scale(v.direction, 0.25)),
                velocity: scale(v.direction, 1.5),
                ticks: 30,
                guided: false,
                victims: vec![],
            });
            self.dart_delay = 40;
            c.world
                .ability_sound(v.position, "minecraft:item.crossbow.shoot");
            return true;
        }
        let Some(slot) = self.equipped else {
            return false;
        };
        if slot == 0 {
            if !a.ground || c.objects.len() >= MAX_OBJECTS {
                return false;
            }
            let at = add(a.eye(), scale(a.direction, 0.8));
            let Some(mut view) =
                RemoteView::new(c.world, at, self.settings.drone_range.clamp(1., 64.))
            else {
                return false;
            };
            if block_hit(c.world, a.eye(), at, 0.15).is_some()
                || !self.slots[0].activate(DEFINITIONS[0])
            {
                return false;
            }
            view.look(a.direction);
            self.view = Some(view);
            self.anchor = Some(a.position);
            *c.next = c.next.saturating_add(1);
            self.drone = Some(*c.next);
            self.life = self.settings.drone_ticks.clamp(1, 300);
            c.objects.push(Object {
                last_hit: None,
                id: *c.next,
                owner,
                kind: ObjectKind::Drone,
                position: add(at, [0., -0.15, 0.]),
                health: 8.,
                ticks: self.life,
                velocity: [0.; 3],
                target: None,
            });
            c.world.ability_sound(at, "minecraft:block.beacon.activate");
            return true;
        }
        if slot == 1 || slot == 2 {
            if alt {
                self.bounces = (self.bounces + 1) % 3;
            } else {
                self.charging.get_or_insert(0);
            }
            return true;
        }
        if alt
            || self.windup > 0
            || self.shot_delay > 0
            || self.fury_shots == 0
            || (self.fury_time > 0 && self.fury_time < 10)
        {
            return false;
        }
        if self.fury_time == 0 {
            if !self.slots[3].activate(DEFINITIONS[3]) {
                return false;
            }
            self.fury_time = 160;
        }
        self.fury_shots -= 1;
        self.windup = 10;
        self.shot_delay = 20;
        c.world
            .ability_sound(a.position, "minecraft:item.trident.thunder");
        true
    }
    pub fn release(&mut self, a: &Actor, c: &mut Context<'_, impl World>) -> bool {
        let Some(power) = self.charging.take() else {
            return true;
        };
        let Some(slot @ (1 | 2)) = self.equipped else {
            return false;
        };
        let Entity::Player(owner) = a.entity else {
            return false;
        };
        if !a.eligible
            || c.projectiles.len() >= MAX_PROJECTILES
            || (slot == 2
                && c.objects.len()
                    + c.projectiles
                        .iter()
                        .filter(|p| matches!(p.kind, ProjectileKind::SovaBolt { recon: true, .. }))
                        .count()
                    >= MAX_OBJECTS)
            || !self.slots[slot].activate(DEFINITIONS[slot])
        {
            return false;
        }
        c.projectiles.push(Projectile {
            owner,
            kind: ProjectileKind::SovaBolt {
                recon: slot == 2,
                bounces: self.bounces,
            },
            position: a.eye(),
            velocity: scale(a.direction, 0.6 + f64::from(power.min(30)) / 30. * 1.4),
            ticks: 100,
            guided: false,
            victims: vec![],
        });
        self.equipped = None;
        c.world
            .ability_sound(a.position, "minecraft:entity.arrow.shoot");
        true
    }
    pub fn tick(&mut self, a: &Actor, c: &mut Context<'_, impl World>) -> Vec<Entity> {
        for (r, d) in self.slots.iter_mut().zip(DEFINITIONS) {
            r.tick(d);
        }
        self.dart_delay = self.dart_delay.saturating_sub(1);
        self.shot_delay = self.shot_delay.saturating_sub(1);
        if let Some(t) = &mut self.charging {
            *t = (*t + 1).min(30);
        }
        if !a.eligible {
            self.cancel();
        }
        let Entity::Player(owner) = a.entity else {
            return vec![];
        };
        if self.view.is_some() {
            self.life = self.life.saturating_sub(1);
            if self.life == 0
                || !c
                    .objects
                    .iter()
                    .any(|o| Some(o.id) == self.drone && o.health > 0. && o.ticks > 0)
                || self.anchor.is_some_and(|p| {
                    distance(p, a.position) > 4.
                        || block_hit(c.world, add(p, [0., 0.05, 0.]), add(p, [0., -0.3, 0.]), 0.)
                            .is_none()
                })
            {
                self.cancel();
            }
        }
        if let Some(v) = &mut self.view {
            v.look(a.direction);
            let vertical = a.direction[1] * dot(horizontal(a.direction), a.movement);
            v.step(
                c.world,
                scale(
                    add(
                        a.movement,
                        [0., vertical + if a.jump { 0.5 } else { 0. }, 0.],
                    ),
                    0.45,
                ),
            );
            if let Some(o) = c.objects.iter_mut().find(|o| Some(o.id) == self.drone) {
                o.position = add(v.position, [0., -0.15, 0.]);
            }
        }
        if let Some(at) = self.anchor {
            c.world.move_player(owner, at);
            c.world.velocity(owner, [0.; 3], false);
        }
        c.world.protect(
            owner,
            effects::Protection {
                health: a.health,
                disarmed: self.view.is_some(),
                ..Default::default()
            },
        );
        let mut kills = vec![];
        if self.fury_time > 0 {
            self.fury_time -= 1;
            c.world.effect(a.entity, "minecraft:slowness", 2, 1);
            if self.windup > 0 {
                self.windup -= 1;
                if self.windup == 0 {
                    let from = a.eye();
                    let to =
                        recon::loaded_end(c.world, from, a.direction, self.settings.fury_range);
                    for t in c
                        .world
                        .actors()
                        .into_iter()
                        .filter(|t| {
                            t.eligible
                                && t.health > 0.
                                && t.entity != a.entity
                                && c.world.can_harm(owner, t.entity)
                                && c.world
                                    .boxes(t.position.map(|n| n.floor() as i32))
                                    .is_some()
                                && t.bounds.expanded(0.8).segment(from, to).is_some()
                        })
                        .collect::<Vec<_>>()
                    {
                        let hit =
                            c.world
                                .hurt(owner, t.entity, self.settings.fury_damage.clamp(0., 30.));
                        if hit.accepted {
                            c.intel.reveal(c.world, owner, t.entity, 30);
                        }
                        if hit.killed {
                            kills.push(t.entity);
                        }
                    }
                    for o in c
                        .objects
                        .iter_mut()
                        .filter(|o| o.hit(from, to, 0.8).is_some())
                    {
                        o.damage(self.settings.fury_damage, Some(Entity::Player(owner)), from);
                    }
                    if c.beams.len() >= recon::MAX_BEAMS {
                        c.beams.remove(0);
                    }
                    c.beams.push(Beam { from, to, ticks: 4 });
                }
            }
            if self.fury_time == 0 || (self.fury_shots == 0 && self.windup == 0) {
                self.equipped = None;
                self.fury_time = 0;
                self.fury_shots = 0;
            }
        }
        self.slots[0].active = if self.view.is_some() { self.life } else { 0 };
        self.slots[3].active = self.fury_time;
        kills
    }
}
/// Utility arrows do not require unfinished weapons and pass bullet-only shields.
pub fn arrow(p: &mut Projectile, c: &mut Context<'_, impl World>) -> bool {
    let ProjectileKind::SovaBolt { recon, bounces } = p.kind else {
        return false;
    };
    p.velocity[1] -= 0.03;
    let end = add(p.position, p.velocity);
    let block = block_collision(c.world, p.position, end, 0.08);
    let actor = c
        .world
        .actors()
        .into_iter()
        .filter(|a| a.eligible && a.health > 0. && a.entity != Entity::Player(p.owner))
        .filter_map(|a| a.bounds.expanded(0.08).segment(p.position, end))
        .min_by(f64::total_cmp);
    let time = match (block, actor) {
        (Some(b), Some(t)) => Some(b.time.min(t)),
        (Some(b), None) => Some(b.time),
        (None, t) => t,
    };
    p.ticks = p.ticks.saturating_sub(1);
    if let Some(t) = time {
        if block.is_some_and(|b| !b.known) {
            return false;
        }
        let at = add(p.position, scale(p.velocity, (t - 0.001).max(0.)));
        if let Some(b) = block.filter(|b| actor.is_none_or(|t| b.time < t))
            && bounces > 0
        {
            p.position = add(at, scale(b.normal, 0.02));
            p.velocity = sub(p.velocity, scale(b.normal, 2. * dot(p.velocity, b.normal)));
            p.kind = ProjectileKind::SovaBolt {
                recon,
                bounces: bounces - 1,
            };
            return p.ticks > 0;
        }
        if recon {
            if c.objects.len() < MAX_OBJECTS {
                *c.next = c.next.saturating_add(1);
                c.objects.push(Object {
                    last_hit: None,
                    id: *c.next,
                    owner: p.owner,
                    kind: ObjectKind::ReconBolt,
                    position: at,
                    health: 1.,
                    ticks: 61,
                    velocity: [0.; 3],
                    target: None,
                });
            }
        } else {
            c.bursts.push(explosions::Explosion {
                owner: p.owner,
                position: at,
                radius: 4.,
                damage: 15.,
                push: 0.,
                self_damage: true,
            });
        }
        c.world
            .ability_sound(at, "minecraft:block.amethyst_block.chime");
        false
    } else {
        p.position = end;
        p.ticks > 0
    }
}
