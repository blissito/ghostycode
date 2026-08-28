"""Run Ghosty through a Verifiers v0.2 interception endpoint.

The harness owns an isolated Ghosty home for every rollout, forwards only
the interception secret supplied by Verifiers, and retains a bounded terminal
receipt rather than copying raw program output into trace metadata.
"""

from __future__ import annotations

import hashlib
import json
import logging
import re
import shlex
from collections import Counter
from typing import Any, Literal

from verifiers.v1.clients import ModelContext
from verifiers.v1.harness import Harness, HarnessConfig
from verifiers.v1.runtimes import ProgramResult, Runtime
from verifiers.v1.trace import Trace

logger = logging.getLogger(__name__)

INSTALL_DIR = "/tmp/vf-ghosty"
DEFAULT_BINARY = f"{INSTALL_DIR}/bin/ghosty"
RELEASE_ROOT = "https://github.com/blissito/ghostycode/releases/download"
STREAM_SCHEMA = "ghosty.exec-stream"
STREAM_SCHEMA_VERSION = 1
MAX_TERMINAL_RECEIPT_BYTES = 8_192
MAX_TERMINAL_STRING_CHARS = 512
_VERSION = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$")
_TOOL = re.compile(r"^[a-z0-9][a-z0-9_-]*$")
_SHA256 = re.compile(r"^sha256:[0-9a-f]{64}$")
_EVENT_TYPES = {
    "content",
    "tool_use",
    "tool_result",
    "sandbox_denied",
    "workflow_event",
    "session_capture",
    "turn_usage",
    "metadata",
    "done",
    "error",
}
_TERMINAL_FIELDS = {
    "receipt_kind",
    "provider",
    "provider_id",
    "model",
    "route_source",
    "input_tokens",
    "output_tokens",
    "prompt_cache_hit_tokens",
    "prompt_cache_miss_tokens",
    "prompt_cache_write_tokens",
    "reasoning_tokens",
    "duration_ms",
    "retry_count",
    "approval_posture",
    "sandbox_posture",
    "binary_sha256",
    "config_sha256",
    "prompt_sha256",
    "tool_catalog_sha256",
    "visible_final_answer_chars",
    "message_count",
    "status",
    "termination_reason",
    "error_category",
}


class GhostyHarnessConfig(HarnessConfig):
    version: str = "0.9.1"
    """Ghosty release to install, pinned for reproducible rollouts."""

    binary_path: str | None = None
    """Preinstalled facade inside the runtime; useful for local candidate testing."""

    max_turns: int | None = None
    """Optional model-step ceiling; omitted rollouts are unlimited."""

    sandbox: Literal["auto", "read-only", "workspace-write", "external-sandbox"] = (
        "auto"
    )
    """`auto` keeps subprocess runs workspace-bound and trusts isolated runtimes."""


class GhostyHarness(Harness[GhostyHarnessConfig]):
    APPENDS_SYSTEM_PROMPT = True
    SUPPORTS_MCP = True
    SUPPORTS_USER_SIM = False

    def _validate_config(self) -> None:
        if not _VERSION.fullmatch(self.config.version):
            raise ValueError("version must be a semantic release identifier")
        if (
            self.config.max_turns is not None
            and not 1 <= self.config.max_turns <= 10_000
        ):
            raise ValueError("max_turns must be between 1 and 10000")
        invalid = [
            tool
            for tool in self.config.disabled_tools or []
            if not _TOOL.fullmatch(tool)
        ]
        if invalid:
            raise ValueError(
                "disabled_tools must use Ghosty catalog identifiers: "
                + ", ".join(repr(tool) for tool in invalid)
            )

    @property
    def binary(self) -> str:
        configured = (self.config.binary_path or "").strip()
        return configured or DEFAULT_BINARY

    async def setup(self, runtime: Runtime) -> None:
        self._validate_config()
        if self.config.binary_path:
            logger.info("ghosty: verifying preinstalled %s", self.binary)
            result = await runtime.run([self.binary, "--version"], {})
            version_text = f"{result.stdout}\n{result.stderr}"
            if result.exit_code != 0 or not _has_version(
                version_text, self.config.version
            ):
                raise RuntimeError(
                    "configured Ghosty binary is unavailable or does not report "
                    f"version {self.config.version}"
                )
            return

        logger.info("ghosty: ensuring Ghosty %s is installed", self.config.version)
        script = _install_script(self.config.version)
        result = await runtime.run(["sh", "-c", script], {})
        if result.exit_code != 0:
            raise RuntimeError(
                "Ghosty install failed: "
                + (result.stderr or result.stdout).strip()[-500:]
            )

    async def launch(
        self,
        ctx: ModelContext,
        trace: Trace,
        runtime: Runtime,
        endpoint: str,
        secret: str,
        mcp_urls: dict[str, str],
    ) -> ProgramResult:
        self._validate_config()
        system, prompt = self.resolve_prompt(trace.task.data)
        trace_key = hashlib.sha256(str(trace.id).encode()).hexdigest()[:32]
        home = f".vf-ghosty/{trace_key}"
        mcp_path = f"{home}/mcp.json"
        mcp = {"servers": {name: {"url": url} for name, url in mcp_urls.items()}}
        await runtime.write(
            mcp_path,
            json.dumps(mcp, sort_keys=True, separators=(",", ":")).encode(),
        )

        endpoint = endpoint.rstrip("/")
        env = {
            **self.config.resolved_env,
            "GHOSTY_HOME": home,
            "GHOSTY_PROVIDER": "openai",
            "DEEPSEEK_PROVIDER": "openai",
            "GHOSTY_MODEL": ctx.model,
            "DEEPSEEK_MODEL": ctx.model,
            "OPENAI_MODEL": ctx.model,
            "OPENAI_BASE_URL": endpoint,
            "OPENAI_API_KEY": secret,
            "GHOSTY_MCP_CONFIG": mcp_path,
            "DEEPSEEK_MCP_CONFIG": mcp_path,
            "GHOSTY_TELEMETRY": "false",
            "DEEPSEEK_TELEMETRY": "false",
            "GHOSTY_MEMORY": "false",
            "NO_COLOR": "1",
        }
        if endpoint.startswith("http://") or any(
            url.startswith("http://") for url in mcp_urls.values()
        ):
            # Verifiers' interception and colocated MCP endpoints are often
            # ephemeral HTTP services inside an already-isolated runtime.
            env["GHOSTY_ALLOW_INSECURE_HTTP"] = "1"

        sandbox = self.config.sandbox
        if sandbox == "auto":
            sandbox = (
                "workspace-write"
                if runtime.type == "subprocess"
                else "external-sandbox"
            )
        argv = [
            self.binary,
            "--provider",
            "openai",
            "--model",
            ctx.model,
            "--telemetry",
            "false",
            "--workspace",
            ".",
            "--skip-onboarding",
            "--no-project-config",
            "exec",
            "--auto",
            "--sandbox",
            sandbox,
            "--output-format",
            "stream-json",
        ]
        if self.config.max_turns is not None:
            argv.extend(["--max-turns", str(self.config.max_turns)])
        if self.config.disabled_tools:
            argv.extend(["--disallowed-tools", ",".join(self.config.disabled_tools)])
        if system:
            argv.extend(["--append-system-prompt", system])
        argv.extend(["--", str(prompt or "")])

        result = await runtime.run_program(argv, env)
        if result.exit_code == 0:
            receipt = _parse_stream_receipt(result.stdout)
            terminal = receipt["terminal"]
            if terminal.get("provider") != "openai":
                raise RuntimeError("Ghosty terminal receipt did not use provider openai")
            if terminal.get("model") != ctx.model:
                raise RuntimeError("Ghosty terminal receipt model did not match rollout")
            if terminal.get("approval_posture") != "auto_tools":
                raise RuntimeError("Ghosty terminal receipt did not confirm auto tools")
            if terminal.get("sandbox_posture") != sandbox:
                raise RuntimeError("Ghosty terminal receipt sandbox did not match launch")
            if receipt["events"].get("error", 0) != 0:
                raise RuntimeError("Ghosty successful run contained an error event")
            if terminal.get("status") != "completed":
                raise RuntimeError("Ghosty terminal receipt did not report completion")
            if terminal.get("termination_reason") != "resolved":
                raise RuntimeError("Ghosty terminal receipt was not resolved")
            trace.info["ghosty"] = receipt
        return result


def _has_version(output: str, version: str) -> bool:
    return (
        re.search(
            rf"(?<![0-9A-Za-z.+-]){re.escape(version)}(?![0-9A-Za-z.+-])",
            output,
        )
        is not None
    )


def _bounded_terminal(meta: dict[str, Any]) -> dict[str, Any]:
    terminal: dict[str, Any] = {}
    for key in _TERMINAL_FIELDS:
        if key not in meta or meta[key] is None:
            continue
        value = meta[key]
        if isinstance(value, str):
            if len(value) > MAX_TERMINAL_STRING_CHARS:
                raise RuntimeError("Ghosty terminal receipt exceeded its string bound")
        elif isinstance(value, int) and not isinstance(value, bool):
            if value < 0 or value > 2**63 - 1:
                raise RuntimeError("Ghosty terminal receipt contained an invalid count")
        else:
            raise RuntimeError("Ghosty terminal receipt contained a non-scalar field")
        terminal[key] = value
    encoded = json.dumps(terminal, sort_keys=True, separators=(",", ":")).encode()
    if len(encoded) > MAX_TERMINAL_RECEIPT_BYTES:
        raise RuntimeError("Ghosty terminal receipt exceeded its total bound")
    return terminal


def _install_script(version: str) -> str:
    version_q = shlex.quote(version)
    install_q = shlex.quote(INSTALL_DIR)
    release_q = shlex.quote(RELEASE_ROOT)
    return f"""
set -eu
version={version_q}
install_dir={install_q}
release_root={release_q}
if [ "$(uname -s)" != Linux ]; then
    echo "automatic Ghosty installation supports Linux runtimes; set binary_path" >&2
    exit 1
fi
case "$(uname -m)" in
    x86_64|amd64) platform=linux-x64 ;;
    aarch64|arm64) platform=linux-arm64 ;;
    *) echo "unsupported Ghosty runtime architecture: $(uname -m)" >&2; exit 1 ;;
esac
if ! command -v curl >/dev/null 2>&1 \
    || ! command -v sha256sum >/dev/null 2>&1 \
    || ! command -v flock >/dev/null 2>&1; then
    if command -v apt-get >/dev/null 2>&1; then
        apt-get update -qq
        apt-get install -y -qq curl ca-certificates coreutils util-linux >/dev/null
    elif command -v apk >/dev/null 2>&1; then
        apk add --no-cache curl ca-certificates coreutils util-linux >/dev/null
    else
        echo "Ghosty install needs curl, sha256sum, and flock" >&2
        exit 1
    fi
fi
mkdir -p "$install_dir"
exec 9>"$install_dir/install.lock"
flock 9
mkdir -p "$install_dir/bin"
if [ -f "$install_dir/bin/.version" ] \
    && [ "$(cat "$install_dir/bin/.version")" = "$version" ] \
    && (cd "$install_dir/bin" && sha256sum -c .sha256 >/dev/null 2>&1); then
    exit 0
fi
tmp="$(mktemp -d "$install_dir/install.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
base="$release_root/v$version"
curl -fsSL "$base/ghosty-artifacts-sha256.txt" -o "$tmp/manifest"
for pair in \
    "ghosty-$platform:ghosty" \
    "ghosty-tui-$platform:ghosty-tui" \
    "ghosty-tui-$platform:ghosty-tui"
do
    asset="${{pair%%:*}}"
    target="${{pair#*:}}"
    curl -fsSL "$base/$asset" -o "$tmp/$asset"
    expected="$(awk -v asset="$asset" '$2 == asset {{ print $1; exit }}' "$tmp/manifest")"
    actual="$(sha256sum "$tmp/$asset" | awk '{{print $1}}')"
    if [ -z "$expected" ] || [ "$actual" != "$expected" ]; then
        echo "Ghosty checksum verification failed for $asset" >&2
        exit 1
    fi
    cp "$tmp/$asset" "$install_dir/bin/$target.tmp.$$"
    chmod 0755 "$install_dir/bin/$target.tmp.$$"
    mv -f "$install_dir/bin/$target.tmp.$$" "$install_dir/bin/$target"
done
(cd "$install_dir/bin" && sha256sum ghosty ghosty-tui ghosty-tui > .sha256.tmp)
mv -f "$install_dir/bin/.sha256.tmp" "$install_dir/bin/.sha256"
printf '%s' "$version" > "$install_dir/bin/.version.tmp"
mv -f "$install_dir/bin/.version.tmp" "$install_dir/bin/.version"
"""


def _parse_stream_receipt(stdout: str) -> dict[str, Any]:
    counts: Counter[str] = Counter()
    terminal: dict[str, Any] | None = None
    ordered_types: list[str] = []
    for line_number, line in enumerate(stdout.splitlines(), start=1):
        if not line.strip():
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError as error:
            raise RuntimeError(
                f"Ghosty stream-json line {line_number} was not valid JSON"
            ) from error
        if not isinstance(event, dict):
            raise RuntimeError(
                f"Ghosty stream-json line {line_number} was not an object"
            )
        if event.get("schema") != STREAM_SCHEMA or event.get(
            "schema_version"
        ) != STREAM_SCHEMA_VERSION:
            raise RuntimeError("Ghosty stream-json schema did not match v0.9.1")
        event_type = event.get("type")
        if event_type not in _EVENT_TYPES:
            raise RuntimeError("Ghosty stream-json contained an unknown event type")
        counts[event_type] += 1
        ordered_types.append(event_type)
        if event_type == "metadata":
            if terminal is not None:
                raise RuntimeError("Ghosty emitted more than one terminal metadata receipt")
            meta = event.get("meta")
            if not isinstance(meta, dict) or meta.get("receipt_kind") != "terminal":
                raise RuntimeError("Ghosty metadata event was not a terminal receipt")
            terminal = _bounded_terminal(meta)

    if terminal is None:
        raise RuntimeError("Ghosty stream-json omitted terminal metadata")
    if counts["done"] != 1 or not ordered_types or ordered_types[-1] != "done":
        raise RuntimeError("Ghosty stream-json did not end with exactly one done event")
    if ordered_types[-2:-1] != ["metadata"]:
        raise RuntimeError("Ghosty terminal metadata did not immediately precede done")
    for field in ["binary_sha256", "prompt_sha256"]:
        if not isinstance(terminal.get(field), str) or not _SHA256.fullmatch(
            terminal[field]
        ):
            raise RuntimeError(f"Ghosty terminal receipt omitted a valid {field}")
    return {
        "schema": STREAM_SCHEMA,
        "schema_version": STREAM_SCHEMA_VERSION,
        "events": dict(sorted(counts.items())),
        "terminal": terminal,
    }
