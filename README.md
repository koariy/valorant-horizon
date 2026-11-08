# Valorant Horizon — тестовая сборка 0.5.0-dev.1

Скачать: [valorant_horizon-0.5.0-dev.1.hnmod](valorant_horizon-0.5.0-dev.1.hnmod). Исходники: [valorant_horizon-0.5.0-dev.1-source.zip](valorant_horizon-0.5.0-dev.1-source.zip). [Контрольные суммы](SHA256SUMS.txt).

Исходный commit `da3836f128ed6bbc44a58b2c3e7d2d6d4946756e`, ветка `feat/agents-expansion`. 22 агента, общая система оформления, Low/Medium/High, оригинальные геометрические иконки, новый HUD, экран поиска/категорий/управления/локальных настроек. Инструкция: https://github.com/koariy/valorant-horizon/blob/da3836f128ed6bbc44a58b2c3e7d2d6d4946756e/docs/PRESENTATION.md

351 Rust-тест, 14 Python-тестов, Clippy, полная нативная сборка Horizon и валидатор пакета прошли. Все 110 файлов пакета совпадают с проверенными исходниками/стадией native build. Отчёт: https://github.com/koariy/valorant-horizon/blob/da3836f128ed6bbc44a58b2c3e7d2d6d4946756e/verification/presentation/RESULTS.md

Игровой мир/LAN/внешний вид/FPS/звук ещё не проверены. Прозрачность использует доступный проход Horizon с записью глубины; пересечения остаются приближёнными. Sage Resurrection, Clove Not Dead Yet и Iso Kill Contract отключены до соответствующих интеграций.

.hnmod содержит Rust-код: совместимый launcher пересобирает Horizon под систему пользователя. Это не готовый JAR/DLL и не ресурсный .hmod. Совместимость: Horizon client0.1.0/Minecraft26.3 commit41f822b659789780f58d32c63ddfb6887ea1b8bf; mixins0.6.0 commit5a64af938ec86b00d53d16a408aabc0038fb3e57. Проверена Linux x86_64 сборка без default features. Совместимая сборка нужна у сервера и участников.

Установка: https://github.com/koariy/valorant-horizon/blob/da3836f128ed6bbc44a58b2c3e7d2d6d4946756e/README.md
