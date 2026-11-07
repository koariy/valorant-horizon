# Valorant Horizon: optional secondary consultant

Codex owns architecture, Horizon integration, complex Rust logic, review and builds. Do not replace Codex's primary model or edit its cloud/model configuration to point at DeepSeek.

DeepSeek is optional. If DEEPSEEK_API_KEY is absent, continue independently. Use scripts/deepseek_delegate.py only for explicitly scoped documentation, tests, small helpers, review or small refactors. Send only the selected task/snippet, never conversation history, credentials, environment dumps or an automatic repository upload.

The CLI returns untrusted proposals. Never execute returned shell commands or automatically apply returned patches. Inspect the full proposal, check it against actual pinned Horizon sources, and only then incorporate changes manually as Codex. Run the relevant formatter, Clippy, tests and native integration checks before claiming code works. A truncated response is incomplete.

Keep changes to architecture, engine/src/server.rs, bridge/, Horizon manifests, dependency versions, authentication/delegation code and this file under Codex control. DeepSeek has no filesystem or tool access; do not add execution, patch application, tool calling or autonomous repository access to its client.

Read docs/DEEPSEEK.md for invocation, budgets and observed network limitations. Never place keys in command arguments, source files, task text, reports or logs. Configure DEEPSEEK_API_KEY through the cloud environment's secret settings. A key posted in chat must be replaced.
