use super::{geometry::*, *};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ObjectKind {
    SeismicFront,
    WallCharge,
    Fragment,
    FlashDrive,
    SuppressionBlade,
    KayoCore,
    LightCluster,
    LightBeacon,
    LightPath,
    IsoOrb,
    IsoAura,
    Undercut,
    EnergyWall,
    Echo,
    DormantEcho,
    Gatecrash,
    Blindside,
    RelayBolt,
    HauntOrb,
    SeizeOrb,
    Haunt,
    Prowler,
    Trapwire,
    Spycam,
    CageDevice,
    Drone,
    ReconBolt,
    Shade,
    Leer,
    Soul,
    BoomBot,
    BlastPack,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Object {
    #[serde(skip)]
    pub last_hit: Option<UtilityDamage>,
    pub id: u64,
    #[serde(rename = "o")]
    pub owner: u128,
    #[serde(rename = "k", alias = "kind")]
    pub kind: ObjectKind,
    #[serde(rename = "p", alias = "position")]
    pub position: V3,
    #[serde(rename = "h", alias = "health")]
    pub health: f32,
    #[serde(rename = "t", alias = "ticks")]
    pub ticks: u32,
    #[serde(rename = "v", alias = "velocity")]
    pub velocity: V3,
    #[serde(rename = "g", alias = "target")]
    pub target: Option<Entity>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UtilityCause {
    Other,
    Shot,
    Melee,
    Explosion,
}
#[derive(Clone, Debug)]
pub struct UtilityDamage {
    pub cause: UtilityCause,
    pub attacker: Option<Entity>,
    pub from: V3,
}
impl ObjectKind {
    pub fn damageable(self) -> bool {
        !matches!(
            self,
            Self::SeismicFront
                | Self::WallCharge
                | Self::Fragment
                | Self::FlashDrive
                | Self::KayoCore
                | Self::LightCluster
                | Self::LightBeacon
                | Self::LightPath
                | Self::RelayBolt
                | Self::Blindside
                | Self::DormantEcho
                | Self::IsoAura
                | Self::Undercut
                | Self::EnergyWall
        )
    }
    pub fn is_advanced(self) -> bool {
        matches!(
            self,
            Self::SeismicFront
                | Self::WallCharge
                | Self::Fragment
                | Self::FlashDrive
                | Self::SuppressionBlade
                | Self::KayoCore
                | Self::LightCluster
                | Self::LightBeacon
                | Self::LightPath
                | Self::Trapwire
                | Self::Spycam
                | Self::CageDevice
                | Self::HauntOrb
                | Self::SeizeOrb
                | Self::Haunt
                | Self::Prowler
                | Self::RelayBolt
                | Self::Echo
                | Self::DormantEcho
                | Self::Gatecrash
                | Self::Blindside
                | Self::IsoOrb
                | Self::IsoAura
                | Self::Undercut
                | Self::EnergyWall
        )
    }
}
impl Object {
    pub fn pending_damage_event(&self) -> bool {
        matches!(self.kind, ObjectKind::Echo | ObjectKind::IsoOrb)
    }
    pub fn damage(&mut self, amount: f32, attacker: Option<Entity>, from: V3) {
        self.damage_with(amount, attacker, from, UtilityCause::Other);
    }
    pub fn damage_with(
        &mut self,
        amount: f32,
        attacker: Option<Entity>,
        from: V3,
        cause: UtilityCause,
    ) {
        if self.health <= 0. || amount <= 0. || !self.kind.damageable() {
            return;
        }
        if matches!(self.kind, ObjectKind::IsoOrb)
            && (cause != UtilityCause::Shot || attacker != Some(Entity::Player(self.owner)))
        {
            return;
        }
        self.last_hit = Some(UtilityDamage {
            attacker,
            from,
            cause,
        });
        self.health = (self.health - amount).max(0.);
    }
    pub fn hit_by(&self, from: V3, to: V3, padding: f64, attacker: Option<Entity>) -> Option<f64> {
        if matches!(self.kind, ObjectKind::IsoOrb) && attacker != Some(Entity::Player(self.owner)) {
            return None;
        }
        self.hit(from, to, padding)
    }
    pub fn hit(&self, from: V3, to: V3, padding: f64) -> Option<f64> {
        if self.health <= 0.
            || self.ticks == 0
            || (!self.kind.damageable() && !matches!(self.kind, ObjectKind::EnergyWall))
        {
            return None;
        }
        if matches!(self.kind, ObjectKind::EnergyWall) {
            let d = sub(to, from);
            let normal = horizontal(self.velocity);
            let denominator = dot(d, normal);
            let relative = sub(from, self.position);
            let side = [-normal[2], 0., normal[0]];
            if dot(relative, normal).abs() <= padding
                && relative[1] >= -padding
                && relative[1] <= 3. + padding
                && dot(relative, side).abs() <= 2. + padding
            {
                return Some(0.);
            }
            if denominator.abs() < 1e-9 {
                return None;
            }
            let t = dot(sub(self.position, from), normal) / denominator;
            let at = add(from, scale(d, t));
            let rel = sub(at, self.position);
            let side = [-normal[2], 0., normal[0]];
            return ((0. ..=1.).contains(&t)
                && rel[1] >= -padding
                && rel[1] <= 3. + padding
                && dot(rel, side).abs() <= 2. + padding)
                .then_some(t);
        }
        if !matches!(self.kind, ObjectKind::Trapwire) {
            return self.bounds().expanded(padding).segment(from, to);
        }
        let d = sub(self.velocity, self.position);
        let n = dot(d, d);
        if n < 1e-9 {
            return None;
        }
        let steps = (distance(from, to) / 0.04).ceil().clamp(1., 256.) as u32;
        (0..=steps).find_map(|i| {
            let t = f64::from(i) / f64::from(steps);
            let p = add(from, scale(sub(to, from), t));
            let q = add(
                self.position,
                scale(d, (dot(sub(p, self.position), d) / n).clamp(0., 1.)),
            );
            (distance(p, q) < 0.05 + padding).then_some(t)
        })
    }

    pub fn bounds(&self) -> Box3 {
        if matches!(self.kind, ObjectKind::Trapwire) {
            return Box3 {
                min: std::array::from_fn(|i| self.position[i].min(self.velocity[i]) - 0.05),
                max: std::array::from_fn(|i| self.position[i].max(self.velocity[i]) + 0.05),
            };
        }
        if matches!(self.kind, ObjectKind::EnergyWall) {
            let n = horizontal(self.velocity);
            let half = [n[2].abs() * 2. + 0.05, 0., n[0].abs() * 2. + 0.05];
            return Box3 {
                min: sub(self.position, half),
                max: add(add(self.position, half), [0., 3., 0.]),
            };
        }
        let (width, height) = match self.kind {
            ObjectKind::SeismicFront
            | ObjectKind::WallCharge
            | ObjectKind::Fragment
            | ObjectKind::FlashDrive
            | ObjectKind::SuppressionBlade
            | ObjectKind::KayoCore
            | ObjectKind::LightCluster
            | ObjectKind::LightBeacon
            | ObjectKind::LightPath
            | ObjectKind::Trapwire
            | ObjectKind::Spycam
            | ObjectKind::CageDevice => (0.12, 0.25),
            ObjectKind::HauntOrb
            | ObjectKind::SeizeOrb
            | ObjectKind::Haunt
            | ObjectKind::Prowler => (0.25, 0.5),
            ObjectKind::IsoOrb
            | ObjectKind::IsoAura
            | ObjectKind::Undercut
            | ObjectKind::EnergyWall => (0.2, 0.4),
            ObjectKind::Echo => (0.3, 1.8),
            ObjectKind::DormantEcho | ObjectKind::Gatecrash => (0.15, 0.2),
            ObjectKind::Blindside => (0.1, 0.2),
            ObjectKind::RelayBolt => (0.1, 0.2),
            ObjectKind::Drone => (0.15, 0.3),
            ObjectKind::ReconBolt => (0.12, 0.25),
            ObjectKind::Soul => (0.15, 0.3),
            ObjectKind::Leer => (0.35, 0.7),
            ObjectKind::Shade => (0.35, 1.8),
            ObjectKind::BoomBot => (0.3, 0.45),
            ObjectKind::BlastPack => (0.2, 0.12),
        };
        Box3 {
            min: add(self.position, [-width, 0.0, -width]),
            max: add(self.position, [width, height, width]),
        }
    }
}
pub fn strike(objects: &mut Vec<Object>, a: &Actor, w: &impl World) -> bool {
    let from = a.eye();
    let to = add(from, scale(a.direction, 3.0));
    let wall = block_hit(w, from, to, 0.0).unwrap_or(1.0);
    let wall = w
        .actors()
        .iter()
        .filter(|t| t.entity != a.entity && t.eligible)
        .filter_map(|t| t.bounds.segment(from, to))
        .fold(wall, f64::min);
    let hit = objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.kind.damageable() && !matches!(o.kind, ObjectKind::IsoOrb))
        .filter_map(|(i, o)| o.hit(from, to, 0.).filter(|t| *t < wall).map(|t| (t, i)))
        .min_by(|(a, _), (b, _)| a.total_cmp(b))
        .map(|(_, i)| i);
    if let Some(i) = hit {
        objects[i].damage_with(4., Some(a.entity), from, UtilityCause::Melee);
        objects.retain(|o| o.health > 0.0 || o.pending_damage_event());
        true
    } else {
        false
    }
}
