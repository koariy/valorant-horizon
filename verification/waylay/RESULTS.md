# Waylay — 2026-11-08, 0.4.0-dev.6

PASS: fmt, Clippy all-targets -D warnings, 239 Rust tests (20 Waylay), engine release build and all helper/bridge rustfmt checks. PASS: horizon-mixins apply, native cargo check --locked --lib --no-default-features and full cargo build --locked --bin horizon-client --no-default-features. Native binary hash, archive/source/stage integrity, package validator and logs are included. Optional delegate CLI tests: 14 passed.

Tests cover real Saturate ground collision, wall bounce, TTL/unknown chunks, Hinder vs ally/protected/phase/LOS, pool refusal without spending; one/two Lightspeed legs, upward first leg and updated horizontal second leg, body sweep vs actor/terrain/unknown chunks/budget; real-ground beacon, protected body return and dash-path recording, new terrain, semantic teleport/expiry/cancel/cleanup; delayed expanding Convergent Paths, real speed, terrain/phase rules, resource refusal and registration/HUD.

The stricter maximal-snapshot fixture uses a longer object kind and unavailable-ultimate error. It exposed a 2-byte excess over the reserved 29000-byte limit; compact serde aliases for repeated object/projectile/zone/toxin fields restored margin without increasing the 30000-byte sender cap. Decode round-trip and all previous kits pass. Human-readable Rust field names remain intact.

Refract follows a bounded recorded path and stops at new obstacles, actors or unknown chunks rather than clipping through Minecraft construction. Hinder maps to native movement and attack-speed/mining modifiers; it does not invent weapon reload logic. Friendly targets are excluded by the mod's common hostile-target policy, a documented adaptation. See docs/WAYLAY.md for controls, settings and limits.

Horizon client0.1.0/Minecraft26.3 commit41f822b659789780f58d32c63ddfb6887ea1b8bf; mixins0.6.0 commit5a64af938ec86b00d53d16a408aabc0038fb3e57; Rust1.99.1/Linux x86_64. No Minecraft world, LAN, rendering, audio or input-feel acceptance was run. Next: KAY/O.
