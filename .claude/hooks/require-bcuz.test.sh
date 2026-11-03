#!/usr/bin/env bash
# Checks .claude/hooks/require-bcuz.sh: run it with `bash .claude/hooks/require-bcuz.test.sh`.
# Uses a fake bcuz on PATH so that it works before bcuz is installed.
# The single-quoted $( and backtick in the cases below must not expand.
# shellcheck disable=SC2016
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
hook="$here/require-bcuz.sh"
bin=$(mktemp -d)
trap 'rm -rf "$bin"' EXIT
printf '#!/bin/sh\nexit 0\n' >"$bin/bcuz"
chmod +x "$bin/bcuz"
export PATH="$bin:$PATH"

fail=0
# expect <allow|deny> <command>
expect() {
  local want=$1 cmd=$2 got=allow
  if ! jq -n --arg c "$cmd" '{tool_input: {command: $c}}' | bash "$hook" >/dev/null 2>&1; then
    got=deny
  fi
  if [ "$got" != "$want" ]; then
    echo "FAIL: want $want, got $got: $cmd" >&2
    fail=1
  fi
}

# allowed
expect allow 'bcuz "lint" qsoku lint'
expect allow '  bcuz "lint" qsoku lint'
expect allow "bcuz 'a && b; c' echo hi"
expect allow 'bcuz "x" qsoku check 2>&1 | tail -20'
expect allow 'bcuz "x" make test > out.txt 2>&1'
expect allow 'bcuz "x" sh -c "cd sub && make"'
expect allow 'bcuz "x" env VERSION=v0.1.0 qsoku dist'
expect allow 'bcuz -- "-x reason" ls'
expect allow 'bcuz -H'
expect allow 'bcuz -H --date 2026-10-14'
expect allow 'bcuz --init'
expect allow 'bcuz --version'
expect allow 'bcuz -h'
expect allow 'bcuz "日本語の理由" git status'
expect allow 'bcuz "grep" grep -rn "a;b" .'
expect allow 'git commit -m "fix: a; b && c"'
expect allow 'git commit'
expect allow $'git commit -m "title\n\nbody"'
expect allow '  git commit -F msg.txt'
expect allow 'bcuz "git commit の前の確認" git status'
expect allow 'bcuz "x" git commit-tree x'
expect allow "bcuz \"x\" sh -c 'echo \$(date)'"
expect allow "bcuz \"x\" sh -c 'echo \`date\`'"
expect allow "bcuz \"x\" grep -n '\$(' file"
expect allow 'bcuz "x" echo \$(date)'

# refused
expect deny 'qsoku check'
expect deny 'cd x && bcuz "y" ls'
expect deny 'bcuz "a" ls && rm -rf x'
expect deny 'bcuz "a" ls; rm x'
expect deny 'bcuz "a" ls || echo fail'
expect deny 'bcuz "a" sleep 10 &'
expect deny $'bcuz "a" ls\nrm x'
expect deny 'bcuz "a" echo $(whoami)'
expect deny 'bcuz "a $(whoami)" ls'
expect deny 'bcuz "a" echo `whoami`'
expect deny 'bcuz "a" sh -c "echo $(whoami)"'
expect deny "bcuz \"a\" sh -c \"echo 'x' \$(whoami)\""
expect deny 'bcuz "a" diff <(ls a) <(ls b)'
expect deny 'bcuz -x ls'
expect deny 'FOO=1 bcuz "a" ls'
expect deny 'bcuzz "a" ls'
expect deny 'echo bcuz'
expect deny 'bcuz "commit" git commit -m x'
expect deny 'bcuz -- "-x" git commit -m x'
expect deny 'git commit -m x && git push'
expect deny 'git commit -m "$(cat msg)"'
expect deny 'git push'
expect deny 'git commit-tree x'

if [ "$fail" -ne 0 ]; then
  exit 1
fi
echo "require-bcuz: ok"
