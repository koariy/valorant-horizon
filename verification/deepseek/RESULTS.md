# DeepSeek: проверка прототипа 2026-11-07

14 offline-тестов PASS. Настоящий CLI GET /models: api_reachable=true, authenticated=false, HTTPS с проверкой сертификата. Обычный CLI без DEEPSEEK_API_KEY: status=disabled, fallback=codex, exit code 0. Сетевые ошибки, размер ответа, отсутствие исполнения tool calls, редактирование секретов, запрет redirects, deadline и межпроцессный SQLite-бюджет проверены с тестовыми данными.

Ключ в environment отсутствует; сообщение с ключом не использовалось. Авторизованный GET, платный POST, качество ответов и доступность модели аккаунту не проверены. Поэтому полученного от настоящего DeepSeek кода в проекте нет. Новые задачи выполнял Codex самостоятельно.

Официальные страницы https://api-docs.deepseek.com/ и https://api-docs.deepseek.com/api/create-chat-completion проверены. Используются deepseek-flash, thinking.type=disabled, фиксированный официальный endpoint. Скрипт не меняет модель Codex и не устанавливает MCP server. Инструкция docs/DEEPSEEK.md, политика проверки AGENTS.md.
