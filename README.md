# Valorant Horizon — тестовая сборка 0.3.0-dev.4

Файл для скачивания: `valorant_horizon-0.3.0-dev.4.hnmod`. Контрольная сумма — `SHA256SUMS.txt`.

Пакет создан из commit d1472c86bc9a0c4728f5fc3fe6e31d1711023a1f ветки feat/sova и совпадает с проверенным bundle (90467 байт). Это отдельная ветка только для скачивания пакета. Исходники и результаты тестов находятся в feat/sova.

.hnmod — нативный пакет Horizon с Rust-кодом: launcher пересобирает клиент с этим модом под систему пользователя. Это не JAR для Fabric/Forge. Совместимость: Minecraft 26.3, Horizon 41f822b659789780f58d32c63ddfb6887ea1b8bf, horizon-mixins 5a64af938ec86b00d53d16a408aabc0038fb3e57.

122 Rust-теста, 14 Python-тестов, Clippy, нативная сборка Horizon и валидатор пакета пройдены. Запуск в игровом мире и LAN ещё не проверен. Sage Resurrection и Clove Not Dead Yet отключены. Используй отдельный тестовый мир.

## Экспериментальный native .hmod

Добавлен `valorant_horizon-0.3.0-dev.4-native-experimental.hmod`. Это тот же нативный ZIP-пакет с `horizon-native.json` и Rust-исходниками, без фиктивного контентного манифеста и без WASM. Документированный native .hmod использует тот же layout, поэтому байты и SHA-256 совпадают с .hnmod.

**Это не готовая замена .hnmod для стандартной установки.** Библиотека Bundle принимает контейнер, но проверенный launcher commit 41f822b659789780f58d32c63ddfb6887ea1b8bf ищет установленные нативные файлы только с расширением .hnmod. Автоматическая проверка как контентного .hmod его отвергает. Результат явной проверки как native-mod — PASS; см. hmod-format-check.json.

Для обычного тестирования используй .hnmod. Нативный .hmod не позволяет запускать этот Rust-код без пересборки Horizon и не становится обычным ресурсным модом от смены расширения. Полная установка native .hmod требует соответствующей поддержки launcher/игры. Источники правил: docs/api/mod-files.md и crates/horizon-mods/src/index/native.rs закреплённого Horizon; ограничение установки — launcher/src/native_mods.rs::installed_mods, launcher/src/native_status.rs::is_native_file.
