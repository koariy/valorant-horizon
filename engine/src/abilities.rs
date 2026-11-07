/// Shared slot classification; persistent passives are evaluated by their agent kit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Active,
    Passive,
    Ultimate,
}
#[derive(Clone, Copy)]
pub struct Ability {
    pub kind: Kind,
    pub cooldown: u32,
    pub duration: u32,
    pub charges: u8,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Runtime {
    pub remaining: u32,
    pub active: u32,
    pub charges: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Started(usize),
    Ended(usize),
}
impl Runtime {
    pub fn activate(&mut self, definition: Ability) -> bool {
        if definition.kind == Kind::Passive || self.charges == 0 {
            return false;
        }
        self.charges -= 1;
        self.active = definition.duration;
        if self.remaining == 0 {
            self.remaining = definition.cooldown.max(1);
        }
        true
    }
    pub fn tick(&mut self, definition: Ability) -> bool {
        let ended = self.active == 1;
        self.active = self.active.saturating_sub(1);
        if self.remaining > 0 {
            self.remaining -= 1;
            if self.remaining == 0 {
                self.charges = (self.charges + 1).min(definition.charges);
                if self.charges < definition.charges {
                    self.remaining = definition.cooldown.max(1);
                }
            }
        }
        ended
    }
}
