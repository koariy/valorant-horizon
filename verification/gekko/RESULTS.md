# Gekko — 2026-11-08, 0.4.0-dev.10

PASS: fmt, Clippy all-targets -D warnings, 331 Rust tests (30 Gekko/compatibility), release engine and bridge/helper/native-state rustfmt. PASS: 14 Python CLI tests. PASS: mixins apply, native cargo check --locked --lib --no-default-features and full cargo build --locked --bin horizon-client --no-default-features.

PASS: bundle validator and byte equality of 100 packaged source files against repository, native build inputs and staged crates. SHA-256/size of archive and linked native binary recorded separately. Native binary was recorded before generated-cache cleanup. No Minecraft world, LAN, render, audio or input-feel acceptance was performed.

Tests exercise hostile/visible seeking, creature HP/expiry, Wingman concussion and explicit Spike rejection; Dizzy plasma contact ordering, parent exclusion, new terrain/shields/phase/allies, facing-independent blind and mob fallback; finite Mosh DOT/explosion/self-damage/terrain; vulnerable remote Thrash body, reactivation/lunge, detain and one reclaim per ultimate cycle; hold-R range/LOS/damage/movement/lease/release/cancel/foreign-object checks, recovery, expiry and bounded capacity retries. Compatibility covers detain vs KAY/O stabilization, same-frame phase/protection publication and suppression interruption, logout flag cleanup, pending-fatal persistence, hostile mob policy and all-agent registry/protocol limits.

Shared controlled companions are reused by Skye and Thrash. Native logout API57 cleanup removes cached damage routes and protection without holding the session lock during Horizon calls. Detain retains gravity and is not invulnerability. Server controls own HP, targets, collisions and recovery; clients receive snapshots and render geometry/fog/sounds.

Official current Gekko page rechecked 2026-11-08; historical patch7.04 corroborates pre-explosion Mosh damage. Ordinary-world charge regeneration, Minecraft HP/timings, local navigation and vanilla mob slow/weakness fallback are explicit adaptations. No Spike mode; no claim of altered vanilla AI perception. Details: ../../docs/GEKKO.md.

Pinned Horizon client0.1.0/Minecraft26.3 commit41f822b659789780f58d32c63ddfb6887ea1b8bf; mixins0.6.0 commit5a64af938ec86b00d53d16a408aabc0038fb3e57; Rust1.99.1/Linux x86_64, no default client features.
