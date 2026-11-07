# Изменения относительно MVP 0.1.0

Сохранены существующие реестр, runtime, настройки, меню и HUD. Добавлены независимые комплекты агентов, общее серверное ядро и тонкие адаптеры Horizon. Исходный checkout Horizon не является частью изменений мода.

## Изменённые файлы

- `FILES.md`
- `README.md`
- `engine/Cargo.toml`
- `engine/src/abilities.rs`
- `engine/src/agents.rs`
- `engine/src/lib.rs`
- `helpers/client_tick_post.rs`
- `helpers/key.rs`
- `horizon-mixin.toml`
- `scripts/package.py`
- `scripts/verify.py`
- `verification/RESULTS.md`
- `verification/core.log`
- `verification/integration-build.log`
- `verification/integration-check.log`
- `verification/mixins-apply.json`
- `verification/package.json`

## Новые файлы

- `bridge/client.rs`
- `bridge/damage.rs`
- `bridge/deaths.rs`
- `bridge/server.rs`
- `docs/ABILITIES-AUDIT.md`
- `docs/JETT.md`
- `docs/MANUAL-TESTS.md`
- `docs/OMEN.md`
- `docs/PHOENIX.md`
- `docs/RAZE.md`
- `docs/REYNA.md`
- `docs/SAGE.md`
- `engine/src/agents/jett.rs`
- `engine/src/agents/omen.rs`
- `engine/src/agents/phoenix.rs`
- `engine/src/agents/raze.rs`
- `engine/src/agents/reyna.rs`
- `engine/src/agents/sage.rs`
- `engine/src/gameplay/barrier.rs`
- `engine/src/gameplay/deaths.rs`
- `engine/src/gameplay/effects.rs`
- `engine/src/gameplay/explosions.rs`
- `engine/src/gameplay/geometry.rs`
- `engine/src/gameplay/mod.rs`
- `engine/src/gameplay/objects.rs`
- `engine/src/gameplay/projectiles.rs`
- `engine/src/gameplay/smoke.rs`
- `engine/src/gameplay/teleport.rs`
- `engine/src/gameplay/zones.rs`
- `engine/src/server.rs`
- `engine/tests/jett.rs`
- `engine/tests/omen.rs`
- `engine/tests/phoenix.rs`
- `engine/tests/protocol.rs`
- `engine/tests/raze.rs`
- `engine/tests/reyna.rs`
- `engine/tests/sage.rs`
- `engine/tests/support/mod.rs`
- `helpers/block_changed.rs`
- `helpers/consume_key.rs`
- `helpers/damage_post.rs`
- `helpers/damage_pre.rs`
- `helpers/fog.rs`
- `helpers/incoming.rs`
- `helpers/interaction.rs`
- `helpers/mob_death.rs`
- `helpers/net_client.rs`
- `helpers/net_server.rs`
- `helpers/player_death.rs`
- `helpers/server_tick.rs`
- `helpers/stop.rs`
- `helpers/unload.rs`
- `helpers/world.rs`
- `helpers/world_enable.rs`
- `verification/agent-compilation.log`

## Артефакты

- `dist/valorant_horizon-0.2.0.hnmod` — исходный native bundle.
- `dist/valorant-horizon-source.zip` — полный проект, тесты, документация и отчёты; без target/Cargo.lock.
- Пакет 0.1.0 оставлен как исторический артефакт.

Подробный журнал и границы проверки: `verification/RESULTS.md`.


## Следующий этап: Brimstone (2026-11-07)

Изменены: README.md, engine/src/agents.rs, engine/src/server.rs, engine/src/lib.rs, engine/src/gameplay/{mod,zones,projectiles}.rs, bridge/{client,server}.rs.
Добавлены: engine/src/agents/brimstone.rs, engine/tests/brimstone.rs, docs/GROUP2.md, verification/group2/{tests,clippy,brimstone-core,apply,native-check}.log.

Проверено: 49 тестов (6 новых), Clippy без предупреждений, cargo check нативного Horizon --lib --no-default-features. Полная бинарная сборка и запуск Minecraft для этого этапа не выполнялись. Старые dist-архивы намеренно сохранены как предыдущий выпуск, не содержат Brimstone. Остальные семь агентов новой группы пока не реализованы.


## Продолжение: Viper

Добавлены engine/src/agents/viper.rs, engine/src/gameplay/toxin.rs, engine/tests/viper.rs, docs/VIPER.md и verification/group2/viper-*.log.
Изменены engine/src/agents.rs, engine/src/server.rs, engine/src/lib.rs, engine/src/gameplay/{mod,projectiles,zones}.rs, engine/tests/{protocol.rs,support/mod.rs}, bridge/{client,server}.rs, README.md, docs/GROUP2.md.

Общая система decay сохраняет долг восстановления в attachment жертвы; серверные токсичные объёмы синхронизируются отдельно от локального fog/геометрии. Тест Viper на быстрое пересечение стены и протокольный тест расширены. Проверки этого этапа находятся отдельно от предыдущих выпусков; итоги — verification/group2/VIPER-RESULTS.md.


## Необязательное делегирование DeepSeek

Добавлены AGENTS.md, scripts/deepseek_delegate.py, tests/test_deepseek_delegate.py, docs/DEEPSEEK.md, verification/deepseek/{tests.log,probe.json}. Изменены .gitignore и README.md. Клиент только возвращает предложения, не изменяет Rust/Horizon, не включает секреты в .hnmod и не заменяет основную модель Codex.


## Подготовка Astra после интеграции помощника

Добавлены engine/src/gameplay/remote_view.rs, engine/tests/remote_view.rs, docs/ASTRA-AUDIT.md и verification/remote-view/{core.log,native-check.log,RESULTS.md}. Изменён engine/src/gameplay/mod.rs. Итог: 63 Rust-теста, 14 Python-тестов; Astra ещё не готовый агент.


## Astra — 0.3.0-dev.1

Новые файлы: engine/src/agents/astra.rs, engine/src/gameplay/divide.rs, engine/tests/astra.rs, spatial/Cargo.toml, spatial/src/lib.rs, bridge/spatial.rs, helpers/astra_{arrows,audio,camera,hand}.rs, docs/ASTRA.md, verification/astra/*.

Изменены: horizon-mixin.toml, engine/Cargo.toml, engine/src/{agents,lib,server}.rs, engine/src/gameplay/{mod,effects,zones}.rs, engine/tests/protocol.rs, bridge/{client,server}.rs, helpers/{key,consume_key,interaction,unload,stop}.rs, scripts/{verify,package}.py, README.md, docs/{GROUP2,ASTRA-AUDIT}.md.

Добавлены общий запас/возврат звёзд, отдельная камера, Gravity Well/Nova Pulse/Nebula/Dissipate/Cosmic Divide, перехват стрел и звука, защита от повторных нажатий и очистка привязок при смене измерения. Артефакты этого этапа: dist/valorant_horizon-0.3.0-dev.1.hnmod и dist/valorant-horizon-0.3.0-dev.1-source.zip. Старые архивы — исторические, не обновлены.


## Этап Harbor — 0.3.0-dev.2

Новые: `engine/src/agents/harbor.rs`, `engine/src/gameplay/shields.rs`, `engine/tests/harbor.rs`, `docs/HARBOR.md`, `verification/harbor/`.

Изменены: `engine/src/agents.rs`, `engine/src/lib.rs`, `engine/src/server.rs`, `engine/src/gameplay/mod.rs`, `engine/src/gameplay/projectiles.rs`, `engine/src/gameplay/zones.rs`, `engine/tests/protocol.rs`, `bridge/client.rs`, `bridge/server.rs`, `helpers/key.rs`, `helpers/client_tick_post.rs`, `engine/Cargo.toml`, `spatial/Cargo.toml`, `horizon-mixin.toml`, `README.md`, `docs/GROUP2.md`, `FILES.md`.

Артефакты: `dist/valorant_horizon-0.3.0-dev.2.hnmod`, `dist/valorant-horizon-0.3.0-dev.2-source.zip`. Старые архивы сохранены. Список не включает scratch-сборку Horizon в /workspace/work и временные Cargo targets.


## Этап Clove — 0.3.0-dev.3

Новые: `engine/src/agents/clove.rs`, `engine/src/gameplay/combat.rs`, `engine/tests/clove.rs`, `helpers/clove_death_hud.rs`, `helpers/clove_death_consume.rs`, `docs/CLOVE.md`, `verification/clove/`.

Изменены: `engine/src/agents.rs`, `engine/src/lib.rs`, `engine/src/server.rs`, `engine/src/gameplay/mod.rs`, `engine/src/gameplay/projectiles.rs`, `engine/src/gameplay/toxin.rs`, `bridge/server.rs`, `bridge/client.rs`, `bridge/deaths.rs`, `helpers/key.rs`, `helpers/damage_post.rs`, `helpers/stop.rs`, `horizon-mixin.toml`, `engine/Cargo.toml`, `spatial/Cargo.toml`, `README.md`, `docs/GROUP2.md`, `FILES.md`.

Артефакты: `dist/valorant_horizon-0.3.0-dev.3.hnmod`, `dist/valorant-horizon-0.3.0-dev.3-source.zip`. Предыдущие пакеты сохранены. Not Dead Yet не реализована и в комплект работающих способностей не включается.
