# Breach — 2026-11-08, 0.4.0-dev.8

PASS: fmt, Clippy all-targets -D warnings, 279 Rust tests (15 Breach), release engine and helper/bridge/native-state rustfmt. PASS: mixins apply, native cargo check --locked --lib --no-default-features and full cargo build --locked --bin horizon-client --no-default-features. Logs, binary hash, archive/source/stage integrity and package validator included.

Tests cover two delayed explosion bursts behind real terrain, flash origin behind the wall vs caster/facing/allies/phase, air/thick/unknown/budget rejection with no spending, newly blocked exit and secondary terrain, charged range and required release/quick cast, through-known-terrain waves vs height/width/unknown chunks/allies/protected/phase, real player and mob velocity, one concuss per target, resource refusal, cancel/death cleanup, KAY/O suppression interrupt vs already-fired quake, ceiling placement, registry/HUD. Actual World.push_entity mob implementation remains native_entity_set_velocity; mock now models both player and mob velocity.

Official Riot descriptions rechecked; patch7.04 corroborates two Aftershock bursts. Minecraft damage/range/time are explicitly adapted, not claimed exact live balance values. Concuss shares real movement/attack-speed/weakness statuses, so prior Neon/other kit regressions are included. Flash/nearsight rendering affects players; vanilla mob AI perception is not changed by the blindness status. Native stun/knock-up and damage still affect mobs. See docs/BREACH.md.

Horizon client0.1.0/Minecraft26.3 commit41f822b659789780f58d32c63ddfb6887ea1b8bf; mixins0.6.0 commit5a64af938ec86b00d53d16a408aabc0038fb3e57; Rust1.99.1/Linux x86_64. No Minecraft world/LAN/render/audio acceptance. Next: Skye.
