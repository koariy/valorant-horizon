#![allow(dead_code)]
pub mod extra;
use valorant_horizon_engine::gameplay::{geometry::*, *};
pub fn actor(id: u128, p: V3) -> Actor {
    Actor {
        entity: Entity::Player(id),
        position: p,
        bounds: Box3 {
            min: add(p, [-0.3, 0.0, -0.3]),
            max: add(p, [0.3, 1.8, 0.3]),
        },
        direction: [0.0, 0.0, 1.0],
        movement: [0.0; 3],
        velocity: [0.0; 3],
        health: 20.0,
        max_health: 20.0,
        ground: true,
        jump: false,
        eligible: true,
    }
}
pub struct TestWorld {
    pub teleports: std::collections::HashMap<Entity, u64>,
    pub intel: std::collections::HashMap<u128, Vec<u128>>,
    pub unloaded_z: Option<i32>,
    pub actors: Vec<Actor>,
    pub blocks: std::collections::HashMap<[i32; 3], String>,
    pub ledger: Vec<barrier::OwnedBlock>,
    pub decay: Vec<(Entity, toxin::Decay)>,
    pub protected: Vec<Entity>,
    pub shield: f32,
    pub time: i64,

    pub wall: Option<Box3>,
    pub requested: bool,
    pub effects: Vec<(Entity, String)>,
    pub damage: Vec<(Entity, f32, String)>,
}
impl Default for TestWorld {
    fn default() -> Self {
        Self {
            teleports: Default::default(),
            intel: Default::default(),
            unloaded_z: None,
            blocks: Default::default(),
            ledger: vec![],
            decay: vec![],
            protected: vec![],
            shield: 0.,
            time: 0,
            actors: vec![actor(1, [0.0, 1.0, 0.0]), actor(2, [1.0, 1.0, 1.0])],
            wall: None,
            requested: false,
            effects: vec![],
            damage: vec![],
        }
    }
}
impl World for TestWorld {
    fn intel_viewers(&self, owner: u128) -> Vec<u128> {
        self.intel.get(&owner).cloned().unwrap_or(vec![owner])
    }
    fn network_id(&self, e: Entity) -> Option<u32> {
        match e {
            Entity::Player(id) => u32::try_from(id).ok(),
            Entity::Mob(id) => Some(id),
        }
    }
    fn can_harm(&self, _owner: u128, target: Entity) -> bool {
        !self.protected.contains(&target)
    }
    fn now(&self) -> i64 {
        self.time
    }
    fn block(&self, cell: [i32; 3]) -> Option<String> {
        Some(self.blocks.get(&cell).cloned().unwrap_or_else(|| {
            if cell[1] <= 0 {
                "minecraft:stone".into()
            } else {
                "minecraft:air".into()
            }
        }))
    }
    fn set_block(&mut self, cell: [i32; 3], kind: &str) -> bool {
        self.blocks.insert(cell, kind.into());
        true
    }
    fn saved_blocks(&mut self) -> Result<Vec<barrier::OwnedBlock>, String> {
        Ok(self.ledger.clone())
    }
    fn save_blocks(&mut self, blocks: &[barrier::OwnedBlock]) -> bool {
        self.ledger = blocks.to_vec();
        true
    }
    fn actors(&self) -> Vec<Actor> {
        self.actors.clone()
    }
    fn boxes(&self, cell: [i32; 3]) -> Option<Vec<Box3>> {
        if self.unloaded_z.is_some_and(|z| cell[2] >= z) {
            return None;
        }
        let mut out = Vec::new();
        if cell[1] == 0 || self.blocks.get(&cell).is_some_and(|b| b != "minecraft:air") {
            let p = cell.map(f64::from);
            out.push(Box3 {
                min: p,
                max: add(p, [1.0; 3]),
            });
        }
        if let Some(b) = self.wall {
            out.push(b);
        }
        Some(out)
    }
    fn move_player(&mut self, id: u128, p: V3) -> bool {
        let Some(a) = self
            .actors
            .iter_mut()
            .find(|a| a.entity == Entity::Player(id))
        else {
            return false;
        };
        a.bounds = a.bounds.translated(sub(p, a.position));
        a.position = p;
        true
    }
    fn teleport_player(&mut self, id: u128, position: V3) -> bool {
        if !self.move_player(id, position) {
            return false;
        }
        *self.teleports.entry(Entity::Player(id)).or_default() += 1;
        true
    }
    fn teleport_stamp(&self, target: Entity) -> u64 {
        self.teleports.get(&target).copied().unwrap_or(0)
    }
    fn velocity(&mut self, id: u128, v: V3, _: bool) -> bool {
        let a = self
            .actors
            .iter_mut()
            .find(|a| a.entity == Entity::Player(id))
            .unwrap();
        a.velocity = v;
        true
    }
    fn hurt(&mut self, owner: u128, t: Entity, d: f32) -> Hit {
        self.hurt_with(owner, t, d, "minecraft:arrow")
    }
    fn hurt_with(&mut self, _: u128, t: Entity, d: f32, kind: &str) -> Hit {
        self.damage.push((t, d, kind.into()));
        let Some(a) = self.actors.iter_mut().find(|a| a.entity == t) else {
            return Hit::default();
        };
        let before = a.health;
        a.health = (a.health - d).max(0.0);
        Hit {
            accepted: before > 0.0,
            killed: before > 0.0 && a.health == 0.0,
        }
    }
    fn absorb(&mut self, _owner: u128, delta: f32) -> f32 {
        let old = self.shield;
        self.shield = (old + delta).max(0.);
        self.shield - old
    }
    fn saved_decay(&mut self) -> Result<Vec<(Entity, toxin::Decay)>, String> {
        Ok(self.decay.clone())
    }
    fn save_decay(&mut self, d: &[(Entity, toxin::Decay)]) -> bool {
        self.decay = d.to_vec();
        true
    }
    fn temporary_health(&mut self, t: Entity, value: f32) -> bool {
        if let Some(a) = self
            .actors
            .iter_mut()
            .find(|a| a.entity == t && a.health > 0.)
        {
            a.health = value;
            true
        } else {
            false
        }
    }
    fn heal(&mut self, t: Entity, d: f32) -> bool {
        let Some(a) = self
            .actors
            .iter_mut()
            .find(|a| a.entity == t && a.health > 0.0)
        else {
            return false;
        };
        a.health = (a.health + d).min(a.max_health);
        true
    }
    fn effect(&mut self, t: Entity, name: &str, _: u32, _: u8) -> bool {
        self.effects.push((t, name.into()));
        true
    }
    fn return_requested(&self, _: u128) -> bool {
        self.requested
    }
}
