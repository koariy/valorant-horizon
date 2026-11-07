# Аудит способностей 0.2.0

Дата проверки исходников и официальных описаний: 2026-11-06. Дополняет аудит MVP, не заменяет его исторические результаты. Разработка шла последовательно: Jett, Phoenix, Sage, Omen, Raze, Reyna; перед следующим комплектом запускались тесты агента и проверка нативной интеграции предыдущего.

Horizon: `41f822b659789780f58d32c63ddfb6887ea1b8bf`, client 0.1.0, Minecraft 26.3.
Mixins: `5a64af938ec86b00d53d16a408aabc0038fb3e57`, 0.6.0.

Все ссылки ниже относятся к [проверенному commit Horizon](https://github.com/horizon-rust/horizon/tree/41f822b659789780f58d32c63ddfb6887ea1b8bf). Путь отсчитывается от корня репозитория. `gameplay::World` — собственная тестируемая абстракция мода, не API Horizon.

| Функция | Реализация | Ограничения / дополнительная интеграция | Проверенный источник |
|---|---|---|---|
| Серверный тик, UUID/измерение | level_tick_post, native_server_players, sessions(epoch,dimension), очереди намерений | сервер и клиенты Horizon с модом; максимум 64 отслеживаемых игроков | crates/horizon-client/src/game/native_hooks.rs; native_server.rs; crates/horizon-client/src/mod_api.rs; crates/horizon-mod-api-gen/src/lib.rs |
| Ввод C/Q/E/X, применение/отмена | key, interaction_key, native Game::key_pressed cancellable mixin | raw key_pressed нужен для поглощения Q/E; V/B зарезервированы | game/native_hooks.rs; game.rs; crates/horizon-client-input/src |
| Настройки/HUD | native_setting/set_setting, native_mod_key/set_mod_key, hud | серверные временные заряды не сохраняются между рестартами | game/native_hooks.rs; native_client_services.rs |
| Сетевой обмен | net_from_client/server; native_net_send_to_server/player | JSON запрос ≤512 байт, snapshot <30 KB, sequence/agent и лимиты очереди | game/native_net.rs; native_server.rs; native_hooks.rs |
| Коллизии и видимость | native_level_is_loaded/block, native_block_collision; общие swept AABB и segment ray | неизвестный чанк считается препятствием; используются стандартные формы блока без специального entity collision context | game/native_hooks.rs; native_level_refs.rs; crates/horizon-client/src/mod_api.rs |
| Перемещение Jett/Blast Pack | for_player, PlayerMotion.velocity, SetEntityMotion; перенос тела шагами с коллизией | требует проверки синхронизации/ощущения движения на LAN; ordinary fall для Blast Pack сохранён | game/commands/players.rs; motion.rs; game/commands.rs; horizon-protocol play packets |
| Телепортация Omen/Phoenix | target_by_uuid, teleport_to_in_level | только текущий мир/измерение, загруженная безопасная точка; без тактической карты | game/commands.rs:1529; game/commands/players.rs:95 |
| Урон снарядов/огня/взрыва | приватный command_damage_mob через отдельный child module; hurt_player_from для игроков | дополнительная source-level интеграция, не публичный стабильный Host API; vanilla броня, cooldown, PvP/team policy | game/commands/world_cmds.rs:396; game/creatures.rs:3716; health.rs; game/lan_pvp.rs |
| Ножи, огненные орбы, гранаты, ракеты | собственная серверная симуляция с непрерывной проверкой отрезка, нанесение настоящего урона | временные снаряды не vanilla сущности, не подбираются, без зависимости от оружия | приведённые методы урона + native world canvas; engine/src/gameplay/projectiles.rs |
| Дым и узнаваемые эффекты | native_world_quad, render_fog; сферы/объёмы из кода | не влияет на AI видимость, без Riot ассетов; сырой world_geometry guard нужен при mixin-only | game/native_world_draw.rs:124,147; native_hooks.rs; helpers/world_enable.rs |
| Вспышки/ограничение зрения | fog, штатный blindness через EffectInstance | зрение Horizon-игроков; vanilla blindness не равнозначен изменению AI мобов | game/native_hooks.rs; effects.rs; game/potions.rs |
| Лечение и замедление | apply_effect_action(Heal), add_player_effect; Mob.add_effect_with | Minecraft HP и эффекты; Sage союзная цель — игрок, не resurrection | game/potions.rs; effects.rs; mobs.rs |
| Overheal Reyna | PlayerMotion.internal_set_absorption и текущая health.absorption | разделяет pool absorption с Minecraft/другими модами, максимальный добавленный shield 10 HP | crates/horizon-client/src/attribute_updates.rs:27; health.rs:385 |
| Dismiss/Run It Back | incoming_damage отмена, damage_pre/post контроль фактического урона, Protection по network ID | void не отменяется; Run It Back возвращает на следующем тике; блокированная точка безопасно отклоняется | game/native_hooks.rs:7975,8010,8034; game.rs регистрация HURT_SERVER/DAMAGE_PRE/POST |
| Разрушаемая стена Sage | реальные frosted_ice через registry.default_state и native_set_block | прочность/таяние Minecraft, без оригинального укрепления; не заменяет занятые клетки | game/native_level_refs.rs; native_hooks.rs; assets registry |
| Сохранение владения стеной | AttachmentType<String>, Host level attachment, NBT codec, block_changed | постоянен только ledger блоков; чужие замены не удаляются, выгруженные клетки ждут загрузки | crates/horizon-world/src/attachments.rs; crates/horizon-client/src/mod_api.rs; game/native_server.rs; native_hooks.rs |
| Shade / Leer / Boom Bot / Blast Pack | общие логические серверные объекты: HP, коллизии, наведение и урон | не vanilla LivingEntity; обычные стрелы и AI их пока не видят; mod melee/knives/explosions поддержаны | engine/src/gameplay/objects.rs, server.rs; world draw + damage bridges |
| Убийства / Soul Orb / refill | provisional death из mob_died и guest_die, проверка живой цели на следующем тике, дедупликация; собственные попадания | нужны raw targets: public LivingDeath пропускается в mixin-only конфигурации; отсутствующий/неустановимый убийца не даёт награду | game/creatures.rs:1359–1395; game/lan_actions.rs:4061; guest_combat.last_blow |
| Resurrection | отключена с явной диагностикой и 0 зарядов | нужен проверенный respawn с inventory, hardcore и dimension policy, отдельные интеграционные тесты | приватный game/lan_actions.rs::guest_respawn, native death/respawn hooks |
| Empress оружейные бонусы | haste/speed и обновление состояния при убийстве | оружейные equip/reload/fire-rate Valorant отсутствуют; обещаний о них нет | EffectInstance / vanilla effect registry; будущий weapons module |

## Первичные описания агентов

Перед кодом каждого агента прочитаны тексты способностей на официальных страницах (значения кулдаунов/урона эти страницы полностью не задают):

- https://playvalorant.com/en-us/agents/jett/
- https://playvalorant.com/en-us/agents/phoenix/
- https://playvalorant.com/en-us/agents/sage/
- https://playvalorant.com/en-us/agents/omen/
- https://playvalorant.com/en-us/agents/raze/
- https://playvalorant.com/en-us/agents/reyna/

Оригинальные названия сохранены. Набор не включает игровой режим Valorant. Тайминги/заряды определены отдельно в DEFINITIONS каждого агента, один тик равен серверному тику Minecraft, обычно 1/20 секунды. Это игровые настройки адаптации, не заявление о точных числах актуального патча Valorant.

## Проверки и границы результата

Детальные результаты и логи находятся в verification. Headless-тесты проверяют реальную серверную модель с World double; они не проверяют сетевой клиент Minecraft, анимации, визуальную читаемость и совместимость с другими модами. Сборка адаптеров против настоящего Horizon проверяет существование и типы API. Запуск Minecraft здесь не выполнен из-за отсутствия игровых ресурсов; необходимые сценарии перечислены в MANUAL-TESTS.md.
