# Valorant Horizon — 0.4.0-dev.8

Rust/.hnmod мод для обычного мира Horizon: 20 агентов, серверные способности, меню, HUD и сохранение клавиш. Без раундов, Spike, экономики и обязательной зависимости от оружия. Текущий этап — Breach; механики и ограничения в `docs/BREACH.md`, результаты проверок в `verification/breach/RESULTS.md`.

Проверки выполняются в коде и нативной сборке. Игровой запуск и LAN-приёмка ещё не проведены. Sage Resurrection, Clove Not Dead Yet и Iso Kill Contract отключены до соответствующей серверной интеграции. Необязательный DeepSeek CLI: `docs/DEEPSEEK.md`; без ключа Codex работает самостоятельно.

## Совместимость

- Horizon client 0.1.0 / Minecraft Java 26.3, commit `41f822b659789780f58d32c63ddfb6887ea1b8bf`.
- horizon-mixins 0.6.0, commit `5a64af938ec86b00d53d16a408aabc0038fb3e57`.
- Проверки выполнялись Rust 1.99.1 на Linux, без default features клиента.
- Raw mixin targets привязаны к этим ревизиям. Другие версии не проверены.
- API и ограничения: `docs/ABILITIES-AUDIT.md`; результаты: `verification/RESULTS.md`.

## Возможности и ограничения

| Агент | Реализованная серверная механика | Подробности |
|---|---|---|
| Jett | Updraft, Tailwind, управляемый Cloudburst, Blade Storm с уроном/попаданиями, Drift | `docs/JETT.md` |
| Phoenix | Blaze, Curveball, Hot Hands, точка возврата Run It Back | `docs/PHOENIX.md` |
| Sage | разрушаемая стена, Slow Orb, Healing Orb; Resurrection отключена | `docs/SAGE.md` |
| Omen | Dark Cover, Paranoia, Shrouded Step, From the Shadows с отменяемым Shade | `docs/OMEN.md` |
| Raze | движущийся Boom Bot, Blast Pack, Paint Shells с осколками, ракета Showstopper | `docs/RAZE.md` |
| Reyna | разрушаемый Leer, Soul Orb, Devour/overheal, Dismiss, Empress | `docs/REYNA.md` |
| Brimstone | Stim Beacon, Incendiary, Sky Smoke, Orbital Strike | `docs/GROUP2.md` |
| Viper | Snake Bite, Poison Cloud, Toxic Screen, Viper’s Pit, общий ресурс токсина | `docs/VIPER.md` |
| Astra | звёзды, Gravity Well, Nova Pulse, Nebula/Dissipate, Astral Form/Cosmic Divide | `docs/ASTRA.md` |
| Harbor | High Tide, Storm Surge, Cove с HP щита, Reckoning; выбор Cove взглядом | `docs/HARBOR.md` |
| Clove | Pick-Me-Up с damaging assist, Meddle decay, Ruse после смерти; Not Dead Yet отключена | `docs/CLOVE.md` |
| Sova | управляемый дрон/дротик, заряжаемые Shock/Recon Bolt с отскоками, приватная разведка, три луча Fury | `docs/SOVA.md` |
| Cypher | растяжки с HP/tether/reveal, активируемые клетки, камера с дротиком, раскрытие по подтверждённой смерти | `docs/CYPHER.md` |
| Fade | Prowler с преследованием/следами, удержание Seize, разрушаемый Haunt по LOS, Nightfall с обратимым decay и deafen | `docs/FADE.md` |
| Neon | High Gear с энергией/slide, отражающийся Relay Bolt, две стены Fast Lane, Overdrive и восстановление по kill credit | `docs/NEON.md` |
| Breach | Aftershock/Flashpoint за выбранной стеной, заряжаемая Fault Line и расширяющийся Rolling Thunder с реальным knock-up | `docs/BREACH.md` |
| KAY/O | FRAG/MENT с импульсами, FLASH/DRIVE, разрушаемый ZERO/POINT и общая suppression, NULL/CMD со stim/downed/стабилизацией союзником | `docs/KAYO.md` |
| Waylay | наземный Saturate/Hinder, один/два Lightspeed с коллизиями, защищённый Refract по пути, расширяющийся Convergent Paths и speed | `docs/WAYLAY.md` |
| Iso | движущаяся неразрушаемая стена, проходящий стены FRAGILE, focus/flow и приватные shootable орбы с одноразовым щитом; X отключён | `docs/ISO.md` |
| Yoru | движущееся/неактивное Fakeout с flash, отражающийся Blindside, настоящий/ложный Gatecrash, защищённый Drift; стилизованное эхо без копии личного скина | `docs/YORU.md` |

Resurrection требует проверенной интеграции respawn/инвентаря; подмена лечением отсутствует. Omen выбирает наземную точку взглядом до 64 блоков, отдельной тактической карты нет. Урон и тайминги адаптированы под Minecraft с 20 HP и обычными кулдаунами. Логические объекты имеют серверные HP; обычные/спектральные стрелы, трезубцы, ножи Jett, melee и общие взрывы могут поражать их. Прочие native projectiles отдельно не интегрированы. Дым и ограничение зрения не меняют зрение vanilla AI. Empress не ускоряет отсутствующую перезарядку оружия Valorant. Геометрические эффекты создаются кодом, оригинальные ассеты Riot не включены.

## Установка

`dist/valorant_horizon-0.4.0-dev.8.hnmod` — исходный native bundle, для установки launcher пересобирает Horizon. Это не DLL и не Bukkit/Fabric-мод. Все участники и сервер должны использовать совместимую сборку Horizon с модом.

```sh
horizon-launcher settings native-profile dev
horizon-launcher native-build /absolute/path/to/valorant-horizon --instance YOUR_INSTANCE --trust-native-code
```

Используйте отдельный тестовый instance с собственными ресурсами Minecraft 26.3. Не передавайте `engine/target` и `engine/Cargo.lock` в native-build; пакет их не включает. Ресурсный .hmod не нужен: используются штатный шрифт и белый atlas sprite Horizon.

## Управление

- V: меню; Left/Right переключают страницы по 10 агентов. На первой странице 1 Jett, 2 Sage, 3 Phoenix, 4 Omen, 5 Raze, 6 Reyna, 7 Brimstone, 8 Viper, 9 Astra, 0 Harbor; на второй 1 Clove, 2 Sova, 3 Cypher, 4 Fade, 5 Neon, 6 Yoru, 7 Iso, 8 Waylay, 9 KAY/O, 0 Breach. Сохранены быстрые клавиши минус (-) Clove и = Sova. R возвращает устройства Cypher вблизи.
- C/Q/E/X: четыре слота. В меню та же клавиша выбирает слот для переназначения, следующая клавиша сохраняется. Esc отменяет ввод.
- ЛКМ/ПКМ: применение подготовленной способности / альтернативное применение. Особые действия описаны в документации агента.
- B: отмена подготовки/канала, потраченные заряды не возвращаются.
- Мод поглощает свои игровые клавиши: Q не выбрасывает предмет, E не открывает инвентарь при применении способности. F1 скрывает HUD, но не дым/ослепление.
- Агент и клавиши сохраняются штатными настройками. Кулдауны серверные, не замораживаются открытием инвентаря. Переход измерения/переподключение не восстанавливает заряды; активные каналы отменяются. Перезапуск сервера сбрасывает временные способности/заряды; временные блоки имеют отдельный постоянный ledger.
- Смена агента запрещена при активных состояниях или непогашенных кулдаунах, чтобы не обновлять заряды переключением.

## Архитектура и проверки

Сохранены реестр, настройки, HUD и runtime исходного MVP. `engine/src/agents/` содержит прежние независимые комплекты, `engine/src/advanced/` — новые комплекты агентов с общим диспетчером и бюджетом; `gameplay/` — общие коллизии, дым, зоны, снаряды, телепортацию, эффекты, объекты и взрывы. `server.rs` валидирует запросы и владеет состоянием UUID/мира/измерения. `bridge/` и `helpers/` используют проверенные методы Horizon, не содержат вымышленных API. Клиент передаёт намерения, а не урон/координаты попадания. Состояние не блокируется mutex во время реентерабельных вызовов Horizon.

```sh
python3 scripts/verify.py
python3 scripts/package.py
```

Тесты каждого агента лежат отдельно в `engine/tests/`. Протокольный тест ограничивает максимальный snapshot ниже 30 KB. Проверка ядра не заменяет сборку интеграции. Ручные сценарии: `docs/MANUAL-TESTS.md`. Исторический аудит MVP сохранён в `docs/AUDIT.md`; актуальное дополнение — `docs/ABILITIES-AUDIT.md`.
