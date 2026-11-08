//! Private, short-lived intelligence. Never sets a global glowing flag.
use super::{geometry::*, *};
use serde::{Deserialize, Serialize};
pub const MAX_CONTACTS: usize = 8;
pub const MAX_TAGS: usize = 64;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Contact {
    pub entity: u32,
    #[serde(rename = "t", alias = "ticks")]
    pub ticks: u32,
}
struct Reveal {
    owner: u128,
    viewer: u128,
    target: Entity,
    contact: Contact,
}
pub struct Tag {
    pub owner: u128,
    pub target: Entity,
    pub ticks: u32,
}
#[derive(Default)]
pub struct Intel {
    contacts: Vec<Reveal>,
    pub tags: Vec<Tag>,
}
impl Intel {
    pub fn reveal(&mut self, w: &impl World, owner: u128, target: Entity, ticks: u32) {
        if !w.recon_target(owner, target) {
            return;
        }
        let Some(entity) = w.network_id(target) else {
            return;
        };
        for viewer in w.intel_viewers(owner).into_iter().take(64) {
            if let Some(c) = self
                .contacts
                .iter_mut()
                .find(|c| c.viewer == viewer && c.target == target)
            {
                c.owner = owner;
                c.contact = Contact {
                    entity,
                    ticks: c.contact.ticks.max(ticks),
                };
            } else if self.contacts.iter().filter(|c| c.viewer == viewer).count() < MAX_CONTACTS
                && self.contacts.len() < 64 * MAX_CONTACTS
            {
                self.contacts.push(Reveal {
                    owner,
                    viewer,
                    target,
                    contact: Contact { entity, ticks },
                });
            }
        }
    }
    pub fn tag(&mut self, owner: u128, target: Entity) -> bool {
        if let Some(t) = self
            .tags
            .iter_mut()
            .find(|t| t.owner == owner && t.target == target)
        {
            t.ticks = 60;
            true
        } else if self.tags.len() < MAX_TAGS {
            self.tags.push(Tag {
                owner,
                target,
                ticks: 60,
            });
            true
        } else {
            false
        }
    }
    pub fn scan(&mut self, w: &impl World, owner: u128, at: V3, range: f64) {
        for a in w.actors().into_iter().filter(|a| {
            a.eligible
                && a.health > 0.
                && a.entity != Entity::Player(owner)
                && w.recon_target(owner, a.entity)
                && distance(at, a.eye()) <= range
                && block_hit(w, at, a.eye(), 0.).is_none()
        }) {
            self.reveal(w, owner, a.entity, 8);
        }
    }
    pub fn tick(&mut self, w: &impl World) {
        let actors = w.actors();
        self.contacts.retain_mut(|c| {
            c.contact.ticks = c.contact.ticks.saturating_sub(1);
            c.contact.ticks > 0
                && w.recon_target(c.owner, c.target)
                && w.network_id(c.target) == Some(c.contact.entity)
                && actors
                    .iter()
                    .any(|a| a.entity == c.target && a.eligible && a.health > 0.)
                && w.intel_viewers(c.owner).contains(&c.viewer)
        });
        let mut pulses = vec![];
        for t in &mut self.tags {
            if t.ticks % 20 == 0
                && actors
                    .iter()
                    .any(|a| a.entity == t.target && a.eligible && a.health > 0.)
            {
                pulses.push((t.owner, t.target));
            }
            t.ticks = t.ticks.saturating_sub(1);
        }
        self.tags.retain(|t| {
            t.ticks > 0
                && actors
                    .iter()
                    .any(|a| a.entity == t.target && a.eligible && a.health > 0.)
        });
        for (owner, target) in pulses {
            self.reveal(w, owner, target, 8);
        }
    }
    pub fn snapshot(&self, viewer: u128) -> Vec<Contact> {
        self.contacts
            .iter()
            .filter(|c| c.viewer == viewer)
            .map(|c| c.contact.clone())
            .take(MAX_CONTACTS)
            .collect()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Beam {
    pub from: V3,
    pub to: V3,
    pub ticks: u32,
}
pub const MAX_BEAMS: usize = 4;
/// A wall-piercing path still stops before unknown chunks; never loads new terrain.
pub fn loaded_end(w: &impl World, from: V3, direction: V3, range: f64) -> V3 {
    let mut end = from;
    for n in 1..=(range.clamp(0., 64.) * 2.).floor() as u32 {
        let at = add(from, scale(unit(direction), f64::from(n) * 0.5));
        if w.boxes(at.map(|v| v.floor() as i32)).is_none() {
            break;
        }
        end = at;
    }
    end
}
