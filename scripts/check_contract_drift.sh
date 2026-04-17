#!/usr/bin/env bash
#
# Contract drift guard.
#
# Catches documents / rules / prompt references that have regressed to the
# legacy `reply(to, text, status)` signature or the old `text` envelope
# field. The canonical wire contract (as of the agent_id-aware routing
# unification) is `reply(target, message, status)` with target shaped as
# `{"kind":"user|role|agent", "role":"...", "agentId":"..."}` — all three
# target fields required, unused ones filled with "".
#
# Run manually: ./scripts/check_contract_drift.sh
# Hook into CI (future .github/workflows/*.yml) as a required check.
#
# Exits 0 if no drift, 1 if any legacy patterns found.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

errors=0

# Active doc roots only — historical plans may legitimately reference the
# legacy signature for context (they're frozen archives, not live contracts).
ACTIVE_DOCS=(
  .claude/agents
  .claude/rules
  .claude/skills
  docs/agents
)

# ── 1. Legacy `reply(to=...)` in live docs / rules / agent prompts ─
if grep -rn 'reply(to=' "${ACTIVE_DOCS[@]}" 2>/dev/null; then
  echo "ERROR: legacy 'reply(to=...)' signature found in active docs."
  echo "       reply tool now requires structured \`target\` object."
  echo "       Use reply(target={kind, role, agentId}, message, status) instead."
  errors=$((errors + 1))
fi

# ── 2. Legacy `reply(target, text, status)` signature (text → message) ──
if grep -rn 'reply(target, text, status)' \
    "${ACTIVE_DOCS[@]}" src/ src-tauri/ bridge/ 2>/dev/null; then
  echo "ERROR: legacy 'reply(target, text, status)' found — envelope field"
  echo "       was renamed to \`message\` for canonical alignment with Codex"
  echo "       output_schema. Replace 'text' with 'message' in prompts/docs."
  errors=$((errors + 1))
fi

# ── 3. `to="..."` as kwarg in active docs ─────────────────────────
if grep -rn 'reply([^)]*\bto="' "${ACTIVE_DOCS[@]}" 2>/dev/null; then
  echo "ERROR: 'reply(to=\"...\")' keyword-argument form found in active docs."
  echo "       Canonical form is reply(target={kind, role, agentId}, ...)."
  errors=$((errors + 1))
fi

if [ "$errors" -gt 0 ]; then
  echo ""
  echo "$errors drift pattern(s) detected. Fix the above references."
  exit 1
fi

echo "OK: no legacy reply/envelope drift detected."
