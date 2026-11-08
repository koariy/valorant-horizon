//! Additional kits share the existing authoritative World and resource pools.
pub mod cypher;
use crate::{
    abilities::{Ability, Runtime},
    gameplay::{
        geometry::*,
        objects::*,
        projectiles::*,
        recon::{Beam, Intel},
        remote_view::RemoteView,
        smoke::Smoke,
        zones::Zone,
        *,
    },
};
pub const FIRST: usize = 12;
pub const MAX_CORPSES: usize = 32;
#[derive(Clone)]
pub struct Corpse {
    pub target: Entity,
    pub killer: u128,
    pub at: V3,
    pub ticks: u32,
    pub used: bool,
}
impl Corpse {
    pub fn enemy(&self, w: &impl World, owner: u128) -> bool {
        !self.used
            && (self.killer == owner
                || w.intel_viewers(self.killer).contains(&owner)
                || matches!(self.target, Entity::Player(_)) && w.recon_target(owner, self.target))
    }
}
#[derive(Clone)]
pub struct Core {
    pub slots: [Runtime; 4],
    pub defs: [Ability; 4],
    pub equipped: Option<usize>,
    pub firing: bool,
    pub charge: u32,
}
impl Core {
    pub fn new(defs: [Ability; 4]) -> Self {
        Self {
            slots: std::array::from_fn(|i| Runtime {
                charges: defs[i].charges,
                ..Default::default()
            }),
            defs,
            equipped: None,
            firing: false,
            charge: 0,
        }
    }
    pub fn tick(&mut self) {
        for (r, d) in self.slots.iter_mut().zip(self.defs) {
            r.tick(d);
        }
        if self.firing {
            self.charge = (self.charge + 1).min(40);
        }
    }
    pub fn equip(&mut self, slot: usize) -> bool {
        if slot >= 4 || self.slots[slot].charges == 0 {
            return false;
        }
        self.equipped = Some(slot);
        self.firing = false;
        self.charge = 0;
        true
    }
    pub fn spend(&mut self, slot: usize) -> bool {
        slot < 4 && self.slots[slot].activate(self.defs[slot])
    }
    pub fn cancel(&mut self) {
        self.equipped = None;
        self.firing = false;
        self.charge = 0;
    }
    pub fn busy(&self) -> bool {
        self.slots.iter().any(|s| s.remaining > 0 || s.active > 0)
    }
}
pub struct Budget {
    pub rays: u32,
    pub actors: u32,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            rays: 128,
            actors: 4096,
        }
    }
}
pub struct Context<'a, W: World> {
    pub world: &'a mut W,
    pub actors: &'a [Actor],
    pub objects: &'a mut Vec<Object>,
    pub next: &'a mut u64,
    pub projectiles: &'a mut Vec<Projectile>,
    pub zones: &'a mut Vec<Zone>,
    pub smokes: &'a mut Vec<Smoke>,
    pub intel: &'a mut Intel,
    pub beams: &'a mut Vec<Beam>,
    pub bursts: &'a mut Vec<explosions::Explosion>,
    pub vision: &'a mut std::collections::HashMap<u128, effects::Vision>,
    pub corpses: &'a mut Vec<Corpse>,
    pub kills: &'a mut Vec<(u128, Entity)>,
    pub budget: &'a mut Budget,
}
impl<W: World> Context<'_, W> {
    pub fn visible(&mut self, from: V3, to: V3) -> bool {
        // Bound both ray count and voxel work for long diagonal lines.
        let parts = (distance(from, to) / 2.).ceil().max(1.) as u32;
        if parts > 64 || parts > self.budget.rays {
            return false;
        }
        self.budget.rays -= parts;
        (0..parts).all(|i| {
            let d = sub(to, from);
            block_hit(
                self.world,
                add(from, scale(d, f64::from(i) / f64::from(parts))),
                add(from, scale(d, f64::from(i + 1) / f64::from(parts))),
                0.,
            )
            .is_none()
        })
    }

    pub fn enemies(&mut self, owner: u128, at: V3, range: f64) -> Vec<Actor> {
        let mut out = vec![];
        for a in self.actors {
            if self.budget.actors == 0 {
                break;
            }
            self.budget.actors -= 1;
            if a.eligible
                && a.health > 0.
                && distance(a.position, at) <= range
                && self.world.recon_target(owner, a.entity)
            {
                out.push(a.clone());
                if out.len() == 32 {
                    break;
                }
            }
        }
        out
    }
    pub fn object(
        &mut self,
        owner: u128,
        kind: ObjectKind,
        at: V3,
        health: f32,
        ticks: u32,
        velocity: V3,
    ) -> Option<u64> {
        if self.objects.len() >= MAX_OBJECTS
            || self.world.boxes(at.map(|n| n.floor() as i32)).is_none()
        {
            return None;
        }
        *self.next = self.next.saturating_add(1);
        let id = *self.next;
        self.objects.push(Object {
            id,
            owner,
            kind,
            position: at,
            health,
            ticks,
            velocity,
            target: None,
        });
        Some(id)
    }
    pub fn pulse(&mut self, owner: u128, at: V3, range: f64, los: bool, ticks: u32) {
        for a in self.enemies(owner, at, range) {
            if !los || self.visible(at, a.eye()) {
                self.intel.reveal(self.world, owner, a.entity, ticks);
            }
        }
    }
    pub fn hurt(&mut self, owner: u128, target: Entity, damage: f32) {
        let h = self.world.hurt(owner, target, damage);
        if h.killed && self.kills.len() < 128 {
            self.kills.push((owner, target));
        }
    }
    pub fn concuss(&mut self, target: Entity, ticks: u32) {
        self.world.effect(target, "minecraft:slowness", ticks, 4);
        self.world.effect(target, "minecraft:weakness", ticks, 1);
    }
    pub fn flash(&mut self, at: V3, range: f64, ticks: u32) {
        let targets: Vec<_> = self
            .actors
            .iter()
            .filter(|a| a.eligible && distance(a.eye(), at) <= range)
            .take(32)
            .cloned()
            .collect();
        for a in targets {
            if dot(a.direction, unit(sub(at, a.eye()))) > 0.15 && self.visible(at, a.eye()) {
                if let Entity::Player(id) = a.entity {
                    self.vision.entry(id).or_default().flash = ticks;
                }
                self.world.effect(a.entity, "minecraft:blindness", ticks, 0);
            }
        }
    }
}
pub struct Camera {
    pub remote: RemoteView,
    pub anchor: V3,
    pub source: u64,
}
impl Camera {
    pub fn new(w: &impl World, a: &Actor, at: V3, source: u64) -> Option<Self> {
        let mut remote = RemoteView::new(w, at, 32.)?;
        remote.look(a.direction);
        Some(Self {
            remote,
            anchor: a.position,
            source,
        })
    }
    pub fn update(&mut self, a: &Actor, c: &mut Context<'_, impl World>, fly: bool) -> bool {
        if !a.eligible
            || distance(a.position, self.anchor) > 4.
            || !c
                .objects
                .iter()
                .any(|o| o.id == self.source && o.health > 0. && o.ticks > 0)
            || block_hit(
                c.world,
                add(self.anchor, [0., 0.05, 0.]),
                add(self.anchor, [0., -0.3, 0.]),
                0.,
            )
            .is_none()
        {
            return false;
        }
        self.remote.look(a.direction);
        if fly {
            self.remote.step(
                c.world,
                scale(
                    add(
                        a.movement,
                        [
                            0.,
                            a.direction[1] * dot(horizontal(a.direction), a.movement)
                                + if a.jump { 0.5 } else { 0. },
                            0.,
                        ],
                    ),
                    0.4,
                ),
            );
        }
        let Entity::Player(id) = a.entity else {
            return false;
        };
        c.world.move_player(id, self.anchor);
        c.world.velocity(id, [0.; 3], false);
        true
    }
}
pub fn owner(a: &Actor) -> Option<u128> {
    if let Entity::Player(id) = a.entity {
        Some(id)
    } else {
        None
    }
}
pub fn landed(o: &mut Object, w: &impl World, gravity: f64) -> bool {
    if length(o.velocity) < 1e-6 {
        return true;
    }
    o.velocity[1] -= gravity;
    let end = add(o.position, o.velocity);
    if let Some(h) = block_collision(w, o.position, end, 0.08) {
        if !h.known {
            o.health = 0.;
        }
        o.position = add(o.position, scale(o.velocity, (h.time - 0.01).max(0.)));
        o.velocity = [0.; 3];
        true
    } else {
        o.position = end;
        false
    }
}
pub trait Agent {
    fn core(&self) -> &Core;
    fn core_mut(&mut self) -> &mut Core;
    fn activate(&mut self, slot: usize, a: &Actor, c: &mut Context<'_, impl World>) -> bool;
    fn fire(&mut self, alt: bool, a: &Actor, c: &mut Context<'_, impl World>) -> bool;
    fn tick(&mut self, a: &Actor, c: &mut Context<'_, impl World>);
    fn cancel(&mut self) {
        self.core_mut().cancel();
    }
    fn leave(&mut self) {
        self.cancel();
    }
    fn release(&mut self, _a: &Actor, _c: &mut Context<'_, impl World>) -> bool {
        self.core_mut().firing = false;
        true
    }
    fn recall(&mut self, _a: &Actor, _c: &mut Context<'_, impl World>) -> bool {
        false
    }
    fn protection(&self, a: &Actor) -> effects::Protection {
        effects::Protection {
            health: a.health,
            disarmed: self.camera().is_some(),
            ..Default::default()
        }
    }
    fn phased(&self) -> bool {
        false
    }
    fn camera(&self) -> Option<V3> {
        None
    }
    fn owns(&self, _id: u64) -> bool {
        false
    }
    fn stats(&self) -> [u32; 4] {
        [0; 4]
    }
    fn kill(&mut self) {}
    fn equipped(&self) -> bool {
        self.core().equipped.is_some() || self.camera().is_some()
    }
}
macro_rules! kits{($(($index:literal,$field:ident,$name:literal,$ty:path)),+ $(,)?)=>{
 pub const COUNT:usize=[$($index),+].len();
 pub fn names(i:usize)->Option<[&'static str;4]>{match i{$($index=>Some($field::NAMES),)+_=>None}}
 pub fn register(r:&mut crate::agents::Registry){$(r.register(crate::agents::Agent{id:stringify!($field),name:$name,slots:$field::DEFINITIONS.map(Some)}).expect("unique additional kit");)+}
 #[derive(Default)]pub struct Expansion{$(pub $field:$ty,)+}
 impl Expansion{
 pub fn slots(&self,i:usize)->[Runtime;4]{match i{$($index=>self.$field.core().slots,)+_=>[Runtime::default();4]}}
 pub fn busy(&self)->bool{$(self.$field.core().busy())||+}
 pub fn cancel(&mut self){$(self.$field.cancel();)+}
 pub fn leave(&mut self){$(self.$field.leave();)+}
 pub fn owns(&self,i:usize,id:u64)->bool{match i{$($index=>self.$field.owns(id),)+_=>false}}
 pub fn phased(&self,i:usize)->bool{match i{$($index=>self.$field.phased(),)+_=>false}}
 pub fn camera(&self,i:usize)->Option<V3>{match i{$($index=>self.$field.camera(),)+_=>None}}
 pub fn equipped(&self,i:usize)->bool{match i{$($index=>self.$field.equipped(),)+_=>false}}
 pub fn stats(&self,i:usize)->[u32;4]{match i{$($index=>self.$field.stats(),)+_=>[0;4]}}
 pub fn kill(&mut self,i:usize){match i{$($index=>self.$field.kill(),)+_=>{}}}
 pub fn activate(&mut self,i:usize,slot:usize,a:&Actor,c:&mut Context<'_,impl World>)->bool{match i{$($index=>self.$field.activate(slot,a,c),)+_=>false}}
 pub fn fire(&mut self,i:usize,alt:bool,a:&Actor,c:&mut Context<'_,impl World>)->bool{match i{$($index=>self.$field.fire(alt,a,c),)+_=>false}}
 pub fn release(&mut self,i:usize,a:&Actor,c:&mut Context<'_,impl World>)->bool{match i{$($index=>self.$field.release(a,c),)+_=>false}}
 pub fn recall(&mut self,i:usize,a:&Actor,c:&mut Context<'_,impl World>)->bool{match i{$($index=>self.$field.recall(a,c),)+_=>false}}
 pub fn tick(&mut self,i:usize,a:&Actor,c:&mut Context<'_,impl World>){match i{$($index=>{self.$field.core_mut().tick();if a.eligible{self.$field.tick(a,c);}else{self.$field.leave();}},)+_=>{}}if let Some(id)=owner(a){match i{$($index=>c.world.protect(id,self.$field.protection(a)),)+_=>{}}}}
 }
};}
kits!((12, cypher, "Cypher", cypher::Cypher));
