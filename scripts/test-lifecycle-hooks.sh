#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
hook_cli=${VIBEBUS_HOOK_CLI:-"$repo_root/target/debug/vibebus"}

if [ ! -x "$hook_cli" ]; then
    cargo build --locked
fi

test_root=$(mktemp -d "${TMPDIR:-/tmp}/vibebus-hooks.XXXXXX")
plugin_data="$test_root/plugin-data"
mkdir -p "$plugin_data"
trap 'rm -rf -- "$test_root"' EXIT HUP INT TERM

export PLUGIN_ROOT="$repo_root/plugins/vibebus"
export PLUGIN_DATA="$plugin_data"
export VIBEBUS_HOOK_DRY_RUN=1
export VIBEBUS_HOOK_TEST_BINDING='{"agent":"hook-test-agent","taskId":"HOOK-TEST-001","threadId":"hook-session","unboundAt":null}'
export VIBEBUS_HOOK_TEST_GIT='{"commitSha":"0123456789abcdef0123456789abcdef01234567","summary":"Hook fixture","changedPaths":["src/lib.rs","tests/core_workflows.rs"]}'
export VIBEBUS_HOOK_TEST_PROPOSAL='{"taskId":"HOOK-TEST-001","status":"working","gitCommits":[],"testResults":[],"artifacts":[],"decisions":[],"nextActions":["Review and send explicitly"]}'

invoke_hook() {
    hook_name=$1
    input=$2
    printf '%s' "$input" | "$hook_cli" hook "$hook_name"
}

commit_result=$(invoke_hook post-tool-use "{\"session_id\":\"hook-session\",\"cwd\":\"$repo_root\",\"hook_event_name\":\"PostToolUse\",\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"git commit -m fixture\"},\"tool_response\":{\"exit_code\":0,\"output\":\"content is deliberately ignored\"}}")
case "$commit_result" in
    *"changed paths only"*) ;;
    *) echo "Git hook did not report its bounded path-only behavior." >&2; exit 1 ;;
esac

test_result=$(invoke_hook post-tool-use "{\"session_id\":\"hook-session\",\"cwd\":\"$repo_root\",\"hook_event_name\":\"PostToolUse\",\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"cargo test --all-targets --locked\"},\"tool_response\":{\"exitCode\":0,\"output\":\"test logs are deliberately ignored\"}}")
case "$test_result" in
    *"command output was not stored"*) ;;
    *) echo "Test hook did not report its bounded no-log behavior." >&2; exit 1 ;;
esac

unknown_result=$(invoke_hook post-tool-use "{\"session_id\":\"hook-session\",\"cwd\":\"$repo_root\",\"hook_event_name\":\"PostToolUse\",\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"cargo test\"},\"tool_response\":{\"output\":\"no exit metadata\"}}")
case "$unknown_result" in
    *"reliable exit code"*) ;;
    *) echo "Test hook must skip unknown outcomes instead of guessing." >&2; exit 1 ;;
esac

stop_result=$(invoke_hook stop "{\"session_id\":\"hook-session\",\"cwd\":\"$repo_root\",\"hook_event_name\":\"Stop\",\"stop_hook_active\":false,\"last_assistant_message\":\"This field is deliberately ignored.\"}")
case "$stop_result" in
    *"no handoff was sent automatically"*) ;;
    *) echo "Stop hook did not preserve the explicit-send boundary." >&2; exit 1 ;;
esac

proposal_count=$(find "$plugin_data/handoff-proposals" -type f -name '*.json' | wc -l | tr -d ' ')
if [ "$proposal_count" -ne 1 ]; then
    echo "Expected one bounded proposal file, found $proposal_count." >&2
    exit 1
fi
proposal_file=$(find "$plugin_data/handoff-proposals" -type f -name '*.json' -print)
if ! grep -q '"taskId": "HOOK-TEST-001"' "$proposal_file"; then
    echo "Stop hook proposal was not task scoped." >&2
    exit 1
fi

hooks_file="$repo_root/plugins/vibebus/hooks/hooks.json"
if ! grep -q 'hook session-start' "$hooks_file" ||
   ! grep -q 'hook post-tool-use' "$hooks_file" ||
   ! grep -q 'hook stop' "$hooks_file"; then
    echo "Plugin lifecycle hook configuration is incomplete." >&2
    exit 1
fi

printf '%s\n' '{"ok":true,"checks":7,"failures":0,"skipped":0,"platform":"macos"}'
