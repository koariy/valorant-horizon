pub mod abilities;
pub mod advanced;
pub mod agents;
pub mod gameplay;
pub mod server;
pub mod targeting;
use abilities::{Event, Runtime};
use agents::Registry;
use std::sync::{Mutex, OnceLock};

pub const ID: &str = "valorant_horizon";
pub const NAMES: [&str; 4] = ["c", "q", "e", "x"];
pub const DEFAULTS: [&str; 4] = [
    "key.keyboard.c",
    "key.keyboard.q",
    "key.keyboard.e",
    "key.keyboard.x",
];

/// Only accessed by the window hooks. Never used for server gameplay state.
pub fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(State::default()))
}
pub struct State {
    pub held: std::collections::HashSet<String>,
    pub registry: Registry,
    pub selected: usize,
    pub menu: bool,
    pub menu_page: usize,
    pub rebind: Option<usize>,
    pub loaded: bool,
    pub diagnostic: Option<String>,
    pub sequence: u64,
    pub pending_equipped: Option<u64>,
    pub heartbeat: u8,
    pub consumed: Option<String>,
    pub snapshot: Option<gameplay::Snapshot>,
    pub runtime: [Runtime; 4],
}
impl Default for State {
    fn default() -> Self {
        let mut state = Self {
            held: Default::default(),
            registry: Registry::default(),
            selected: 0,
            menu: false,
            menu_page: 0,
            rebind: None,
            loaded: false,
            diagnostic: None,
            sequence: 0,
            pending_equipped: None,
            heartbeat: 0,
            consumed: None,
            snapshot: None,
            runtime: [Runtime::default(); 4],
        };
        state.reset();
        state
    }
}
impl State {
    pub fn key_transition(&mut self, key: &str, pressed: bool) -> bool {
        if pressed {
            self.held.insert(key.into())
        } else {
            self.held.remove(key);
            false
        }
    }
    fn reset(&mut self) {
        self.runtime = self.registry.agents()[self.selected]
            .slots
            .map(|a| Runtime {
                charges: a.map_or(0, |a| a.charges),
                ..Runtime::default()
            });
    }
    pub fn restore(&mut self, id: &str) {
        if let Some(index) = self.registry.index(id) {
            self.selected = index;
        }
        self.reset();
        self.loaded = true;
    }
    pub fn select(&mut self, index: usize) -> bool {
        // Switching cannot refill a used ability or cancel an active effect.
        if index >= self.registry.agents().len()
            || self.runtime.iter().any(|r| r.remaining > 0 || r.active > 0)
        {
            return false;
        }
        self.selected = index;
        self.menu = false;
        self.reset();
        true
    }
    pub fn activate(&mut self, slot: usize) -> Option<Event> {
        if self.menu {
            return None;
        }
        let definition = *self.registry.agents()[self.selected].slots.get(slot)?;
        self.runtime[slot]
            .activate(definition?)
            .then_some(Event::Started(slot))
    }
    pub fn tick(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        for (slot, definition) in self.registry.agents()[self.selected]
            .slots
            .iter()
            .enumerate()
        {
            if let Some(definition) = definition
                && self.runtime[slot].tick(*definition)
            {
                events.push(Event::Ended(slot));
            }
        }
        events
    }
    pub fn lines(&self, bindings: &[String; 4]) -> Vec<String> {
        let mut lines = vec![format!(
            "Valorant Horizon | {} | [V] agents",
            self.registry.agents()[self.selected].name
        )];
        for (i, key) in bindings.iter().enumerate() {
            let r = self.runtime[i];
            lines.push(format!(
                "{}: {}  {}/{}  CD {:.1}s{}",
                key.trim_start_matches("key.keyboard."),
                if let Some(names) = advanced::names(self.selected) {
                    names[i]
                } else if self.selected == 0 {
                    agents::jett::NAMES[i]
                } else if self.selected == 1 {
                    agents::sage::NAMES[i]
                } else if self.selected == 2 {
                    agents::phoenix::NAMES[i]
                } else if self.selected == 3 {
                    agents::omen::NAMES[i]
                } else if self.selected == 4 {
                    agents::raze::NAMES[i]
                } else if self.selected == 5 {
                    agents::reyna::NAMES[i]
                } else if self.selected == 11 {
                    agents::sova::NAMES[i]
                } else if self.selected == 10 {
                    agents::clove::NAMES[i]
                } else if self.selected == 9 {
                    agents::harbor::NAMES[i]
                } else if self.selected == 8 {
                    agents::astra::NAMES[i]
                } else if self.selected == 7 {
                    agents::viper::NAMES[i]
                } else if self.selected == 6 {
                    agents::brimstone::NAMES[i]
                } else {
                    "unimplemented"
                },
                r.charges,
                self.registry.agents()[self.selected].slots[i].map_or(0, |a| a.charges),
                r.remaining as f32 / 20.0,
                if r.active > 0 { " [ACTIVE]" } else { "" }
            ));
        }
        if let Some(info) = &self.diagnostic {
            lines.push(info.clone());
        }
        if let Some(snapshot) = &self.snapshot {
            if snapshot.suppressed > 0 {
                lines.push(format!(
                    "SUPPRESSED {:.1}s",
                    snapshot.suppressed as f32 / 20.
                ));
            }
            if snapshot.stabilize {
                lines.push(format!(
                    "Hold R to stabilize ally | {} ticks",
                    snapshot.stabilizing
                ));
            }
            if self.selected == 18 {
                lines.push(format!(
                    "NULL/CMD: {} | suppressed targets: {} | downed: {} | pool: {}",
                    snapshot.kit[0], snapshot.kit[1], snapshot.kit[2], snapshot.kit[3]
                ));
            }
            if self.selected == 17 {
                lines.push(format!(
                    "Dash legs: {} | Refract: {} | returning: {} | boost: {}",
                    snapshot.kit[0], snapshot.kit[1], snapshot.kit[2], snapshot.kit[3]
                ));
            }
            if self.selected == 16 {
                lines.push(format!(
                    "Focus: {} | flow: {} | shield: {} | orbs: {}",
                    snapshot.kit[0], snapshot.kit[1], snapshot.kit[2], snapshot.kit[3]
                ));
            }
            if self.selected == 15 {
                lines.push(format!(
                    "Echoes: {} | Gate: {} | Teleport: {} | Drift: {:.1}s",
                    snapshot.kit[0],
                    snapshot.kit[1],
                    snapshot.kit[2],
                    snapshot.kit[3] as f32 / 20.
                ));
            }
            if self.selected == 14 {
                lines.push(format!(
                    "Energy: {}% | slide: {} | Overdrive: {:.1}s",
                    snapshot.kit[0],
                    snapshot.kit[1],
                    snapshot.kit[2] as f32 / 20.
                ));
            }
            if self.selected == 13 {
                lines.push(format!(
                    "Prowlers: {} | Seize: {} | Haunts: {} | Nightfall: {}",
                    snapshot.kit[0], snapshot.kit[1], snapshot.kit[2], snapshot.kit[3]
                ));
            }
            if snapshot.vision.deafen > 0 {
                lines.push(format!(
                    "Deafened: {:.1}s",
                    snapshot.vision.deafen as f32 / 20.
                ));
            }
            if self.selected == 12 {
                lines.push(format!(
                    "Tripwires: {} | cage devices: {} | camera: {}",
                    snapshot.kit[0],
                    snapshot.kit[1],
                    if snapshot.kit[3] > 0 {
                        "viewing"
                    } else if snapshot.kit[2] > 0 {
                        "placed"
                    } else {
                        "none"
                    }
                ));
            }
            if self.selected == 11 {
                lines.push(format!(
                    "Bow: {:?}/30 | bounces: {} | Fury: {} | revealed: {}",
                    snapshot.bow_charge,
                    snapshot.bounces,
                    snapshot.fury_shots,
                    snapshot.contacts.len()
                ));
            }
            if self.selected == 10 && snapshot.after_death {
                lines.push(
                    "Ruse after death: E equip | arrows aim | Z mark | R confirm | B cancel".into(),
                );
            }
            if self.selected == 8 {
                lines.push(format!(
                    "Stars: {} | [R] Dissipate | Astral RMB: Divide, LMB: points",
                    snapshot.star_count
                ));
            }
            if let Some(fuel) = snapshot.fuel {
                lines.push(format!("Toxin: {:.0}%", fuel));
            }
            if self.selected == 0 {
                lines.push(format!(
                    "Blade Storm: {} knives | [B] cancel",
                    snapshot.knives
                ));
            } else {
                lines.push("[LMB/RMB] cast equipped ability | [B] cancel".into());
            }
        } else {
            lines.push("Waiting for modded server".into());
        }
        if self.menu {
            for (index, agent) in self
                .registry
                .agents()
                .iter()
                .enumerate()
                .skip(self.menu_page * 10)
                .take(10)
            {
                lines.push(format!(
                    "[{}] {}",
                    if self.menu_page > 0 {
                        ((index % 10 + 1) % 10).to_string()
                    } else if index == 11 {
                        "=".into()
                    } else if index == 10 {
                        "-".into()
                    } else if index == 9 {
                        "0".into()
                    } else {
                        (index + 1).to_string()
                    },
                    agent.name
                ));
            }
            lines.push(format!(
                "[Left/Right] page {} | [C/Q/E/X] rebind | [V] close",
                self.menu_page + 1
            ));
        }

        if self.rebind.is_some() {
            lines.push("Press a new key (Esc cancels; unique keys only)".into());
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cooldown_prevents_agent_switch_and_cancel_does_not_reset_it() {
        let mut s = State::default();
        assert_eq!(s.activate(0), Some(Event::Started(0)));
        assert!(!s.select(1));
        for _ in 0..400 {
            s.tick();
        }
        assert!(s.select(1));
    }
    #[test]
    fn invalid_settings_and_menu_do_not_activate() {
        let mut s = State::default();
        s.restore("unknown");
        assert_eq!(s.selected, 0);
        assert!(!s.select(100));
        s.menu = true;
        assert_eq!(s.activate(2), None);
        assert_eq!(s.activate(99), None);
    }
    #[test]
    fn duplicate_agents_are_rejected() {
        let mut r = Registry::default();
        assert!(
            r.register(agents::Agent {
                id: "jett",
                name: "duplicate",
                slots: [None; 4]
            })
            .is_err()
        );
    }
    #[test]
    fn target_range_rejects_invalid_coordinates() {
        assert!(targeting::in_range([0.0; 3], [3.0, 4.0, 0.0], 5.0));
        assert!(!targeting::in_range([0.0; 3], [3.0, 4.0, 0.0], 4.9));
        assert!(!targeting::in_range([f64::NAN; 3], [0.0; 3], 5.0));
        assert!(!targeting::in_range([0.0; 3], [0.0; 3], -1.0));
    }
}

#[cfg(test)]
mod input_tests {
    #[test]
    fn repeated_keydown_does_not_reactivate_until_release() {
        let mut s = super::State::default();
        assert!(s.key_transition("key.keyboard.x", true));
        assert!(!s.key_transition("key.keyboard.x", true));
        assert!(!s.key_transition("key.keyboard.x", false));
        assert!(s.key_transition("key.keyboard.x", true));
    }
}

#[cfg(test)]
mod astral_input_tests {
    #[test]
    fn astral_mouse_buttons_latch_independently_until_release() {
        let mut s = super::State::default();
        assert!(s.key_transition("astra.attack", true));
        assert!(!s.key_transition("astra.attack", true));
        assert!(s.key_transition("astra.use", true));
        s.key_transition("astra.attack", false);
        assert!(s.key_transition("astra.attack", true));
        assert!(!s.key_transition("astra.use", true));
    }
}
