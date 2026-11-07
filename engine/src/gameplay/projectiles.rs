use super::{Entity, World, geometry::*};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ProjectileKind {
    Incendiary { duration: u32 },
    PoisonEmitter,
    SnakeBite,
    Meddle,
    StormSurge,
    Reckoning,
    HotHands,
    BlastPack,
    Grenade { cluster: bool },
    Rocket,
    DarkCover { target: V3 },
    Paranoia,
    SlowOrb,
    Curveball { right: bool },
    Cloudburst,
    Knife { alternate: bool },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Projectile {
    pub owner: u128,
    pub kind: ProjectileKind,
    pub position: V3,
    pub velocity: V3,
    pub ticks: u32,
    pub guided: bool,
    #[serde(skip)]
    pub victims: Vec<Entity>,
}
pub enum Impact {
    Block(V3),
    Entity(Entity, V3, bool),
}
impl Projectile {
    pub fn advance(&mut self, world: &impl World) -> Option<Impact> {
        let to = add(self.position, self.velocity);
        let block = block_hit(world, self.position, to, 0.08);
        let mut hit = block.map(|t| {
            (
                t,
                Impact::Block(add(self.position, scale(self.velocity, t))),
            )
        });
        if matches!(
            self.kind,
            ProjectileKind::Knife { .. } | ProjectileKind::Rocket
        ) {
            for a in world
                .actors()
                .into_iter()
                .filter(|a| a.eligible && a.health > 0.0 && a.entity != Entity::Player(self.owner))
            {
                if let Some(t) = a.bounds.expanded(0.08).segment(self.position, to)
                    && hit.as_ref().is_none_or(|(old, _)| t < *old)
                {
                    let point = add(self.position, scale(self.velocity, t));
                    let head = point[1] > a.bounds.max[1] - 0.4;
                    hit = Some((t, Impact::Entity(a.entity, point, head)));
                }
            }
        }
        self.position = to;
        self.ticks = self.ticks.saturating_sub(1);
        hit.map(|(_, i)| i)
    }
}

/// A narrow, double-sided kunai silhouette, independent of textures or weapons.
pub fn blade(center: V3, direction: V3) -> Vec<[[f32; 3]; 4]> {
    let dir = unit(direction);
    let side = if dir[0].abs() + dir[2].abs() > 0.01 {
        unit([dir[2], 0.0, -dir[0]])
    } else {
        [1.0, 0.0, 0.0]
    };
    let tip = add(center, scale(dir, 0.4));
    let tail = add(center, scale(dir, -0.25));
    let q = [
        tip,
        add(center, scale(side, 0.075)),
        tail,
        add(center, scale(side, -0.075)),
    ]
    .map(|p| p.map(|v| v as f32));
    vec![q, [q[3], q[2], q[1], q[0]]]
}
