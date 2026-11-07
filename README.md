# Valorant Horizon — 0.3.0-dev.3

Добавлена Clove: V → минус (-), см. `docs/CLOVE.md`: Pick-Me-Up, Meddle и Ruse, в том числе после смерти. Not Dead Yet отключена до безопасной интеграции respawn/инвентаря/Hardcore. Harbor выбирается V → 0. Итоговые проверки — `verification/clove/RESULTS.md`. Это версия разработки: игровая приёмка не выполнена. Предыдущие .hnmod 0.1/0.2 сохраняются отдельно.

Текущая разработка следующей группы: `docs/GROUP2.md`. Интеграция необязательного помощника DeepSeek: `docs/DEEPSEEK.md`, CLI `scripts/deepseek_delegate.py`; без ключа работает только Codex.

В исходниках добавлены Brimstone (V → 7) и Viper (V → 8, `docs/VIPER.md`); архивы dist/0.2.0 относятся к предыдущему выпуску. Игровая приёмка этих агентов ещё не выполнена.

Продолжение существующего Rust/.hnmod проекта для обычного мира Horizon: одиннадцать агентов, серверная симуляция способностей, меню, HUD и сохранение клавиш. Без раундов, Spike, экономики и обязательной зависимости от оружия. Это проверенная компиляцией и headless-тестами реализация; проверка в запущенной игре ещё нужна.

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

Resurrection требует проверенной интеграции respawn/инвентаря; подмена лечением отсутствует. Omen выбирает наземную точку взглядом до 64 блоков, отдельной тактической карты нет. Урон и тайминги адаптированы под Minecraft с 20 HP и обычными кулдаунами. Логические объекты имеют реальные серверные HP, но обычные стрелы пока не поражают их. Дым и ограничение зрения не меняют зрение vanilla AI. Empress не ускоряет отсутствующую перезарядку оружия Valorant. Геометрические эффекты создаются кодом, оригинальные ассеты Riot не включены.

## Установка

`dist/valorant_horizon-0.3.0-dev.3.hnmod` — исходный native bundle, для установки launcher пересобирает Horizon. Это не DLL и не Bukkit/Fabric-мод. Все участники и сервер должны использовать совместимую сборку Horizon с модом.

```sh
horizon-launcher settings native-profile dev
horizon-launcher native-build /absolute/path/to/valorant-horizon --instance YOUR_INSTANCE --trust-native-code
```

Используйте отдельный тестовый instance с собственными ресурсами Minecraft 26.3. Не передавайте `engine/target` и `engine/Cargo.lock` в native-build; пакет их не включает. Ресурсный .hmod не нужен: используются штатный шрифт и белый atlas sprite Horizon.

## Управление

- V: открыть меню; 1 Jett, 2 Sage, 3 Phoenix, 4 Omen, 5 Raze, 6 Reyna, 7 Brimstone, 8 Viper, 9 Astra, 0 Harbor, минус (-) Clove.
- C/Q/E/X: четыре слота. В меню та же клавиша выбирает слот для переназначения, следующая клавиша сохраняется. Esc отменяет ввод.
- ЛКМ/ПКМ: применение подготовленной способности / альтернативное применение. Особые действия описаны в документации агента.
- B: отмена подготовки/канала, потраченные заряды не возвращаются.
- Мод поглощает свои игровые клавиши: Q не выбрасывает предмет, E не открывает инвентарь при применении способности. F1 скрывает HUD, но не дым/ослепление.
- Агент и клавиши сохраняются штатными настройками. Кулдауны серверные, не замораживаются открытием инвентаря. Переход измерения/переподключение не восстанавливает заряды; активные каналы отменяются. Перезапуск сервера сбрасывает временные способности/заряды; временные блоки имеют отдельный постоянный ledger.
- Смена агента запрещена при активных состояниях или непогашенных кулдаунах, чтобы не обновлять заряды переключением.

## Архитектура и проверки

Сохранены реестр, настройки, HUD и runtime исходного MVP. `engine/src/agents/` содержит независимые комплекты; `gameplay/` — общие коллизии, дым, зоны, снаряды, телепортацию, эффекты, объекты и взрывы. `server.rs` валидирует запросы и владеет состоянием UUID/мира/измерения. `bridge/` и `helpers/` используют проверенные методы Horizon, не содержат вымышленных API. Клиент передаёт намерения, а не урон/координаты попадания. Состояние не блокируется mutex во время реентерабельных вызовов Horizon.

```sh
python3 scripts/verify.py
python3 scripts/package.py
```

Тесты каждого агента лежат отдельно в `engine/tests/`. Протокольный тест ограничивает максимальный snapshot ниже 30 KB. Проверка ядра не заменяет сборку интеграции. Ручные сценарии: `docs/MANUAL-TESTS.md`. Исторический аудит MVP сохранён в `docs/AUDIT.md`; актуальное дополнение — `docs/ABILITIES-AUDIT.md`.
