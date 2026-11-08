# Presentation audit — 2026-11-08

Pinned Horizon client0.1.0/Minecraft26.3 `41f822b659789780f58d32c63ddfb6887ea1b8bf`, mixins0.6.0 `5a64af938ec86b00d53d16a408aabc0038fb3e57`. Existing architecture and authoritative ability simulation are retained.

| Feature | Verified implementation path | Limits/source |
|---|---|---|
| World geometry, RGBA, animation | native_world_quad, NativeWorldDraw.camera/partial/game_time | crates/horizon-client/src/game/native_world_draw.rs; white atlas sprite, CPU geometry |
| Transparency | opaque core, alpha shell, camera-relative back-to-front submission | crates/horizon-client-render/src/gfx/world_text.rs: ALPHA_BLENDING, back-face culling, depth writes enabled. Not volumetric ray marching/OIT; intersecting translucent faces remain approximate |
| Smoke interior/flash/nearsight | existing fog hook | helpers/fog.rs, bridge/client.rs; gameplay visibility cannot be disabled by Low/F1 |
| Particles and positional sound | bounded local geometry; existing authoritative level particles/sounds | native_level_send_particles/native_level_play_sound; native_play_local_sound API83 for local UI feedback |
| HUD, original vector icons | native_fill/native_quad/native_text, HudBatch.scale_since | native_hooks.rs API111 and horizon-client-render/src/hud.rs. No extracted Riot assets |
| Agent search/categories/settings screen | native_mod_screen_open/close, screen render/key/char/mouse hooks | native_hooks.rs API78/80/104/111. Screen does not pause world; input ownership checked |
| Persistence | native_setting/native_set_setting and existing mod keys | native_hooks.rs; local presentation settings never accepted as server gameplay commands |
| Target highlight | existing authoritative private contacts + outline hook | helpers/recon_outline.rs; no revealing undiscovered targets |

Quality presets control cosmetic particle counts, mesh detail, distance and frame limits. Core smoke/vision and server damage/collisions/resources are independent. Tests and native compilation must pass before declaring integration verified. World/LAN/visual/audio acceptance remains separate.
