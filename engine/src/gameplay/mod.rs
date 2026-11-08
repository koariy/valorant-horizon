//! Authoritative simulation. This trait is ours, not an invented Horizon API.
pub mod barrier;
pub mod combat;
pub mod deaths;
pub mod divide;
pub mod effects;
pub const MAX_OBJECTS: usize = 16;
pub const MAX_PROJECTILES: usize = 24;
pub const MAX_ZONES: usize = 24;
pub const MAX_SMOKES: usize = 12;
pub mod explosions;
pub mod geometry;
pub mod objects;
pub mod projectiles;
pub mod recon;
pub mod remote_view;
pub mod shields;
pub mod smoke;
pub mod teleport;
pub mod toxin;
pub mod zones;
use geometry::{Box3, V3};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Entity {
    Player(u128),
    Mob(u32),
}
#[derive(Clone, Debug)]
pub struct Actor {
    pub entity: Entity,
    pub position: V3,
    pub bounds: Box3,
    pub velocity: V3,
    pub direction: V3,
    pub movement: V3,
    pub health: f32,
    pub max_health: f32,
    pub ground: bool,
    pub jump: bool,
    pub eligible: bool,
}
impl Actor {
    pub fn eye(&self) -> V3 {
        geometry::add(
            self.position,
            [0.0, (self.bounds.max[1] - self.bounds.min[1]) * 0.9, 0.0],
        )
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Hit {
    pub accepted: bool,
    pub killed: bool,
}
/// Missing chunks must return None. Mutations return success, never silently pretend.
pub trait World {
    fn actors(&self) -> Vec<Actor>;
    fn recon_target(&self, owner: u128, target: Entity) -> bool {
        target != Entity::Player(owner)
            && self.can_harm(owner, target)
            && match target {
                Entity::Player(id) => !self.intel_viewers(owner).contains(&id),
                _ => true,
            }
    }
    fn network_id(&self, _target: Entity) -> Option<u32> {
        None
    }
    fn intel_viewers(&self, owner: u128) -> Vec<u128> {
        vec![owner]
    }
    fn hit_credits(&mut self) -> Vec<(u128, Entity)> {
        vec![]
    }
    fn boxes(&self, cell: [i32; 3]) -> Option<Vec<Box3>>;
    fn move_player(&mut self, player: u128, position: V3) -> bool;
    fn velocity(&mut self, player: u128, velocity: V3, clear_fall: bool) -> bool;
    fn block(&self, _cell: [i32; 3]) -> Option<String> {
        None
    }
    fn set_block(&mut self, _cell: [i32; 3], _kind: &str) -> bool {
        false
    }
    fn saved_blocks(&mut self) -> Result<Vec<barrier::OwnedBlock>, String> {
        Ok(Vec::new())
    }
    fn save_blocks(&mut self, _blocks: &[barrier::OwnedBlock]) -> bool {
        false
    }
    fn ability_sound(&mut self, _at: V3, _sound: &str) {}
    fn burst_fx(&mut self, _at: V3) {}
    fn bot_target(&self, owner: u128, target: Entity) -> bool {
        self.can_harm(owner, target)
    }
    fn can_harm(&self, _owner: u128, _target: Entity) -> bool {
        true
    }
    fn push_entity(&mut self, target: Entity, velocity: V3) -> bool {
        if let Entity::Player(id) = target {
            self.velocity(id, velocity, false)
        } else {
            false
        }
    }
    fn now(&self) -> i64 {
        0
    }
    fn absorb(&mut self, _owner: u128, _delta: f32) -> f32 {
        0.0
    }
    fn saved_decay(&mut self) -> Result<Vec<(Entity, toxin::Decay)>, String> {
        Ok(vec![])
    }
    fn save_decay(&mut self, _entries: &[(Entity, toxin::Decay)]) -> bool {
        true
    }
    fn vulnerability(&mut self, _target: Entity, _active: bool) {}
    fn block_arrows(
        &mut self,
        _walls: &[crate::gameplay::divide::Divide],
        _shields: &mut Vec<shields::Shield>,
        _objects: &mut Vec<objects::Object>,
    ) {
    }
    fn temporary_health(&mut self, _target: Entity, _value: f32) -> bool {
        false
    }
    fn heal(&mut self, _target: Entity, _amount: f32) -> bool {
        false
    }
    fn effect(&mut self, _target: Entity, _name: &str, _ticks: u32, _amplifier: u8) -> bool {
        false
    }
    fn protect(&mut self, _owner: u128, _state: effects::Protection) {}
    fn return_requested(&self, _owner: u128) -> bool {
        false
    }
    fn hurt_with(&mut self, owner: u128, target: Entity, damage: f32, _kind: &str) -> Hit {
        self.hurt(owner, target, damage)
    }
    fn hurt(&mut self, owner: u128, target: Entity, damage: f32) -> Hit;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub expected_agent: usize,
    pub sequence: u64,
    pub action: Action,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Action {
    Select(usize),
    Activate(usize),
    Release(usize),
    Fire(bool),
    Cancel,
    Recall,
    Aim(i8, i8),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub agent: usize,
    pub kit: [u32; 4],
    pub contacts: Vec<recon::Contact>,
    pub beams: Vec<recon::Beam>,
    pub bow_charge: Option<u32>,
    pub bounces: u8,
    pub fury_shots: u8,
    pub after_death: bool,
    pub clove_marks: u8,
    pub sequence: u64,
    pub equipped: bool,
    pub charges: [u8; 4],
    pub cooldowns: [u32; 4],
    pub active: [u32; 4],
    pub knives: u8,
    pub fuel: Option<f32>,
    pub stars: Vec<crate::agents::astra::Star>,
    pub star_count: u8,
    pub camera: Option<[f64; 3]>,
    pub divides: Vec<crate::gameplay::divide::Divide>,
    pub shields: Vec<shields::Shield>,
    pub toxins: Vec<toxin::Field>,
    pub smokes: Vec<smoke::Smoke>,
    pub projectiles: Vec<projectiles::Projectile>,
    pub error: Option<String>,
    pub zones: Vec<zones::Zone>,
    pub objects: Vec<objects::Object>,
    pub preview: Option<smoke::Smoke>,
    pub vision: effects::Vision,
}
