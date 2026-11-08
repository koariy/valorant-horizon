# Cypher — 2026-11-08, 0.4.0-dev.1

PASS: cargo fmt --check, Clippy --all-targets -D warnings, 137 Rust tests (15 Cypher), release engine build, rustfmt helpers/bridge. PASS: 14 Python tests optional DeepSeek CLI, without external API requests. Logs are retained beside this report.

Cypher tests cover two-wall placement/charges, wrong-edge collision normal regression, crossing/tether/reveal/concuss, destroyed devices, capsule hits, camera/body/protection/destruction, remote dart origin/cooldown, real cage activation/slow without damage, actual death requirement/through-wall theft, cancellation, proximity retrieval refusal, server registration and corpse cleanup, 10000-actor budget, unknown chunks and remote/body action conflict. Visibility fails closed when its bounded voxel budget is exhausted. Maximum-payload test includes additional kit fields and remains below 30000 bytes.

PASS: horizon-mixins apply; cargo check --locked -p horizon-client --lib --no-default-features; full cargo build --locked -p horizon-client --bin horizon-client --no-default-features. Native binary digest/size in native-binary.json. Horizon client 0.1.0/Minecraft 26.3 commit 41f822b659789780f58d32c63ddfb6887ea1b8bf; mixins 0.6.0 commit 5a64af938ec86b00d53d16a408aabc0038fb3e57; Rust 1.99.1/Linux x86_64.

PASS: official package validator (package-check.json), archive/source/staging integrity (source-integrity.json). The .hnmod contains source for launcher recompilation, not the executable. Server and clients must use matching 0.4 DTOs.

The geometry failure was fixed in common collision normals: entry-plane selection replaces nearest-face selection, which incorrectly selected a parallel face at voxel edges. The World fixture now includes explicitly placed solid blocks in its collision query. Existing Sova bounce, Harbor, Viper and other-agent regression tests pass.

Not performed: actual Minecraft launch, Survival/Creative/LAN, graphical camera/fog/outline or sound playback tests. No prepared game instance was used; compilation is not gameplay validation. Native audio/controllers default features were disabled. Adaptations and controls: docs/CYPHER.md. No claim of exact Valorant balance or full runtime verification.

Fade implementation follows this passing milestone. Sage Resurrection and Clove Not Dead Yet remain explicitly disabled.
