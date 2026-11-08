use super::{Entity, World, geometry::*};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ProjectileKind {
    Incendiary { duration: u32 },
    PoisonEmitter,
    SnakeBite,
    SovaBolt { recon: bool, bounces: u8 },
    ReconDart,
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
    #[serde(rename = "o")]
    pub owner: u128,
    #[serde(rename = "k", alias = "kind")]
    pub kind: ProjectileKind,
    #[serde(rename = "p", alias = "position")]
    pub position: V3,
    #[serde(rename = "v", alias = "velocity")]
    pub velocity: V3,
    #[serde(rename = "t", alias = "ticks")]
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
            ProjectileKind::Knife { .. } | ProjectileKind::Rocket | ProjectileKind::ReconDart
        ) {
            for a in world.actors().into_iter().filter(|a| {
                a.eligible
                    && a.health > 0.0
                    && a.entity != Entity::Player(self.owner)
                    && !world.status_immune(a.entity)
            }) {
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

/// Common ordering for real arrows and temporary knives against utility and barriers.
pub struct Shot {
    pub attacker: Option<Entity>,
    pub from: V3,
    pub to: V3,
    pub damage: f32,
    pub padding: super::shields::CollisionPadding,
}
pub fn intercept(
    w: &impl World,
    shields: &mut [super::shields::Shield],
    walls: &[super::divide::Divide],
    objects: &mut [super::objects::Object],
    shot: Shot,
) -> bool {
    let limit = super::shields::impact_limit(w, shot.from, shot.to, shot.padding);
    if let Some(hit) = utility_hit(
        shields,
        walls,
        objects,
        shot.from,
        shot.to,
        limit,
        shot.attacker,
    ) {
        apply_utility(
            hit.hit,
            shields,
            objects,
            shot.damage,
            shot.attacker,
            shot.from,
        );
        true
    } else {
        false
    }
}
#[derive(Clone, Copy)]
pub enum UtilityHit {
    Object(usize),
    Shield(usize),
    Divide,
}
pub struct UtilityCollision {
    pub time: f64,
    pub hit: UtilityHit,
}
/// Shared nearest-hit policy for native projectiles and budgeted ability rays.
/// The caller supplies the first terrain/actor boundary to avoid duplicate traces.
pub fn utility_hit(
    shields: &[super::shields::Shield],
    walls: &[super::divide::Divide],
    objects: &[super::objects::Object],
    from: V3,
    to: V3,
    limit: f64,
    attacker: Option<Entity>,
) -> Option<UtilityCollision> {
    let mut nearest = None;
    let mut boundary = limit;
    for d in walls.iter().filter(|d| d.ticks > 0) {
        if let Some(t) = d.crossing(from, to)
            && t <= boundary
        {
            boundary = t;
            nearest = Some(UtilityCollision {
                time: t,
                hit: UtilityHit::Divide,
            });
        }
    }
    for (i, s) in shields.iter().enumerate() {
        if let Some(t) = s.crossing(from, to)
            && t <= boundary
        {
            boundary = t;
            nearest = Some(UtilityCollision {
                time: t,
                hit: UtilityHit::Shield(i),
            });
        }
    }
    for (i, o) in objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.health > 0. && o.ticks > 0)
    {
        if let Some(t) = o.hit_by(from, to, 0.08, attacker)
            && t < boundary
        {
            boundary = t;
            nearest = Some(UtilityCollision {
                time: t,
                hit: UtilityHit::Object(i),
            });
        }
    }
    nearest
}
pub fn apply_utility(
    hit: UtilityHit,
    shields: &mut [super::shields::Shield],
    objects: &mut [super::objects::Object],
    damage: f32,
    attacker: Option<Entity>,
    from: V3,
) {
    match hit {
        UtilityHit::Object(i) => {
            objects[i].damage_with(damage, attacker, from, super::objects::UtilityCause::Shot)
        }
        UtilityHit::Shield(i) => shields[i].damage(damage),
        UtilityHit::Divide => {}
    }
}

/// Double-sided beam surfaces; coordinates may already be camera-relative.
pub fn beam(from: V3, to: V3, width: f64) -> Vec<[[f32; 3]; 4]> {
    let d = unit(sub(to, from));
    let side = if d[0].abs() + d[2].abs() < 0.01 {
        [1., 0., 0.]
    } else {
        unit([d[2], 0., -d[0]])
    };
    let up = [
        d[1] * side[2] - d[2] * side[1],
        d[2] * side[0] - d[0] * side[2],
        d[0] * side[1] - d[1] * side[0],
    ];
    let mut out = vec![];
    for axis in [side, up] {
        let q = [
            add(from, scale(axis, width)),
            add(to, scale(axis, width)),
            add(to, scale(axis, -width)),
            add(from, scale(axis, -width)),
        ]
        .map(|v| v.map(|n| n as f32));
        out.push(q);
        out.push([q[3], q[2], q[1], q[0]]);
    }
    out
}
