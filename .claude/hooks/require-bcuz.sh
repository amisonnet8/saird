#!/usr/bin/env bash
# PreToolUse hook for the Bash tool: every command must be run through bcuz
# (`bcuz "<reason>" <command> [args...]`), so that when / why / what / result
# land in .bcuz/history.jsonl (.claude/rules/bcuz-usage.md).
#
# Allowed:  bcuz "<reason>" cmd ...   bcuz -- "-x reason" cmd ...
#           bcuz -H [...]  bcuz --init  bcuz -h  bcuz --version
#           git commit ...  (the one command run without bcuz: bcuz writes its
#           record after the command ends, so a commit through bcuz would leave
#           .bcuz/history.jsonl changed right after every commit. The reason
#           goes in the commit message.)
# Refused:  anything that does not start with bcuz, and anything that chains
#           more commands behind the first one (&&  ||  ;  &  newline, command
#           substitution outside single quotes), because those would run without
#           a record.
#           bcuz "<reason>" git commit ...: run git commit without bcuz.
# Pipes and redirects are allowed (output shaping: `bcuz "x" qsoku check 2>&1 | tail`).
# Not installed yet (bootstrap, e.g. the first `go install`): allowed.
#
# This is a guard against forgetting, not a security boundary: `bcuz "r" sh -c
# '...'` is allowed on purpose (one logical step, one record).
# The single quotes below are intentional: they are literal characters to match.
# shellcheck disable=SC1003,SC2016
set -euo pipefail

# Remove quoted text, so that operators inside quotes are not mistaken for
# shell syntax. Backslash escapes outside quotes drop the escaped character.
strip_quotes() {
  local s=$1 out="" q="" c i
  for ((i = 0; i < ${#s}; i++)); do
    c=${s:i:1}
    if [ -n "$q" ]; then
      if [ "$q" = '"' ] && [ "$c" = '\' ]; then
        i=$((i + 1))
        continue
      fi
      if [ "$c" = "$q" ]; then q=""; fi
      continue
    fi
    case $c in
      "'" | '"') q=$c ;;
      '\') i=$((i + 1)) ;;
      *) out+=$c ;;
    esac
  done
  printf '%s' "$out"
}

# Prints the reason to refuse, or nothing if the command is fine.
check() {
  local cmd=$1 bare
  cmd="${cmd#"${cmd%%[![:space:]]*}"}" # trim leading whitespace

  if [[ $cmd =~ ^git[[:space:]]+commit([[:space:]]|$) ]]; then
    check_single "$cmd"
    return
  fi
  if [[ ! $cmd =~ ^bcuz([[:space:]]|$) ]]; then
    echo "does not start with bcuz"
    return
  fi
  # The first argument is read as an option only by bcuz itself; here we only
  # let through its own options, a reason, or "--".
  if [[ $cmd =~ ^bcuz[[:space:]]+- ]] &&
    ! [[ $cmd =~ ^bcuz[[:space:]]+(-H|--history|--init|-h|--help|--version|--)([[:space:]]|$) ]]; then
    echo "unknown bcuz option (put '--' before a reason that starts with '-')"
    return
  fi
  bare=$(strip_quotes "$cmd")
  if [[ $bare =~ ^bcuz([[:space:]]+--)?[[:space:]]+([^[:space:]]+[[:space:]]+)?git[[:space:]]+commit([[:space:]]|$) ]]; then
    echo "git commit is run without bcuz (its record would be written after the commit): git commit ..."
    return
  fi
  check_single "$cmd"
}

# Remove only single-quoted text: nothing inside single quotes is expanded, so
# `bcuz "r" sh -c 'echo $(date)'` runs the substitution inside the recorded
# step. Text in double quotes is kept ($(...) there is expanded by the shell
# that runs bcuz). Backslash escapes drop the escaped character (\$( is literal).
strip_single_quotes() {
  local s=$1 out="" q="" c i
  for ((i = 0; i < ${#s}; i++)); do
    c=${s:i:1}
    if [ "$q" = "'" ]; then
      if [ "$c" = "'" ]; then q=""; fi
      continue
    fi
    if [ "$q" = '"' ]; then
      case $c in
        '\') i=$((i + 1)) ;;
        '"') q="" ;;
        *) out+=$c ;;
      esac
      continue
    fi
    case $c in
      "'" | '"') q=$c ;;
      '\') i=$((i + 1)) ;;
      *) out+=$c ;;
    esac
  done
  printf '%s' "$out"
}

# Prints the reason to refuse a command that is not a single command, or nothing.
check_single() {
  local cmd=$1 bare unquoted
  unquoted=$(strip_single_quotes "$cmd")
  if [[ $unquoted == *'$('* || $unquoted == *'`'* || $unquoted == *'<('* || $unquoted == *'>('* ]]; then
    echo "contains command or process substitution (it would run without a record)"
    return
  fi

  bare=$(strip_quotes "$cmd")
  # 2>&1, >&2, &> are redirects, not control operators.
  bare=$(sed -E 's/[0-9]*>&[0-9-]*//g; s/&>>?//g' <<<"$bare")
  if [[ $bare == *$'\n'* ]]; then
    echo "contains a newline (one command per call; put a script in a file)"
    return
  fi
  if [[ $bare == *'&'* || $bare == *';'* || $bare == *'||'* ]]; then
    echo "chains commands (&& || ; &): run each one with its own bcuz, or wrap one step in: bcuz \"reason\" sh -c '...'"
    return
  fi
}

input=$(cat)
command=$(jq -r '.tool_input.command // empty' <<<"$input")
if [ -z "$command" ]; then
  exit 0
fi
if ! command -v bcuz >/dev/null 2>&1; then
  exit 0
fi

reason=$(check "$command")
if [ -z "$reason" ]; then
  exit 0
fi

cat >&2 <<EOF
Refused: this Bash command is not run through bcuz ($reason).
Run it as:  bcuz "<why you are running it>" <command> [args...]
Example:    bcuz "lint after the refactor" qsoku lint
Only git commit is run without bcuz: git commit -m "..."
See .claude/rules/bcuz-usage.md.
EOF
exit 2
