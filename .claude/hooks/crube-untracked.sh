#!/usr/bin/env bash
# After srwr's new creates a file, tell Claude which paths of it still have no
# description, in each language of this repository's .crube/ (English and
# Japanese; .claude/rules/crube-usage.md). Only the new file and the
# directories above it are reported, so that a file made in a new directory
# also asks for that directory. It never blocks: the step before a commit
# (.claude/rules/crube-usage.md) is where status --require all has to pass.
set -euo pipefail

file=$(jq -r '.tool_input.file // empty')
if [ -z "$file" ]; then
  exit 0
fi
cd "$CLAUDE_PROJECT_DIR"
file=${file#"$CLAUDE_PROJECT_DIR"/}

report=""
for lang in en ja; do
  # No crube on PATH or a broken .crube/ is not this hook's business: the
  # step before a commit reports those.
  json=$(env CRUBE_LANG="$lang" crube status --json 2>/dev/null) || continue
  paths=$(printf '%s' "$json" | jq -r --arg f "$file" '
    .items[] | select(.kind == "untracked") | .path as $p
    | select($p == $f or ($p | endswith("/")) and ($f | startswith($p))) | $p')
  if [ -n "$paths" ]; then
    report+="CRUBE_LANG=$lang: $(printf '%s' "$paths" | paste -sd ' ')"$'\n'
  fi
done

if [ -n "$report" ]; then
  jq -n --arg r "$report" '{hookSpecificOutput: {hookEventName: "PostToolUse",
    additionalContext: ("crube: no description yet (set it in both languages, .claude/rules/crube-usage.md):\n" + $r)}}'
fi
