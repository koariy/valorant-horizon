# Skye — аудит перед реализацией

Официальная страница https://playvalorant.com/en-us/agents/skye/ проверена 2026-11-08; тексты совпали с verification/expansion/skye-official.json. Regrowth лечит союзников в LOS из конечного пула, без self-heal. Trailblazer — управление наземным хищником с пounce/взрывом, concuss и прямым уроном. Guiding Light — управляемый удержанием FIRE hawk, повтор E даёт flash и hit confirm по enemy range/LOS. Seekers — до трёх разрушаемых преследователей ближайших разных целей с nearsight.

| Функция | Реализация | Ограничения | Проверенный источник |
|---|---|---|---|
| Regrowth | Серверный channel/LOS/ally pool | Scoreboard allies; no-self/no-downed; Minecraft HP/rate | World::heal, Actor.max_health, native living_heal/1 |
| Trailblazer | Camera + walking::step и swept leap | Body остаётся уязвимым и закреплённым; локальные обходы без глобального pathfinder | Native camera402 + существующий Camera/RemoteView, World::boxes/push_entity |
| Guiding Light | Короткий swept flight, управляемый hold FIRE; ручной flash | Collision/TTL, общий LOS/facing flash; оригинальные assets не извлекаются | throwable::advance, Context::flash, private kit HUD |
| Seekers | До трёх серверных объектов с HP и отдельными target/TTL | Ground/step/unknown chunks, лимиты поиска и budget | walking::step, Context::enemies, World::effect, Vision.nearsight |

Нет новых вымышленных событий SDK. Нужны отдельные тесты remote body, pounce vs terrain/actor/phase, реального HP-heal и расхода пула, hawk guidance/flash confirmation, уникальных Seeker targets и cleanup. Native сборка проверяется до перехода к Gekko.

Horizon client0.1.0/Minecraft26.3 commit41f822b659789780f58d32c63ddfb6887ea1b8bf; mixins0.6.0 commit5a64af938ec86b00d53d16a408aabc0038fb3e57. Результаты реализации и проверок — verification/skye/RESULTS.md.

## Управление и адаптация

V → Right → Right → 1 выбирает Skye. C/удержание ЛКМ лечит раненых scoreboard-союзников в пределах 8 блоков и LOS, без self-heal. Общий пул 20 HP (80 четвертей) расходуется один раз за успешный tick, независимо от числа союзников; отсутствующие, здоровые, dead/downed и недоступные цели его не расходуют. Отпускание ЛКМ или B останавливает канал. На время канала применяются slowdown и серверный запрет обычных атак/предметов. В обычном мире без раундов пул восстанавливается после паузы, это явная адаптация к survival.

Q/ЛКМ создаёт разрушаемого Trailblazer: управление движением, ЛКМ для pounce. Тело остаётся уязвимым у исходной точки; входящий HP-урон, разрушение существа, B или TTL завершают управление. Swept pounce наносит 6 HP прямой цели и concuss ближайшим врагам с LOS; не наносит урон сквозь стену и не загружает чанки. Полблока/локальный обход препятствий обрабатываются общим walking.

E/ЛКМ запускает hawk. Удержание ЛКМ направляет его взглядом; отпускание оставляет последнее направление. Повтор E даёт flash, TTL также автоматически даёт flash согласно официальной patch8.01. Уничтожение/неизвестный чанк не создают ложную вспышку и освобождают управление. Подтверждение по enemy range/LOS отображается только в kit HUD кастера; звук локализован у его тела и не является скрытой передачей координат.

X/ЛКМ выпускает до трёх Seekers по ближайшим разным противникам. Они имеют HP, конечный срок, локальный наземный обход и реальные коллизии. Достигнутая цель получает player nearsight через native blindness и плотный клиентский fog. Ванильный AI мобов не читает этот fog: для них явно применяется адаптация slowness/weakness вместо заявления о потере AI-зрения. Это реальное изменение движения/атаки, не переименование декоративных частиц в AI blind. Глобальная навигация по лабиринтам не реализована.

Настройки Settings ограничивают пул/восстановление и сроки существ/эффектов; внешний GUI этих Rust-параметров не добавлен. В обычном мире заряды имеют долгий renewable cooldown, а не покупку/сброс раунда; это относится и к hawk, который в Valorant после patch8.01 больше не перезаряжается внутри раунда. Сохранены настройки клавиш/выбранного агента. Клиент рисует зелёные силуэты/птицу/Seekers и лучи лечения, сервер определяет HP, цели и движение. Игровой запуск, LAN и визуальная/звуковая приёмка не выполнялись.
