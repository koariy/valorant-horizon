pub mod astra;
pub mod brimstone;
pub mod clove;
pub mod harbor;
pub mod jett;
pub mod omen;
pub mod phoenix;
pub mod raze;
pub mod reyna;
pub mod sage;
pub mod viper;
use crate::abilities::Ability;

pub struct Agent {
    pub id: &'static str,
    pub name: &'static str,
    pub slots: [Option<Ability>; 4],
}

pub struct Registry(Vec<Agent>);
impl Registry {
    pub fn register(&mut self, agent: Agent) -> Result<(), &'static str> {
        if agent.id.is_empty() || self.0.iter().any(|a| a.id == agent.id) {
            return Err("invalid or duplicate agent id");
        }
        self.0.push(agent);
        Ok(())
    }
    pub fn agents(&self) -> &[Agent] {
        &self.0
    }
    pub fn index(&self, id: &str) -> Option<usize> {
        self.0.iter().position(|a| a.id == id)
    }
}
impl Default for Registry {
    fn default() -> Self {
        let mut registry = Self(Vec::new());
        for (id, name) in [
            ("jett", "Jett"),
            ("sage", "Sage"),
            ("phoenix", "Phoenix"),
            ("omen", "Omen"),
            ("raze", "Raze"),
            ("reyna", "Reyna"),
            ("brimstone", "Brimstone"),
            ("viper", "Viper"),
            ("astra", "Astra"),
            ("harbor", "Harbor"),
            ("clove", "Clove"),
        ] {
            registry
                .register(Agent {
                    id,
                    name,
                    slots: if id == "jett" {
                        jett::DEFINITIONS.map(Some)
                    } else if id == "phoenix" {
                        phoenix::DEFINITIONS.map(Some)
                    } else if id == "sage" {
                        sage::DEFINITIONS.map(Some)
                    } else if id == "omen" {
                        omen::DEFINITIONS.map(Some)
                    } else if id == "raze" {
                        raze::DEFINITIONS.map(Some)
                    } else if id == "reyna" {
                        reyna::DEFINITIONS.map(Some)
                    } else if id == "brimstone" {
                        brimstone::DEFINITIONS.map(Some)
                    } else if id == "viper" {
                        viper::DEFINITIONS.map(Some)
                    } else if id == "astra" {
                        astra::DEFINITIONS.map(Some)
                    } else if id == "clove" {
                        clove::DEFINITIONS.map(Some)
                    } else if id == "harbor" {
                        harbor::DEFINITIONS.map(Some)
                    } else {
                        [None; 4]
                    },
                })
                .expect("unique builtin agent");
        }
        registry
    }
}
