> Historical 0.2.0 report. Latest: [0.4.0-dev.10 verification](expansion/RESULTS.md).

# Проверки Valorant Horizon 0.2.0

Выполнены 2026-11-06 на Linux x86_64, Rust 1.99.1. Это исторические результаты версии 0.2.0.

| Проверка | Результат |
|---|---|
| cargo fmt --check для engine | PASS |
| rustfmt --check для helpers и bridge | PASS |
| cargo clippy --all-targets -- -D warnings для engine | PASS, без предупреждений |
| cargo test для engine | PASS, 43 теста, 0 ошибок |
| cargo build --release для engine | PASS |
| horizon-mixins apply | PASS, 19 инъекций, 4 модуля, 1 crate |
| cargo check Horizon --lib --no-default-features --locked | PASS |
| cargo build Horizon --bin horizon-client --no-default-features --locked | PASS, полный исполняемый клиент |
| cargo-horizon-mod check .hnmod --json | PASS, version 0.2.0 |
| Сверка исходников bundle ↔ проект ↔ final mixins input | PASS |
| Запуск Minecraft / LAN / визуальная приёмка | НЕ ВЫПОЛНЕНЫ: игровых ресурсов Minecraft нет |

## Отдельные тесты агентов

| Набор | Тестов | Что проверено |
|---|---|---|
| Jett | 9 | движение/коллизии, дым/управление, ножи/попадания/восстановление, заряды, Drift |
| Phoenix | 5 | огонь/лечение, вспышка/препятствия, Run It Back/летальный урон |
| Sage | 5 | блоки/владение/уборка, отказ без расхода, лечение/замедление, явный отказ Resurrection |
| Omen | 6 | безопасный перенос, Shade/отмена, дым/маркер, Paranoia |
| Raze | 6 | бот/контакт, satchel/импульс, осколки, ракета/стена, убийства/отмена |
| Reyna | 7 | Soul Orb/условия/срок, общий пул, Leer, лечение/overheal/спад, Empress, отменённая смерть |
| Базовое ядро | 4 | runtime, настройки, реестр, диапазоны |
| Протокол | 1 | максимальный snapshot меньше 30000 байт |

Тесты используют World test double. Они проверяют серверную игровую логику, не поведение настоящего Minecraft-клиента. После изменений общего ядра все наборы запускались повторно. Поэтапные проверки компиляции каждого агента сохранены в agent-compilation.log; итоговая сборка включает все последующие исправления.

## Совместимость и команды

Horizon client 0.1.0 / Minecraft 26.3: `41f822b659789780f58d32c63ddfb6887ea1b8bf`.
Horizon-mixins 0.6.0: `5a64af938ec86b00d53d16a408aabc0038fb3e57`.
Оба исходных checkout после генерации чистые. API других ревизий не проверялся.

```sh
python3 scripts/verify.py
horizon-mixins apply --workspace /path/to/horizon --mods /path/to/mods --out /path/to/staged
cargo check --locked --manifest-path /path/to/staged/Cargo.toml -p horizon-client --lib --no-default-features
cargo build --locked --manifest-path /path/to/staged/Cargo.toml -p horizon-client --bin horizon-client --no-default-features
python3 scripts/package.py
cargo-horizon-mod check dist/valorant_horizon-0.2.0.hnmod --json
```

Для интеграции использованы CARGO_BUILD_JOBS=2, CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_DEV_INCREMENTAL=false и общий CARGO_TARGET_DIR. Features audio/controllers в этой среде отключены; конфигурация с default features отдельно не проверена. Engine проверялся в временной копии, исходный bundle не содержит Cargo.lock/target.

Полный бинарный результат сборки: `/workspace/work/horizon/target/debug/horizon-client`, 311858008 байт, SHA-256 `e19644ba6e83b71f1709e23f029226f83af03d9bfc523e0b7ece676d5e0110e4`. Бинарник не запускался и не включён в исходный пакет.

## Артефакт

`dist/valorant_horizon-0.2.0.hnmod`: 52798 байт, SHA-256 `6af511f9607993a21da7f9ed48e7665a22f2fa7a85ca00852e856b1e87baab36`. Это source-only native bundle: установка требует штатной пересборки Horizon. Детерминированный архив содержит все вложенные модули агентов/gameplay и bridge. Отчёт штатного валидатора — package.json.

## Оставшиеся ограничения и следующие шаги

1. Выполнить docs/MANUAL-TESTS.md в Survival/Creative и на LAN, начиная с Jett. Проверить дым, fog, движение/импульсы, события смерти, стены после перезапуска и межмодовые отмены.
2. Отдельно интегрировать Resurrection со стандартным respawn, inventory и hardcore policy. Сейчас способность явно отключена.
3. Интегрировать логические объекты с vanilla projectile damage/AI; сейчас их поражают только mod melee, ножи и взрывы.
4. Добавить тактический интерфейс дальней телепортации Omen; сейчас безопасная точка выбирается взглядом до 64 блоков в загруженной области.
5. После игрового тестирования настроить Minecraft-баланс. Empress использует haste/speed, оружейные equip/reload бонусы ждут независимого weapons модуля.

Дым/ограничение зрения не переписывают восприятие vanilla AI. Frosted ice (minecraft:frosted_ice) имеет Minecraft прочность/таяние. Overheal использует общий pool absorption. Это явно описанные адаптации, а не утверждения о точном воспроизведении всех механик Valorant.
