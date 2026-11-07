"""Offline security/transport tests; no paid requests, real credentials or Rust edits."""
import contextlib
import io
import json
import os
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest
from unittest.mock import patch
import urllib.error

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import deepseek_delegate as d


class FakeResponse:
    def __init__(self, data):
        self.raw = json.dumps(data).encode()

    def __enter__(self):
        return self

    def __exit__(self, *args):
        return False

    def read(self, maximum):
        return self.raw[:maximum]


class ClientTests(unittest.TestCase):
    def test_no_key_disables_without_network_or_input(self):
        with patch.dict(os.environ, {}, clear=True), patch.object(d, "run_http") as network:
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                self.assertEqual(d.main([]), 0)
            self.assertEqual(json.loads(out.getvalue())["status"], "disabled")
            network.assert_not_called()

    def test_secondary_task_uses_official_endpoint_and_env_header(self):
        response = FakeResponse({"choices": [{"message": {"content": "Proposed test"}}]})
        with patch.object(d.urllib.request, "build_opener") as factory:
            factory.return_value.open.return_value = response
            result = d.http_call("dummy-test-value", {"model": "deepseek-flash"}, 3)
            request = factory.return_value.open.call_args.args[0]
            self.assertEqual(request.full_url, "https://api.deepseek.com/chat/completions")
            self.assertEqual(request.get_header("Authorization"), "Bearer dummy-test-value")
            self.assertTrue(result["review_required"])
            self.assertEqual(result["status"], "proposal")

    def test_delegate_has_bounded_non_thinking_proposal_only_payload(self):
        with patch.dict(os.environ, {"DEEPSEEK_API_KEY": "dummy-test-value"}), patch.object(d, "run_http") as net:
            net.return_value = dict(status="proposal", review_required=True, content="test")
            d.delegate("Suggest edge cases.", "tests")
            payload = net.call_args.args[0]
            self.assertEqual(payload["model"], "deepseek-flash")
            self.assertEqual(payload["thinking"], {"type": "disabled"})
            self.assertEqual(payload["max_tokens"], 2048)
            self.assertNotIn("tools", payload)
            self.assertNotIn("dummy-test-value", json.dumps(payload))

    def test_secret_in_task_rejected_before_network(self):
        with patch.dict(os.environ, {"DEEPSEEK_API_KEY": "dummy-test-value"}), patch.object(d, "run_http") as net:
            for task in ["Use dummy-test-value", "Bearer example", "sk-" + "x" * 32]:
                with self.assertRaises(d.Failure) as error:
                    d.delegate(task, "review")
                self.assertEqual(error.exception.result["error"], "credential_in_task")
            net.assert_not_called()

    def test_errors_do_not_echo_provider_body_or_exception(self):
        for status, code in [(401, "authentication_failed"), (402, "insufficient_balance"),
                             (429, "provider_rate_limit"), (503, "http_error"), (302, "http_error")]:
            with self.subTest(status=status), patch.object(d.urllib.request, "build_opener") as factory:
                factory.return_value.open.side_effect = urllib.error.HTTPError(
                    d.API, status, "PRIVATE-dummy-test-value", {}, io.BytesIO(b"PRIVATE"))
                with self.assertRaises(d.Failure) as error:
                    d.http_call("dummy-test-value", {}, 3)
                self.assertEqual(error.exception.result["error"], code)
                self.assertNotIn("PRIVATE", json.dumps(error.exception.result))

    def test_no_redirect_can_forward_authorization(self):
        handler = d.NoRedirect()
        self.assertIsNone(handler.redirect_request(None, None, 307, "", {}, "https://example.invalid"))

    def test_probe_401_means_reachable_not_authenticated(self):
        with patch.object(d.urllib.request, "build_opener") as factory:
            factory.return_value.open.side_effect = urllib.error.HTTPError(d.API, 401, "", {}, None)
            result = d.http_call("", None, 3, probe=True)
            self.assertTrue(result["api_reachable"])
            self.assertFalse(result["authenticated"])

    def test_secret_in_output_is_redacted_and_tool_calls_rejected(self):
        with patch.object(d.urllib.request, "build_opener") as factory:
            factory.return_value.open.return_value = FakeResponse(
                {"choices": [{"message": {"content": "dummy-test-value sk-" + "x" * 32}}]})
            result = d.http_call("dummy-test-value", {}, 3)
            self.assertNotIn("dummy-test-value", result["content"])
            self.assertNotIn("sk-", result["content"])
            factory.return_value.open.return_value = FakeResponse(
                {"choices": [{"message": {"content": "run me", "tool_calls": [{"name": "shell"}]}}]})
            with self.assertRaises(d.Failure):
                d.http_call("dummy-test-value", {}, 3)

    def test_bad_or_truncated_response_is_explicit(self):
        with patch.object(d.urllib.request, "build_opener") as factory:
            for document in [{}, [], {"choices": [{"message": "invalid"}]}]:
                factory.return_value.open.return_value = FakeResponse(document)
                with self.assertRaises(d.Failure):
                    d.http_call("", {}, 3)
            factory.return_value.open.return_value = FakeResponse(
                {"choices": [{"finish_reason": "length", "message": {"content": "partial"}}]})
            self.assertTrue(d.http_call("", {}, 3)["truncated"])

    def test_oversized_response_rejected(self):
        with patch.object(d.urllib.request, "build_opener") as factory:
            factory.return_value.open.return_value = FakeResponse("x" * d.MAX_RESPONSE)
            with self.assertRaises(d.Failure) as error:
                d.http_call("", {}, 3)
            self.assertEqual(error.exception.result["error"], "response_too_large")

    def test_argument_errors_do_not_repeat_sensitive_arguments(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            self.assertEqual(d.main(["--api-key", "DUMMY-PRIVATE"]), 2)
        self.assertNotIn("DUMMY-PRIVATE", out.getvalue())

    def test_timeout_terminates_worker(self):
        # Fake a worker stuck beyond its deadline; this test never accesses the network.
        with patch.object(d.multiprocessing, "get_context") as context:
            receiver, sender = context.return_value.Pipe.return_value = (unittest.mock.Mock(), unittest.mock.Mock())
            process = context.return_value.Process.return_value
            receiver.poll.return_value = False
            process.pid = 10
            process.is_alive.return_value = True
            with self.assertRaises(d.Failure) as error:
                d.run_http({}, 1)
            self.assertEqual(error.exception.result["error"], "deadline_exceeded")
            process.terminate.assert_called_once()
            process.kill.assert_called_once()


class LimiterTests(unittest.TestCase):
    def test_shared_pacing_lease_daily_cap_and_expiry(self):
        with tempfile.TemporaryDirectory() as folder:
            first, second = d.Limiter(folder), d.Limiter(folder)
            try:
                token = first.reserve(5, daily=2, now=1000)
                with self.assertRaises(d.Failure) as e:
                    second.reserve(5, daily=2, now=1001)
                self.assertEqual(e.exception.result["error"], "request_in_progress")
                second.release("wrong-token")
                with self.assertRaises(d.Failure):
                    second.reserve(5, now=1001)
                first.release(token)
                with self.assertRaises(d.Failure) as e:
                    second.reserve(5, now=1001)
                self.assertEqual(e.exception.result["error"], "local_rate_limit")
                second.reserve(5, daily=2, now=1011)  # Crash: do not release.
                with self.assertRaises(d.Failure) as e:
                    first.reserve(5, daily=2, now=1022)
                self.assertEqual(e.exception.result["error"], "daily_budget_exhausted")
                first.reserve(5, daily=2, now=90000)  # Next UTC day, expired lease.
            finally:
                first.close()
                second.close()
            raw = Path(folder, "budget.sqlite3").read_bytes()
            self.assertNotIn(b"DEEPSEEK_API_KEY", raw)

    def test_cli_failure_releases_lease_and_reports_fallback(self):
        with tempfile.TemporaryDirectory() as folder, patch.dict(os.environ, {"DEEPSEEK_API_KEY": "dummy"}), patch.object(d, "run_http") as net:
            task = Path(folder, "task.txt")
            task.write_text("Review this small helper.")
            net.side_effect = d.Failure("deadline_exceeded")
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                self.assertEqual(d.main(["--kind", "review", "--task-file", str(task), "--state-dir", folder]), 1)
            self.assertEqual(json.loads(out.getvalue())["fallback"], "codex")
            db = sqlite3.connect(Path(folder, "budget.sqlite3"))
            self.assertEqual(db.execute("SELECT lease FROM budget").fetchone()[0], 0)
            db.close()


if __name__ == "__main__":
    unittest.main()
