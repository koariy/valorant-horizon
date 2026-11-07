# Valorant Horizon — тестовая сборка 0.3.0-dev.4

Файл для скачивания: `valorant_horizon-0.3.0-dev.4.hnmod`. Контрольная сумма — `SHA256SUMS.txt`.

Пакет создан из commit d1472c86bc9a0c4728f5fc3fe6e31d1711023a1f ветки feat/sova и совпадает с проверенным bundle (90467 байт). Это отдельная ветка только для скачивания пакета. Исходники и результаты тестов находятся в feat/sova.

.hnmod — нативный пакет Horizon с Rust-кодом: launcher пересобирает клиент с этим модом под систему пользователя. Это не JAR для Fabric/Forge. Совместимость: Minecraft 26.3, Horizon 41f822b659789780f58d32c63ddfb6887ea1b8bf, horizon-mixins 5a64af938ec86b00d53d16a408aabc0038fb3e57.

122 Rust-теста, 14 Python-тестов, Clippy, нативная сборка Horizon и валидатор пакета пройдены. Запуск в игровом мире и LAN ещё не проверен. Sage Resurrection и Clove Not Dead Yet отключены. Используй отдельный тестовый мир.
