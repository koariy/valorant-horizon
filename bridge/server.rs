use super::Game;
use horizon_core::geometry::{Aabb, Axis, Vec3};
use valorant_horizon_engine::{
    gameplay::{geometry::*, *},
    server::{self, Session},
};

fn bounds(b: Aabb) -> Box3 {
    Box3 {
        min: [Axis::X, Axis::Y, Axis::Z].map(|a| b.min(a)),
        max: [Axis::X, Axis::Y, Axis::Z].map(|a| b.max(a)),
    }
}
struct Bridge<'a, 'g>(&'a mut Game<'g>);
use horizon_client::mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, Holder, Host, Tag,
};
static WALLS: AttachmentType<String> = AttachmentType {
    namespace: "valorant_horizon",
    name: "barrier_ledger",
    default: || "[]".into(),
    codec: Some(AttachmentCodec {
        encode: |s| Ok([("json".into(), Tag::String(s.as_str().into()))].into()),
        decode: |data| match data.get(&"json".into()) {
            Some(Tag::String(s)) => s.to_string().map_err(|_| AttachmentError::Decode),
            _ => Err(AttachmentError::Decode),
        },
    }),
    copy_on_death: false,
    sync: false,
};
static DECAY: AttachmentType<String> = AttachmentType {
    namespace: "valorant_horizon",
    name: "decay_ledger",
    default: || "null".into(),
    codec: Some(AttachmentCodec {
        encode: |s| Ok([("json".into(), Tag::String(s.as_str().into()))].into()),
        decode: |data| match data.get(&"json".into()) {
            Some(Tag::String(s)) => s.to_string().map_err(|_| AttachmentError::Decode),
            _ => Err(AttachmentError::Decode),
        },
    }),
    copy_on_death: false,
    sync: false,
};
impl Bridge<'_, '_> {
    fn apply_status(
        &mut self,
        target: Entity,
        name: &str,
        ticks: u32,
        amplifier: u8,
        visible: bool,
    ) -> bool {
        if effects::hostile_status(name) && self.status_immune(target) {
            return false;
        }
        let Some(effect) = horizon_client::brewing::effect(name) else {
            return false;
        };
        match target {
            Entity::Player(id) => {
                let Some(t) = self.0.target_by_uuid(id) else {
                    return false;
                };
                self.0
                    .for_player(t, |g| {
                        let mut status = horizon_client::effects::EffectInstance::new(
                            effect.id,
                            ticks as i32,
                            i32::from(amplifier),
                        );
                        status.visible = visible;
                        g.add_player_effect(status)
                    })
                    .unwrap_or(false)
            }
            Entity::Mob(id) => self
                .0
                .mobs
                .iter_mut()
                .find(|m| m.id == id)
                .is_some_and(|m| {
                    m.add_effect_with(effect.id, ticks as i32, i32::from(amplifier), false, false)
                }),
        }
    }
}
impl World for Bridge<'_, '_> {
    fn network_id(&self, target: Entity) -> Option<u32> {
        match target {
            Entity::Mob(id) => Some(id),
            Entity::Player(id) => self
                .0
                .native_server_players()
                .into_iter()
                .find(|p| p.uuid == id)
                .map(|p| p.entity_id as u32),
        }
    }
    fn intel_viewers(&self, owner: u128) -> Vec<u128> {
        let all = self.0.native_server_players();
        let Some(source) = all.iter().find(|p| p.uuid == owner) else {
            return vec![];
        };
        let team = self.0.score.board.players_team(&source.name);
        all.iter()
            .filter(|p| {
                p.dimension == self.0.dim.id()
                    && (p.uuid == owner
                        || team
                            .zip(self.0.score.board.players_team(&p.name))
                            .is_some_and(|(a, b)| a.name == b.name))
            })
            .map(|p| p.uuid)
            .take(64)
            .collect()
    }

    fn hit_credits(&mut self) -> Vec<(u128, Entity)> {
        let Some(l) = self.0.native_level(self.0.dim.id()) else {
            return vec![];
        };
        combat::take(&(l.epoch().get(), self.0.dim.id().into()))
    }
    fn actors(&self) -> Vec<Actor> {
        let game = &self.0;
        let mut actors = Vec::new();
        if let Some(host) = &game.lan {
            for g in host
                .hub
                .guests
                .iter()
                .filter(|g| g.placed && g.dimension == game.dim.id())
            {
                let direction = horizon_world::view_direction::view_vector(g.pitch, g.yaw);
                let yaw = f64::from(g.yaw).to_radians();
                let f = f64::from(i32::from(g.input & 1 != 0) - i32::from(g.input & 2 != 0));
                let s = f64::from(i32::from(g.input & 4 != 0) - i32::from(g.input & 8 != 0));
                actors.push(Actor {
                    entity: Entity::Player(g.uuid),
                    position: g.position,
                    bounds: bounds(g.bounds()),
                    velocity: std::array::from_fn(|i| g.position[i] - g.previous_position[i]),
                    direction: [direction.x, direction.y, direction.z],
                    movement: [
                        -yaw.sin() * f + yaw.cos() * s,
                        0.0,
                        yaw.cos() * f + yaw.sin() * s,
                    ],
                    health: g.health,
                    max_health: g
                        .motion
                        .as_ref()
                        .and_then(|p| p.attributes.cached_value_of("minecraft:max_health").ok())
                        .unwrap_or(20.0) as f32,
                    ground: g.on_ground,
                    jump: g.input & 16 != 0,
                    eligible: !g.dead && g.health > 0.0 && g.game_mode != 3,
                });
            }
        }
        for m in game.mobs.iter().filter(|m| !m.removed && m.alive()) {
            let Some(entity) = game.native_entity(m.id) else {
                continue;
            };
            let Some(b) = game.native_entity_bounding_box(entity) else {
                continue;
            };
            actors.push(Actor {
                entity: Entity::Mob(m.id),
                position: [m.position.x, m.position.y, m.position.z],
                bounds: Box3 {
                    min: b[0],
                    max: b[1],
                },
                velocity: [m.velocity.x, m.velocity.y, m.velocity.z],
                direction: [0.0, 0.0, 1.0],
                movement: [0.0; 3],
                health: m.health,
                max_health: m.attrs.max_health,
                ground: false,
                jump: false,
                eligible: true,
            });
        }
        actors
    }
    fn boxes(&self, cell: [i32; 3]) -> Option<Vec<Box3>> {
        let level = self.0.native_level(self.0.dim.id())?;
        if !self.0.native_level_is_loaded(level, cell) {
            return None;
        }
        let offset = cell.map(f64::from);
        Some(
            self.0
                .native_block_collision(cell)
                .into_iter()
                .map(|b| Box3 {
                    min: add([b[0], b[1], b[2]], offset),
                    max: add([b[3], b[4], b[5]], offset),
                })
                .collect(),
        )
    }
    fn move_player(&mut self, id: u128, position: V3) -> bool {
        let Some(target) = self.0.target_by_uuid(id) else {
            return false;
        };
        self.0.teleport_to_in_level(target, position, None, false);
        true
    }
    fn teleport_player(&mut self, id: u128, position: V3) -> bool {
        let Some(level) = self.0.native_level(self.0.dim.id()) else {
            return false;
        };
        let key = (level.epoch().get(), self.0.dim.id().to_owned(), id);
        if !self.move_player(id, position) {
            return false;
        }
        teleport::record(key);
        true
    }
    fn teleport_stamp(&self, target: Entity) -> u64 {
        let Entity::Player(id) = target else {
            return 0;
        };
        let Some(level) = self.0.native_level(self.0.dim.id()) else {
            return 0;
        };
        teleport::stamp(&(level.epoch().get(), self.0.dim.id().to_owned(), id))
    }
    fn velocity(&mut self, id: u128, v: V3, clear_fall: bool) -> bool {
        let game = &mut self.0;
        let Some(target) = game.target_by_uuid(id) else {
            return false;
        };
        let entity = game
            .native_server_players()
            .into_iter()
            .find(|p| p.uuid == id)
            .map(|p| p.entity_id);
        let changed = game
            .for_player(target, |g| {
                let Some(p) = g.player.as_mut() else {
                    return false;
                };
                p.velocity = Vec3::new(v[0], v[1], v[2]);
                if clear_fall {
                    p.fall_distance = 0.0;
                }
                if v[1] > 0.0 {
                    p.ground.on_ground = false;
                }
                true
            })
            .unwrap_or(false);
        if changed && let Some(entity) = entity {
            game.native_send(
                id,
                horizon_protocol::packets::play::Clientbound::SetEntityMotion(
                    horizon_protocol::packets::play::SetEntityMotion {
                        id: entity,
                        velocity: v,
                    },
                ),
            );
        }
        changed
    }
    fn can_harm(&self, owner: u128, target: Entity) -> bool {
        if let Entity::Player(id) = target {
            let all = self.0.native_server_players();
            let Some(victim) = all.iter().find(|p| p.uuid == id) else {
                return false;
            };
            if effects::protections()
                .lock()
                .ok()
                .is_some_and(|p| p.get(&victim.entity_id).is_some_and(|p| p.invulnerable))
            {
                return false;
            }
            if victim.game_mode == 1 || victim.game_mode == 3 {
                return false;
            }
            if id == owner {
                return true;
            }
            let teams = |id| {
                all.iter()
                    .find(|p| p.uuid == id)
                    .and_then(|p| self.0.score.board.players_team(&p.name))
            };
            self.0.ui.rules.bool("pvp")
                && horizon_client::scoreboard::teams_allow_damage(teams(id), teams(owner))
        } else if let Entity::Mob(id) = target {
            self.0
                .mobs
                .iter()
                .find(|m| m.id == id)
                .is_some_and(|m| !m.invulnerable_flag && !m.removed && m.alive())
        } else {
            false
        }
    }
    fn bot_target(&self, owner: u128, target: Entity) -> bool {
        match target {
            Entity::Mob(id) => self
                .0
                .mobs
                .iter()
                .find(|m| m.id == id)
                .is_some_and(|m| m.hostile()),
            _ => self.can_harm(owner, target),
        }
    }
    fn push_entity(&mut self, target: Entity, velocity: V3) -> bool {
        match target {
            Entity::Player(id) => self.velocity(id, velocity, false),
            Entity::Mob(id) => self
                .0
                .native_entity(id)
                .is_some_and(|entity| self.0.native_entity_set_velocity(entity, velocity)),
        }
    }
    fn ability_sound(&mut self, at: V3, sound: &str) {
        self.0.native_level_play_sound(sound, at, 7, 1., 1.);
    }
    fn burst_fx(&mut self, at: V3) {
        let p = horizon_client::particles::ParticleOptions::simple(
            horizon_client::particles::ParticleType::Explosion,
        );
        self.0
            .native_level_send_particles(&p, None, at, [0.1; 3], 0.0, 1);
        self.0
            .native_level_play_sound("minecraft:entity.generic.explode", at, 7, 1.0, 1.0);
    }
    fn now(&self) -> i64 {
        self.0.world.game_time
    }
    fn block(&self, cell: [i32; 3]) -> Option<String> {
        let level = self.0.native_level(self.0.dim.id())?;
        self.0.native_level_block(level, cell)
    }
    fn set_block(&mut self, cell: [i32; 3], name: &str) -> bool {
        if self.block(cell).is_none() {
            return false;
        }
        let Some(state) = self.0.pack.assets.registry.default_state(name) else {
            return false;
        };
        self.0.native_set_block(self.0.dim.id(), cell, state, 3)
    }
    fn saved_blocks(&mut self) -> Result<Vec<barrier::OwnedBlock>, String> {
        let host: &mut dyn Host = &mut *self.0;
        let data = WALLS
            .get(host, Holder::Level)
            .map_err(|e| format!("{e:?}"))?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    }
    fn save_blocks(&mut self, blocks: &[barrier::OwnedBlock]) -> bool {
        if self.saved_blocks().is_err() {
            return false;
        }
        let Ok(data) = serde_json::to_string(blocks) else {
            return false;
        };
        let host: &mut dyn Host = &mut *self.0;
        WALLS.set(host, Holder::Level, data).is_ok()
    }
    fn saved_decay(&mut self) -> Result<Vec<(Entity, toxin::Decay)>, String> {
        let actors = self.actors();
        let mut result = Vec::new();
        for a in actors {
            let holder = match a.entity {
                Entity::Player(id) => Holder::Player(id),
                Entity::Mob(id) => Holder::Entity(id),
            };
            let host: &mut dyn Host = &mut *self.0;
            let data = DECAY.get(host, holder).map_err(|e| format!("{e:?}"))?;
            let debt: Option<toxin::Decay> =
                serde_json::from_str(&data).map_err(|e| e.to_string())?;
            if let Some(d) = debt {
                result.push((a.entity, d));
            }
        }
        Ok(result)
    }
    fn save_decay(&mut self, entries: &[(Entity, toxin::Decay)]) -> bool {
        let actors = self.actors();
        let mut okay = true;
        for a in actors {
            let holder = match a.entity {
                Entity::Player(id) => Holder::Player(id),
                Entity::Mob(id) => Holder::Entity(id),
            };
            let debt = entries.iter().find(|(e, _)| *e == a.entity).map(|(_, d)| d);
            let Ok(data) = serde_json::to_string(&debt) else {
                return false;
            };
            let host: &mut dyn Host = &mut *self.0;
            okay &= DECAY.set(host, holder, data).is_ok();
        }
        okay
    }
    fn vulnerability(&mut self, target: Entity, active: bool) {
        let id = match target {
            Entity::Mob(id) => Some(id as i32),
            Entity::Player(id) => self
                .0
                .native_server_players()
                .iter()
                .find(|p| p.uuid == id)
                .map(|p| p.entity_id),
        };
        let id = if let Ok(mut ids) = effects::vulnerability_ids().lock() {
            let id = id.or_else(|| ids.get(&target).copied());
            if active {
                if let Some(id) = id {
                    ids.insert(target, id);
                }
            } else {
                ids.remove(&target);
            }
            id
        } else {
            id
        };
        if let Some(id) = id
            && let Ok(mut v) = effects::vulnerable().lock()
        {
            if active {
                v.insert(id, true);
            } else {
                v.remove(&id);
            }
        }
    }
    fn block_arrows(
        &mut self,
        walls: &[divide::Divide],
        shields: &mut Vec<shields::Shield>,
        objects: &mut Vec<objects::Object>,
    ) {
        let ids: std::collections::HashSet<_> = self
            .0
            .creatures
            .projectiles
            .iter()
            .filter(|p| {
                p.arrow
                    .as_ref()
                    .is_some_and(|a| !a.in_ground && !a.no_physics && !a.removed && !a.dealt_damage)
            })
            .filter_map(|p| {
                let from = [p.position.x, p.position.y, p.position.z];
                let v = [p.velocity.x, p.velocity.y, p.velocity.z];
                let to = add(from, v);
                let damage = match p.kind {
                    horizon_client::projectiles::ProjectileKind::Arrow { base_damage, .. } => {
                        (length(v) * base_damage).ceil() as f32
                    }
                    _ => 8.,
                };
                projectiles::intercept(
                    self,
                    shields,
                    walls,
                    objects,
                    projectiles::Shot {
                        attacker: match p.owner {
                            Some(horizon_client::mobs::Who::Mob(id)) => Some(Entity::Mob(id)),
                            Some(horizon_client::mobs::Who::Player) => p.owner_uuid.map(|parts| {
                                Entity::Player(
                                    parts
                                        .into_iter()
                                        .fold(0u128, |uuid, n| (uuid << 32) | u128::from(n as u32)),
                                )
                            }),
                            None => None,
                        },
                        from,
                        to,
                        damage,
                        padding: shields::CollisionPadding {
                            actors: 0.3,
                            terrain: 0.,
                        },
                    },
                )
                .then_some(p.id)
            })
            .collect();
        self.0
            .creatures
            .projectiles
            .retain(|p| !ids.contains(&p.id));
    }
    fn temporary_health(&mut self, target: Entity, value: f32) -> bool {
        if !value.is_finite() || value < 1. {
            return false;
        }
        match target {
            Entity::Player(id) => {
                let Some(t) = self.0.target_by_uuid(id) else {
                    return false;
                };
                let changed = self
                    .0
                    .for_player(t, |g| {
                        g.player
                            .as_mut()
                            .is_some_and(|p| p.set_health(value, false).is_ok())
                    })
                    .unwrap_or(false);
                if changed
                    && let Some(player) =
                        self.0.native_server_players().iter().find(|p| p.uuid == id)
                    && let Ok(mut all) = effects::protections().lock()
                    && let Some(p) = all.get_mut(&player.entity_id)
                {
                    p.health = value;
                }
                changed
            }
            Entity::Mob(id) => self.0.native_set_mob_health(self.0.dim.id(), id, value),
        }
    }
    fn heal(&mut self, target: Entity, amount: f32) -> bool {
        match target {
            Entity::Player(id) => {
                let Some(t) = self.0.target_by_uuid(id) else {
                    return false;
                };
                self.0
                    .for_player(t, |g| {
                        let before = g.player.as_ref().map_or(0., |p| p.health.health());
                        g.apply_effect_action(horizon_client::effects::EffectAction::Heal(amount));
                        g.player
                            .as_ref()
                            .is_some_and(|p| p.health.health() > before)
                    })
                    .unwrap_or(false)
            }
            Entity::Mob(id) => {
                let dim = self.0.dim.id();
                let Some(m) = self.0.native_mob(dim, id) else {
                    return false;
                };
                self.0.native_set_mob_health(dim, id, m.health + amount)
            }
        }
    }
    fn absorb(&mut self, owner: u128, delta: f32) -> f32 {
        let Some(target) = self.0.target_by_uuid(owner) else {
            return 0.;
        };
        self.0
            .for_player(target, |g| {
                let Some(p) = g.player.as_mut() else {
                    return 0.;
                };
                let old = p.health.absorption();
                let value = (old + delta).max(0.);
                if p.internal_set_absorption(value).is_ok() {
                    value - old
                } else {
                    0.
                }
            })
            .unwrap_or(0.)
    }
    fn effect(&mut self, target: Entity, name: &str, ticks: u32, amplifier: u8) -> bool {
        self.apply_status(target, name, ticks, amplifier, true)
    }
    fn single_hit_shield(&self, owner: u128) -> bool {
        self.0
            .native_server_players()
            .iter()
            .find(|p| p.uuid == owner)
            .is_some_and(|p| {
                effects::protections()
                    .lock()
                    .ok()
                    .is_some_and(|all| all.get(&p.entity_id).is_some_and(|p| p.one_hit))
            })
    }
    fn status_immune(&self, target: Entity) -> bool {
        let Entity::Player(id) = target else {
            return false;
        };
        self.0
            .native_server_players()
            .iter()
            .find(|p| p.uuid == id)
            .is_some_and(|p| {
                effects::protections()
                    .lock()
                    .ok()
                    .is_some_and(|all| all.get(&p.entity_id).is_some_and(|s| s.invulnerable))
            })
    }
    fn quiet_effect(&mut self, target: Entity, name: &str, ticks: u32, amplifier: u8) -> bool {
        self.apply_status(target, name, ticks, amplifier, false)
    }
    fn protection_state(&self, owner: u128) -> effects::Protection {
        self.0
            .native_server_players()
            .iter()
            .find(|p| p.uuid == owner)
            .and_then(|p| {
                effects::protections()
                    .lock()
                    .ok()
                    .and_then(|all| all.get(&p.entity_id).copied())
            })
            .unwrap_or_default()
    }
    fn protect(&mut self, owner: u128, mut state: effects::Protection) {
        if let Some(p) = self
            .0
            .native_server_players()
            .iter()
            .find(|p| p.uuid == owner)
            && let Ok(mut all) = effects::protections().lock()
        {
            state.health = p.health;
            let state = effects::merge_protection(all.get(&p.entity_id).copied(), state);
            all.insert(p.entity_id, state);
        }
    }
    fn return_requested(&self, owner: u128) -> bool {
        self.0
            .native_server_players()
            .iter()
            .find(|p| p.uuid == owner)
            .is_some_and(|p| {
                effects::protections()
                    .lock()
                    .ok()
                    .is_some_and(|all| all.get(&p.entity_id).is_some_and(|p| p.requested))
            })
    }
    fn hurt_with(&mut self, owner: u128, target: Entity, damage: f32, kind: &str) -> Hit {
        self.0.vh_damage(owner, target, damage, kind)
    }
    fn hurt(&mut self, owner: u128, target: Entity, damage: f32) -> Hit {
        self.0.vh_damage(owner, target, damage, "minecraft:arrow")
    }
}
impl Game<'_> {
    pub(super) fn vh_control_locked(&self) -> bool {
        let acting = self.native_acting_entity();
        let id = if acting != 0 {
            acting as i32
        } else {
            self.player.as_ref().map_or(0, |p| p.entity_id as i32)
        };
        effects::actions_locked(id)
    }

    pub(super) fn vh_flush_barriers(&mut self) {
        let Some(level) = self.native_level(self.dim.id()) else {
            return;
        };
        let epoch = level.epoch().get();
        let back = self.dim;
        let ledgers = server::sessions()
            .lock()
            .ok()
            .map(|all| {
                all.iter()
                    .filter(|(key, s)| key.0 == epoch && s.blocks_loaded)
                    .map(|(key, s)| (key.1.clone(), s.blocks.clone()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for (dimension, blocks) in ledgers {
            if self
                .enter_level(None, super::Dim::from_id(&dimension))
                .is_ok()
            {
                let _ = Bridge(self).save_blocks(&blocks);
            }
        }
        let _ = self.enter_level(None, back);
    }
    pub(super) fn vh_server_tick(&mut self, dimension: &str) {
        if self.native_window()
            || !self.native_authoritative()
            || dimension != self.dim.id()
            || self.level_pass.running == Some(false)
        {
            return;
        }
        let Some(level) = self.native_level(dimension) else {
            return;
        };
        let key = (level.epoch().get(), dimension.to_owned());
        server::ACTIVE_EPOCH.store(level.epoch().get(), std::sync::atomic::Ordering::Relaxed);
        // Never hold the state lock across Horizon calls: damage may synchronously fire hooks.
        let mut session = {
            let Ok(mut sessions) = server::sessions().lock() else {
                return;
            };
            sessions.remove(&key).unwrap_or_default()
        };
        if let Ok(mut events) = deaths::events().lock() {
            let mut rest = Vec::new();
            for e in events.drain(..) {
                if e.key == key {
                    session.queue_death(e.owner, e.target, e.position, false);
                } else {
                    rest.push(e);
                }
            }
            *events = rest;
        }
        let mut ids: Vec<_> = self
            .native_server_players()
            .into_iter()
            .filter(|p| p.dimension == dimension)
            .map(|p| (p.entity_id, Entity::Player(p.uuid)))
            .collect();
        ids.extend(self.mobs.iter().map(|m| (m.id as i32, Entity::Mob(m.id))));
        combat::register(key.clone(), ids);
        session.tick(&mut Bridge(self));
        for player in self
            .native_server_players()
            .into_iter()
            .filter(|p| p.dimension == dimension)
        {
            if let Some(snapshot) = session.snapshot(player.uuid)
                && let Ok(data) = serde_json::to_vec(&snapshot)
                && data.len() < 30_000
            {
                let _ = self.native_net_send_to_player(player.uuid, server::CHANNEL, &data);
            }
        }
        if let Ok(mut sessions) = server::sessions().lock() {
            sessions.insert(key, session);
        }
    }
    pub(super) fn vh_request(&mut self, id: u128, channel: &str, data: &[u8]) {
        if channel != server::CHANNEL || data.len() > 512 || self.native_window() {
            return;
        }
        let Ok(request) = serde_json::from_slice::<Request>(data) else {
            return;
        };
        let Some(player) = self.native_server_players().into_iter().find(|p| {
            p.uuid == id
                && p.game_mode != 3
                && (p.health > 0.
                    || matches!(
                        request.action,
                        Action::Activate(2)
                            | Action::Fire(_)
                            | Action::Aim(_, _)
                            | Action::Cancel
                            | Action::Release(_)
                    ))
        }) else {
            return;
        };
        let Some(level) = self.native_level(&player.dimension) else {
            return;
        };
        let Some(connection) = self
            .lan
            .as_ref()
            .and_then(|h| h.hub.guests.iter().find(|g| g.uuid == id).map(|g| g.conn))
        else {
            return;
        };
        if let Ok(mut sessions) = server::sessions().lock() {
            let key = (level.epoch().get(), player.dimension);
            let mut moved = None;
            for (old, session) in sessions.iter_mut() {
                if old.0 == key.0
                    && old.1 != key.1
                    && let Some(mut p) = session.players.remove(&id)
                {
                    p.change_dimension();
                    moved = Some(p);
                    break;
                }
            }
            let session = sessions.entry(key).or_insert_with(Session::default);
            if let Some(p) = moved {
                session.players.insert(id, p);
            }
            if session.players.len() >= 64 && !session.players.contains_key(&id) {
                return;
            }
            let p = session.players.entry(id).or_default();
            if p.connection != connection {
                p.connection = connection;
                p.sequence = 0;
            }
            session.enqueue(id, request);
        }
    }
}

impl Game<'_> {
    pub(super) fn vh_mob_credit(&self, m: &horizon_client::mobs::Mob) -> Option<deaths::Death> {
        if m.health > 0. {
            return None;
        }
        let player_blow = matches!(
            m.last_damage,
            Some((
                horizon_client::mobs::Damage {
                    attacker: Some(horizon_client::mobs::Who::Player),
                    ..
                },
                _
            ))
        );
        let owner = if player_blow {
            self.last_hurt_by_player
                .get(&m.id)
                .copied()
                .flatten()
                .and_then(|conn| {
                    self.lan
                        .as_ref()?
                        .hub
                        .guests
                        .iter()
                        .find(|g| g.conn == conn)
                        .map(|g| g.uuid)
                })
                .unwrap_or(0)
        } else {
            0
        };
        let level = self.native_level(self.dim.id())?;
        Some(deaths::Death {
            key: (level.epoch().get(), self.dim.id().into()),
            owner,
            target: Entity::Mob(m.id),
            position: [m.position.x, m.position.y + 0.5, m.position.z],
        })
    }
}

impl Game<'_> {
    pub(super) fn vh_block_cosmic_projectiles(&mut self) {
        if !self.native_authoritative() {
            return;
        }
        let Some(level) = self.native_level(self.dim.id()) else {
            return;
        };
        let key = (level.epoch().get(), self.dim.id().into());
        let mut session = {
            let Ok(mut all) = server::sessions().lock() else {
                return;
            };
            let Some(s) = all.remove(&key) else {
                return;
            };
            s
        };
        if !session.divides.is_empty() || !session.shields.is_empty() || !session.objects.is_empty()
        {
            Bridge(self).block_arrows(&session.divides, &mut session.shields, &mut session.objects);
        }
        if let Ok(mut all) = server::sessions().lock() {
            all.insert(key, session);
        }
    }
}
