# Valorant Horizon — тестовая сборка 0.4.0-dev.10

Скачать: [valorant_horizon-0.4.0-dev.10.hnmod](valorant_horizon-0.4.0-dev.10.hnmod). Исходники: [valorant_horizon-0.4.0-dev.10-source.zip](valorant_horizon-0.4.0-dev.10-source.zip). Контрольные суммы: [SHA256SUMS.txt](SHA256SUMS.txt).

Исходный commit `c286dd72cf3f46da28bd8ec2513af645b096c2b2`, ветка `feat/agents-expansion`. 22 агента из запрошенного списка; последовательно добавлены Cypher, Fade, Neon, Yoru, Iso, Waylay, KAY/O, Breach, Skye и Gekko.

331 Rust-тест, 14 Python-тестов, Clippy, нативная сборка Horizon и валидатор пакета прошли. Игровой мир/LAN/визуальные эффекты/звук ещё не проверены. Sage Resurrection, Clove Not Dead Yet и Iso Kill Contract отключены до соответствующих серверных интеграций. Подробный отчёт: https://github.com/koariy/valorant-horizon/blob/c286dd72cf3f46da28bd8ec2513af645b096c2b2/verification/expansion/RESULTS.md

.hnmod содержит Rust-код: совместимый launcher пересобирает Horizon под систему пользователя. Это не готовый JAR/DLL и не ресурсный .hmod. Совместимость: Horizon client0.1.0/Minecraft26.3 commit41f822b659789780f58d32c63ddfb6887ea1b8bf, mixins0.6.0 commit5a64af938ec86b00d53d16a408aabc0038fb3e57. Проверена Linux x86_64 сборка без default features. Нужна совместимая сборка на сервере и у участников.

Установка и управление: https://github.com/koariy/valorant-horizon/blob/c286dd72cf3f46da28bd8ec2513af645b096c2b2/README.md
