use crate::{
    abilities::{Ability, Kind, Runtime},
    gameplay::{geometry::*, objects::*, projectiles::*, *},
};
pub const NAMES: [&str; 4] = [
    "Shrouded Step",
    "Paranoia",
    "Dark Cover",
    "From the Shadows",
];
pub const DEFINITIONS: [Ability; 4] = [
    Ability {
        kind: Kind::Active,
        cooldown: 600,
        duration: 8,
        charges: 2,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 700,
        duration: 60,
        charges: 1,
    },
    Ability {
        kind: Kind::Active,
        cooldown: 600,
        duration: 260,
        charges: 2,
    },
    Ability {
        kind: Kind::Ultimate,
        cooldown: 2400,
        duration: 60,
        charges: 1,
    },
];
pub struct Omen {
    pub slots: [Runtime; 4],
    pub equipped: Option<usize>,
    pub range: f64,
    pub cast: Option<(V3, V3, u32, bool)>,
    pub shade: Option<u64>,
    pub preview: Option<V3>,
}
impl Default for Omen {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|i| Runtime {
                charges: DEFINITIONS[i].charges,
                ..Default::default()
            }),
            equipped: None,
            range: 16.0,
            cast: None,
            shade: None,
            preview: None,
        }
    }
}
impl Omen {
    pub fn busy(&self) -> bool {
        self.slots.iter().any(|s| s.remaining > 0) || self.cast.is_some()
    }
    pub fn cancel(&mut self) {
        self.slots[0].active = 0;
        self.slots[3].active = 0;
        self.equipped = None;
        self.cast = None;
        self.shade = None;
        self.preview = None;
    }
    pub fn activate(
        &mut self,
        slot: usize,
        a: &Actor,
        w: &impl World,
        objects: &mut Vec<Projectile>,
    ) -> bool {
        let Entity::Player(owner) = a.entity else {
            return false;
        };
        if slot >= 4 || !a.eligible || self.cast.is_some() {
            return false;
        }
        if slot == 2 && self.equipped == Some(2) {
            let target = add(a.eye(), scale(a.direction, self.range));
            if objects.len() >= MAX_PROJECTILES
                || w.boxes(target.map(|v| v.floor() as i32)).is_none()
                || !self.slots[2].activate(DEFINITIONS[2])
            {
                return false;
            }
            objects.push(Projectile {
                owner,
                kind: ProjectileKind::DarkCover { target },
                position: a.eye(),
                velocity: scale(a.direction, 2.0),
                ticks: 50,
                guided: false,
                victims: vec![],
            });
            self.equipped = None;
            self.preview = None;
            return true;
        }
        if self.slots[slot].charges == 0 {
            return false;
        }
        if slot == 1 {
            if objects.len() >= MAX_PROJECTILES || !self.slots[1].activate(DEFINITIONS[1]) {
                return false;
            }
            objects.push(Projectile {
                owner,
                kind: ProjectileKind::Paranoia,
                position: a.eye(),
                velocity: scale(a.direction, 1.2),
                ticks: 24,
                guided: false,
                victims: vec![],
            });
            return true;
        }
        self.equipped = Some(slot);
        true
    }
    pub fn fire(
        &mut self,
        alt: bool,
        a: &Actor,
        w: &impl World,
        objects: &mut Vec<Object>,
        next: &mut u64,
    ) -> bool {
        let Entity::Player(owner) = a.entity else {
            return false;
        };
        let Some(slot) = self.equipped else {
            return false;
        };
        if slot == 2 {
            self.range = (self.range + if alt { -0.75 } else { 0.75 }).clamp(2.0, 64.0);
            return true;
        }
        if ![0, 3].contains(&slot) || self.cast.is_some() {
            return false;
        }
        let Some(target) = teleport::target(w, a, if slot == 0 { 8.0 } else { 64.0 }) else {
            return false;
        };
        if objects.len() >= MAX_OBJECTS || !self.slots[slot].activate(DEFINITIONS[slot]) {
            return false;
        }
        self.equipped = None;
        self.cast = Some((
            a.position,
            target,
            if slot == 0 { 8 } else { 60 },
            slot == 3,
        ));
        if slot == 3 {
            *next = next.saturating_add(1);
            self.shade = Some(*next);
            objects.push(Object {
                id: *next,
                owner,
                kind: ObjectKind::Shade,
                position: target,
                health: 1.0,
                ticks: 65,
                velocity: [0.0; 3],
                target: None,
            });
        }
        true
    }
    pub fn tick(&mut self, a: &Actor, w: &mut impl World, objects: &[Object]) {
        let Entity::Player(owner) = a.entity else {
            return;
        };
        for (r, d) in self.slots.iter_mut().zip(DEFINITIONS) {
            r.tick(d);
        }
        if !a.eligible {
            self.cancel();
            return;
        }
        self.preview = match self.equipped {
            Some(2) => Some(add(a.eye(), scale(a.direction, self.range))),
            Some(0) => teleport::target(w, a, 8.0),
            Some(3) => teleport::target(w, a, 64.0),
            _ => None,
        };
        if let Some((origin, target, left, long)) = self.cast {
            if long
                && !objects
                    .iter()
                    .any(|o| Some(o.id) == self.shade && o.health > 0.0)
            {
                self.cancel();
            } else if left <= 1 {
                let _ = teleport::safe(w, a, target);
                self.cancel();
            } else if !teleport::safe(w, a, origin) {
                self.cancel();
            } else {
                self.cast = Some((origin, target, left - 1, long));
            }
        }
        let shade = self.cast.is_some_and(|c| c.3);
        if shade {
            w.effect(a.entity, "minecraft:invisibility", 4, 0);
        }
        w.protect(
            owner,
            effects::Protection {
                health: a.health,
                invulnerable: shade,
                disarmed: self.cast.is_some(),
                ..Default::default()
            },
        );
    }
}
