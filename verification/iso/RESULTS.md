# Iso — 2026-11-08, 0.4.0-dev.5

PASS: formatter, Clippy all-targets -D warnings, 219 Rust tests (18 Iso), engine release build, helper/bridge rustfmt, 14 optional delegate CLI tests. PASS: horizon-mixins apply, cargo check --locked -p horizon-client --lib --no-default-features and full cargo build --locked -p horizon-client --bin horizon-client --no-default-features. Logs and native-binary.json record this exact source milestone. Source/archive/staging byte comparisons and official native bundle validator pass.

Iso tests cover focus/flow/cancellation, confirmed kills/assists and private orb routing, owner-shot-only orbs, melee/explosion/enemy refusal, native-style utility shots, finite assist/orb TTL, one-hit grants/epochs/duplicate synchronization/expiry, actual incoming-damage helper and void, rejected/canceled/disarmed hits, Undercut through known terrain and ally vulnerability, unknown chunks/phase exclusion, finite moving indestructible Contingency vs terrain/hitscan/melee, bounded resources and death cleanup. The initial wall interception fixture crossed another player's body before the wall; its firing line was corrected to test the wall rather than actor priority.

Native post-damage attribution uses the SDK's scoped health::knockback_attacker only when causing is absent; the existing router validates actual positive HP change. RMB is no longer intercepted as a utility melee attack, permitting native bow use. Protection merges preserve a consumed shield for the same grant epoch.

Kill Contract is explicitly unavailable (zero charges, specific diagnostic): safe isolated arena, transactional participant return and death/drop/disconnect integration are not implemented. See docs/ISO.md for a concrete integration path. No ordinary-world fake arena is advertised as the ultimate.

Horizon client0.1.0/Minecraft26.3 commit41f822b659789780f58d32c63ddfb6887ea1b8bf; mixins0.6.0 commit5a64af938ec86b00d53d16a408aabc0038fb3e57; Rust1.99.1/Linux x86_64. No Minecraft world, LAN, rendering, sound or input-feel acceptance was run. Code tests and compilation do not establish those results. Next: Waylay.
