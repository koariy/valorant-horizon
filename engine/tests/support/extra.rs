use super::*;
use valorant_horizon_engine::{
    advanced::{self, Agent},
    gameplay::{
        effects::Vision,
        explosions::Explosion,
        objects::Object,
        projectiles::Projectile,
        recon::{Beam, Intel},
        smoke::Smoke,
        zones::Zone,
    },
};
#[derive(Default)]
pub struct Scene {
    pub w: TestWorld,
    pub actors: Vec<Actor>,
    pub objects: Vec<Object>,
    pub next: u64,
    pub projectiles: Vec<Projectile>,
    pub zones: Vec<Zone>,
    pub smokes: Vec<Smoke>,
    pub intel: Intel,
    pub beams: Vec<Beam>,
    pub bursts: Vec<Explosion>,
    pub vision: std::collections::HashMap<u128, Vision>,
    pub corpses: Vec<advanced::Corpse>,
    pub kills: Vec<(u128, Entity)>,
    pub shields: Vec<shields::Shield>,
    pub divides: Vec<divide::Divide>,
    pub decay: std::collections::HashMap<Entity, toxin::Decay>,
    pub budget: advanced::Budget,
}
impl Scene {
    pub fn ctx(&mut self) -> advanced::Context<'_, TestWorld> {
        self.actors = self.w.actors();
        advanced::Context {
            world: &mut self.w,
            actors: &self.actors,
            objects: &mut self.objects,
            next: &mut self.next,
            projectiles: &mut self.projectiles,
            zones: &mut self.zones,
            smokes: &mut self.smokes,
            intel: &mut self.intel,
            beams: &mut self.beams,
            bursts: &mut self.bursts,
            vision: &mut self.vision,
            corpses: &mut self.corpses,
            kills: &mut self.kills,
            shields: &mut self.shields,
            divides: &self.divides,
            decay: Some(&mut self.decay),
            budget: &mut self.budget,
        }
    }
    pub fn step(&mut self, k: &mut impl Agent, a: &Actor) {
        self.budget = Default::default();
        k.core_mut().tick();
        k.tick(a, &mut self.ctx());
    }
}
