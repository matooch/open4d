#!/bin/sh
# Overnight implementer loop (docs/agents/HERMES-IMPLEMENTER.md §2b), run on the owner's Mac.
#
# Starts a fresh OpenCode session with the implementer prompt, waits for it to end, and starts the
# next one, so a long queue survives the context limit of a single session. Each session resumes
# from state.json. The loop stops when:
#   - a session writes "QUEUE FINISHED" or "QUEUE PAUSED" to log.md,
#   - two sessions in a row change nothing (no new commit on any hermes/* branch on origin, and
#     state.json unchanged), so a stuck agent cannot burn tokens all night,
#   - MAX_SESSIONS is reached, or a file named STOP appears in the workspace (touch it to stop
#     after the current session).
#
# Usage:
#   sh scripts/hermes-loop.sh /path/to/implementer-prompt.txt
# Optional environment:
#   OPENCODE_MODEL   model for `opencode run -m` (default: OpenCode's configured model)
#   MAX_SESSIONS     default 12
#   REPO             default /Users/bigman/Github/open4d-impl
#   WORKSPACE        default /Users/bigman/forge-implementer
set -u

PROMPT_FILE=${1:?usage: hermes-loop.sh /path/to/implementer-prompt.txt}
REPO=${REPO:-/Users/bigman/Github/open4d-impl}
WORKSPACE=${WORKSPACE:-/Users/bigman/forge-implementer}
MAX_SESSIONS=${MAX_SESSIONS:-12}
LOG="$WORKSPACE/log.md"
LOOPLOG="$WORKSPACE/loop.log"

[ -f "$PROMPT_FILE" ] || { echo "no prompt file: $PROMPT_FILE"; exit 1; }
command -v opencode >/dev/null 2>&1 || { echo "opencode is not on PATH"; exit 1; }
mkdir -p "$WORKSPACE"
touch "$LOG"
rm -f "$WORKSPACE/STOP"

fingerprint() {
  # Remote hermes/* heads plus state.json: changes whenever a session pushed or advanced a step.
  git -C "$REPO" fetch -q origin 2>/dev/null
  {
    git -C "$REPO" for-each-ref --format='%(refname) %(objectname)' 'refs/remotes/origin/hermes/*'
    cat "$WORKSPACE/state.json" 2>/dev/null
  } | shasum | cut -d' ' -f1
}

say() { echo "$(date '+%Y-%m-%d %H:%M:%S') $*" | tee -a "$LOOPLOG"; }

idle=0
n=0
while [ "$n" -lt "$MAX_SESSIONS" ]; do
  n=$((n + 1))
  before=$(fingerprint)
  lines_before=$(wc -l < "$LOG")
  say "session $n start"
  cd "$REPO" || exit 1
  if [ -n "${OPENCODE_MODEL:-}" ]; then
    opencode run -m "$OPENCODE_MODEL" "$(cat "$PROMPT_FILE")" >> "$WORKSPACE/session-$n.out" 2>&1
  else
    opencode run "$(cat "$PROMPT_FILE")" >> "$WORKSPACE/session-$n.out" 2>&1
  fi
  say "session $n ended (exit $?)"

  # Only lines written during this session count (older runs left "QUEUE FINISHED" lines).
  new_lines=$(tail -n +"$((lines_before + 1))" "$LOG")
  if printf '%s\n' "$new_lines" | grep -q 'QUEUE FINISHED'; then
    say "queue finished"; exit 0
  fi
  if printf '%s\n' "$new_lines" | grep -q 'QUEUE PAUSED'; then
    say "queue paused: $(printf '%s\n' "$new_lines" | grep 'QUEUE PAUSED' | tail -1)"; exit 0
  fi
  if [ -f "$WORKSPACE/STOP" ]; then
    say "STOP file found"; exit 0
  fi

  after=$(fingerprint)
  if [ "$before" = "$after" ]; then
    idle=$((idle + 1))
    say "no progress in session $n ($idle in a row)"
    [ "$idle" -ge 2 ] && { say "stopping: two sessions without progress"; exit 1; }
  else
    idle=0
  fi
  sleep 30
done
say "stopping: MAX_SESSIONS=$MAX_SESSIONS reached"
