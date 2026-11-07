# Viper: результаты этапа 2026-11-07

PASS: fmt, rustfmt bridge/helpers, Clippy --all-targets -D warnings, 59 тестов (10 Viper), release-сборка engine. PASS: mixins apply, нативный cargo check --lib --no-default-features --locked и полная cargo build --bin horizon-client --no-default-features --locked. Две конфигурации с audio/controllers по умолчанию отдельно не проверялись.

Horizon 41f822b659789780f58d32c63ddfb6887ea1b8bf / Minecraft 26.3; mixins 5a64af938ec86b00d53d16a408aabc0038fb3e57. Логи этого этапа viper-*.log. Бинарник /workspace/work/horizon/target/debug/horizon-client, 312077016 байт, SHA-256 682e1757b79da1cc61120eb42527b1db5b8977b1dc573af90f4ad88d04d61009.

Выполнена проверка типов настоящих API и серверной модели с World test double. Запуск Minecraft/Survival/Creative/LAN не выполнялся: ресурсы не предоставлены. Нельзя считать проверенными визуализацию, движение, logout/login и реальное сохранение жертвы на сервере. Сценарии — docs/VIPER.md.

Viper: C Snake Bite, Q Poison Cloud, E Toxic Screen, X Viper’s Pit, выбор V/8. Топливо, decay и временные поля контролирует сервер, клиент только рисует полученный Snapshot. Проверен максимальный snapshot с восемью токсичными объёмами (<30000 байт). Ограничения: осевая стена, столкновение Snake Bite до приземления на пол, отсутствие подбора эмиттера, потолок текущих HP вместо изменения max_health. Настройки баланса пока Rust, не runtime UI.

Остальные шесть агентов группы ещё не реализованы. Следующий — Astra. Пакеты dist/0.2.0 не перезаписывались; текущая разработка находится в исходниках.
