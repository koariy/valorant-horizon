# Valorant Horizon 0.4.0-dev.10 — 2026-11-08

22 requested agents are registered. This expansion continued the existing 12-agent architecture; each milestone was tested and built before moving to the next agent. This is code/native verification, not a completed Minecraft playtest.

| Milestone | Version | Dedicated tests | Evidence |
|---|---|---:|---|
| Cypher | dev.1 | 16 in final suite (15 at first milestone) | [report](../cypher/RESULTS.md) |
| Fade | dev.2 | 28 | [report](../fade/RESULTS.md) |
| Neon | dev.3 | 16 | [report](../neon/RESULTS.md) |
| Yoru | dev.4 | 19 | [report](../yoru/RESULTS.md) |
| Iso | dev.5 | 18 | [report](../iso/RESULTS.md) |
| Waylay | dev.6 | 20 | [report](../waylay/RESULTS.md) |
| KAY/O | dev.7 | 25 | [report](../kayo/RESULTS.md) |
| Breach | dev.8 | 15 | [report](../breach/RESULTS.md) |
| Skye | dev.9 | 22 | [report](../skye/RESULTS.md) |
| Gekko | dev.10 | 30 | [report](../gekko/RESULTS.md) |

Final suite: **331 Rust tests, zero failures**, fmt, Clippy all-targets with warnings denied, release engine build and bridge/helper/native-state formatting. **14 Python CLI tests pass**. [Full final log](../gekko/verify.log).

Final native checks: mixins apply, locked lib check and full horizon-client build, Linux x86_64, --no-default-features. Horizon client0.1.0/Minecraft26.3 `41f822b659789780f58d32c63ddfb6887ea1b8bf`; mixins0.6.0 `5a64af938ec86b00d53d16a408aabc0038fb3e57`; Rust1.99.1. [Native log](../gekko/native-build.log), [binary metadata](../gekko/native-binary.json).

The 159406-byte .hnmod passes the official validator; all 100 packaged source files match source/native build inputs/staged crates. [Integrity](../gekko/source-integrity.json), [validator](../gekko/package-check.json), [changed files](FILES.md).

## Remaining integrations and acceptance

Sage Resurrection, Clove Not Dead Yet and Iso Kill Contract remain explicitly disabled: safe respawn/inventory/Hardcore or isolated-arena transactional integration is unavailable. No replacement is presented as the original ability. Gekko Spike interactions are excluded by the original ordinary-world scope.

Minecraft HP, cooldowns and resource regeneration are adaptations. Player fog/blindness does not alter vanilla mob perception; physical fallback and local pathfinding limits are documented per agent. Temporary state resets on server restart. New per-kit Settings are Rust configuration, not a new external configuration editor.

Next work: manual survival/creative and LAN acceptance against the pinned client, checking visuals, audio, controls, ordinary combat, cleanup/disconnect and simultaneous ability load; then the missing respawn/arena integrations. No world/LAN/render/audio acceptance is claimed by these checks. Other platforms/features remain unverified.
