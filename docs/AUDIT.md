# Аудит до реализации

Проверено по исходникам 41f822b659789780f58d32c63ddfb6887ea1b8bf. Horizon client 0.1.0, Minecraft Java 26.3. Это проверенная ревизия API, не подтверждение игрового запуска.

Документация: https://playhorizon.dev/docs/make-a-mod (получена 2026-11-06); исходные примеры: шаблон cargo-horizon-mod и launcher/tests/fixtures/native-crate-demo.

| Функция | Реализация | Ограничения | Проверенный источник |
|---|---|---|---|
| Регистрация агентов | Собственный реестр Rust | Не API Horizon; пока Jett и Sage, без оригинальных способностей | [source](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/crates/cargo-horizon-mod/template/native/horizon-mixin.toml) |
| Меню и HUD | hud → native_fill/native_text | MVP: немодальное меню на HUD, ввод не поглощается | [source](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/crates/horizon-client/src/game/native_hooks.rs#L2558) |
| Клавиши C/Q/E/X | key, native_playing, native_mod_key; set_mod_key | key наблюдает ввод; Q/E также выполняют действия Minecraft; переназначить конфликтующие клавиши | [source](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/crates/horizon-client/src/game/native_hooks.rs#L2700) |
| Тики, перезарядки, заряды, длительность | client_tick_post + собственный автомат | Клиентский тик идёт также при паузе; MVP замораживает таймеры, когда native_playing=false | [source](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/crates/horizon-client/src/game/native_hooks.rs#L2545) |
| Сохранение | native_setting/native_set_setting | Запись подавляет ошибки; MVP сохраняет выбор агента, клавиши сохраняет Horizon | [source](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/crates/horizon-client/src/game/native_hooks.rs#L9618) |
| Активные, пассивные, ультимативные | Собственные определения + серверные хуки для игровых эффектов | В MVP реализована только активная тестовая способность; пассивные/ультимативные — типы, без игровых обработчиков | [source](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/docs/native-mods.md) |
| Состояния и события | Собственная длительность импульса + события Started/Ended | Не vanilla potion effect; не синхронизируется | [source](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/docs/native-mods.md#the-clients-tick) |
| Цели, дальность, препятствия | native_player/native_camera; native_client_visual_clear | Клиентская видимость: стекло прозрачно; незагруженный путь отклоняется; для боя нужна серверная валидация | [source](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/crates/horizon-client/src/game/native_client_services.rs#L185) |
| Коллизии | native_block_collision, client_block_obstruction | Пустой collision context; незагруженные блоки могут дать пустые boxes; нельзя считать это свободным местом | [source](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/docs/native-mods.md#camera-and-collision-reads) |
| Сервер и multiplayer | server_tick, attachments, mod messages | Один экземпляр мода может обслуживать сервер и окно. MVP только клиентский, боевого воздействия нет | [source](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/docs/native-mods.md#server-attachments) |
| Оружие и VFX | use hooks, raycast, render hooks | Не реализованы; потребуют authority, моделей, анимаций и отдельной проверки | [source](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/native-hooks.json) |
| Нативный пакет | horizon-mixin.toml + [[crate]] и HEAD helpers | Исходники в ZIP .hnmod, не динамическая библиотека; launcher пересобирает игру | [source](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/crates/horizon-mods/src/index/native.rs) |

Horizon-mixins 0.6.0, commit `5a64af938ec86b00d53d16a408aabc0038fb3e57` из launcher/src/native_mods.rs. Выбраны документированные mixin helpers и независимый crate: это позволяет тестировать автомат без компиляции движка. Ревизия закреплена: прямые native_* helpers не обещают переносимость между версиями. Отдельный ресурсный .hmod в MVP не требуется.

Обнаружено при проверке: `client_tick_post` существует в исходниках и pending fragment, но отсутствует в корневом native-hooks.json. Поэтому используется подтверждённый raw target `Game::native_client_tick_post` с точной фиксацией ревизии, а не несуществующий для mixins каталоговый hook.

### Где нужна дополнительная интеграция

- `key` имеет контракт listen, не cancel. Нельзя отменить выбрасывание предмета/инвентарь возвращаемым значением этого хука. Для бесконфликтных C/Q/E/X нужен отдельный cancellable pre-key hook Horizon либо точная инъекция в обработку клавиатуры с новой проверкой ревизии. Существующие screen key hooks относятся к открытым экранам, interaction_key — к attack/use/pick, и не заменяют общий перехват Q/E. В MVP предлагается переназначение.
- Клиентский `raycast` и `client_visual_clear` нельзя использовать как источник авторитетного урона. Сервер должен заново проверять отправителя, измерение, позицию, доступность способности и геометрию. Доступность строительных блоков API подтверждена, готового Valorant-протокола в Horizon нет.
- Для рывков недостаточно block-local collision boxes с пустым контекстом. Нужны loaded-only проверки и swept AABB реального игрока на сервере; проверка одного конечного блока может пропустить стену. Эта интеграция пока не реализована.
- Произвольное всплывающее меню выбора не следует выдавать за готовый native container. MVP рисует немодальное HUD-меню; полноценный экран требует адаптера экрана, обработки фокуса/поглощения ввода и отдельной проверки контроллера.
