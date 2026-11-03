# mtqg journal format

This document specifies how mtqg stores its records. It is the source of the
`.mtqg/SCHEMA.md` file that `mtqg init` writes into every project, so that any
person or AI can read the records without the mtqg command.

A Japanese translation is in `schema_ja.md` in the mtqg repository.

**Format version: 1.** The compatibility rules in [Versioning](#versioning)
apply: a field can be added if old readers can ignore it safely, but a field's
meaning is never changed and existing lines are never rewritten.

## What mtqg records

mtqg keeps the part of a project's process that does not survive in the code:
things noticed while working, things to do, questions and their answers, agreed
terms, and standing conventions. There are six kinds of record:

| Kind | Meaning | State |
|---|---|---|
| `memo` | Free-form note. Decisions, findings, reasons, hand-overs | none |
| `todo` | Something to do | `open` / `done` |
| `qa` | A question, or an answer to a question | questions: `open` / `done`; answers: none |
| `bug` | A bug report, and the exchange about it (a reply to it) | bugs: `open` / `done`; replies: none |
| `glossary` | A term (`word`) and its definition (`text`) | none |
| `rule` | A convention to follow, in force until it is edited or deleted | none |

`qa` and `bug` have the same shape: a record that can be replied to, and its
replies. They differ in what they are about. A bug is `done` when it is fixed or
no longer pursued.

Versioning is left to git. mtqg only **appends** lines; changes, answers,
corrections and deletions are all expressed as new lines.

## Files

```
.mtqg/
  .gitattributes     *.jsonl text eol=lf merge=union
  .gitignore         .local/
  version            format version of this directory (one integer and a newline)
  SCHEMA.md          this document
  journal.jsonl      all current events
  archive/           events moved out of view by `mtqg archive`
    2021-01-01..2024-09-18.jsonl
  .local/            machine-local temporary files (never committed)
```

- All files except `.local/` are committed to git like ordinary files.
- `journal.jsonl` holds every event that is in normal view.
- `archive/<start>..<end>.jsonl` holds events of items that were archived for
  that date range (start and end inclusive, local dates, always written as
  `YYYY-MM-DD`). mtqg commands do not read `archive/`.
- `.local/` holds things that only matter on this machine: the write lock,
  temporary files used while rewriting, an agent hook's session bookkeeping
  (`sessions/`, see cli.md's "Agent hooks"), and similar. It is ignored by git
  (`.mtqg/.gitignore`), holds nothing that other readers need, and can be
  deleted at any time when mtqg is not running.

## Line format

- JSON Lines: **one event per line**, each line a JSON object.
- UTF-8 without BOM. Line endings are LF. **Every line, including the last,
  ends with LF.**
- Newlines inside text are escaped as `\n`; an event never spans lines.

Example:

```jsonl
{"id":"6b0d549b6f03475a8600a35a099950d8","op":"create","type":"todo","status":"open","text":"Support C syntax","v":1,"ts":"2026-09-17T00:00:00Z","author":{"kind":"human","name":"yamada"}}
{"id":"1012f037b64c44228c38fb2918f135d2","op":"create","type":"qa","status":"open","text":"Should nested block comments be supported?","v":1,"ts":"2026-09-17T00:10:00Z","author":{"kind":"ai","name":"claude-code"}}
{"id":"95e761d177314f10b06bf2efc6f87718","op":"create","type":"qa","re":"1012f037b64c44228c38fb2918f135d2","text":"Not in the first version. Revisit if there is demand","v":1,"ts":"2026-09-17T00:41:00Z","author":{"kind":"human","name":"yamada"},"tty":"3e9a0b12"}
{"id":"7f3a2b1c09d84e6fa5b17c2d3e4f5a60","op":"create","type":"bug","status":"open","text":"Parser crashes on empty input","v":1,"ts":"2026-09-17T00:50:00Z","author":{"kind":"human","name":"yamada"}}
{"id":"f28c105d1fb14c2390c192cfd3ac94af","op":"create","type":"glossary","word":"token","text":"The smallest unit produced by lexing","v":1,"ts":"2026-09-17T01:00:00Z","author":{"kind":"human","name":"yamada"}}
{"id":"3d8e4a0b12c94f77b6a08d1e5f2c9b34","op":"create","type":"bug","re":"7f3a2b1c09d84e6fa5b17c2d3e4f5a60","text":"Reproduced on macOS too. The empty file has no first token","v":1,"ts":"2026-09-17T01:20:00Z","author":{"kind":"ai","name":"claude-code"}}
{"id":"6b0d549b6f03475a8600a35a099950d8","op":"status","from":"open","status":"done","v":1,"ts":"2026-09-17T01:30:00Z","author":{"kind":"ai","name":"claude-code"}}
```

## Fields

| Field | Type | Present in | Meaning |
|---|---|---|---|
| `id` | string | all | ID of the record the event is about (see [IDs](#ids)) |
| `op` | string | all | `create`, `status`, `edit` or `delete` |
| `type` | string | `create` | `memo`, `todo`, `qa`, `bug`, `glossary` or `rule` |
| `re` | string | `create` of an answer or a reply | ID of the record this one replies to. It has the same `type` as this one |
| `from` | string | `status` | state before the change, as the writer saw it |
| `status` | string | `create` of todo / question / bug, `status` | state after the event (`open` or `done`) |
| `basis` | integer | `status`, `edit` (optional) | how many events the record had (including its `create`), as the writer saw it before writing this one. Used to tell a change made without knowing of another change to the same record (see below) |
| `word` | string | `create` of glossary | the term |
| `text` | string | `create`, `edit` | body text. For glossary, the definition |
| `v` | integer | all | format version the line was written in |
| `ts` | string | all | time of the event, UTC, RFC 3339 with `Z` (`2026-09-17T01:32:00Z`) |
| `author` | object | all | who is responsible for the content: `{"kind": "human" \| "ai", "name": string}` |
| `tty` | string | optional | short hash identifying the terminal that wrote the line |

Fields without a value are **omitted**, never written as `null`.

`author.kind` is the kind of the party responsible for the content. When an AI
writes down a human's decision, the author is the human (and the text says an
AI wrote it on their behalf).

`author.name` comes from wherever the record was written: the CLI takes it
from an environment variable or from git, and the MCP server (see `cli.md`)
takes it from the name the connecting client gives at startup.

## Operations

| `op` | Meaning | Fields |
|---|---|---|
| `create` | a new record | `type`, `text`; `word` for glossary; `status:"open"` for todo, questions and bugs; `re` for answers and replies |
| `status` | state change of a todo, a question or a bug | `from`, `status`, `basis` |
| `edit` | replace the body text | `text`, `basis` |
| `delete` | hide the record | none |

- A `qa` record with `re` is an **answer**; without `re` it is a **question**.
  A `bug` record with `re` is a **reply**; without `re` it is a **bug**.
  Answers and replies cannot be replied to. Only answers and replies have `re`.
- **A reply has the same `type` as the record it replies to**: `re` of a `qa`
  record is the ID of a question, and `re` of a `bug` record is the ID of a
  bug. A record whose `re` names a record of another `type` is not a reply to
  it.
- `edit` replaces `text` only. A glossary `word` cannot be changed.
- `delete` hides the record from normal view. Deleting a question or a bug also
  hides its answers or replies. The lines remain in the file and in git history.

## IDs

- A record ID is a random UUID (version 4) written as **32 lowercase hex digits
  without hyphens**, e.g. `81e74ef5e8e24d949ed904759531985d`.
- The ID is chosen when the record is created (`op:"create"`); every later event
  about that record carries the same `id`.
- IDs need no coordination between clones, branches or machines.
- Tools usually show only the first 10 digits (`81e74ef5e8`) and accept any
  unique prefix as input. Stored data always uses the full ID, so references
  (`re`) stay exact even if a short prefix later becomes ambiguous.

## Reading: building the current state

1. Read `journal.jsonl` line by line. Blank lines are ignored. A line is
   skipped with a warning if it is not a valid JSON object, contains invalid
   UTF-8, or has no `id` or no `op`.
2. Treat lines whose content is exactly identical as one event.
3. Order events by `ts`, then by `id` for equal `ts`. **Do not rely on the
   order of lines in the file.** Clock skew between machines can reorder
   events; that is accepted.
4. Apply events per `id` in that order: `create` starts a record, `status` sets
   its state, `edit` replaces its text, `delete` hides it. An event whose time
   is earlier than the `create` of its record (the clock of the writer runs
   behind) still counts: it is applied after the `create`, in order with the
   other changes. An event for an `id` that has no `create` is ignored.
5. Ignore fields you do not know.

Repeated events are not errors. If the same state change appears more than
once, the result is the same.

**Concurrent changes are facts, not errors.** If two `status` events for one
record have the same `from` but were written independently (for example in two
branches), both are kept and shown as they are; mtqg does not decide which is
right. Likewise, two glossary records with the same `word` are both kept and
shown side by side. `basis` (below) catches this the same way for a `status`
event or an `edit`, whether or not the two events changed the same field.

## Merging

`.mtqg/.gitattributes` declares `merge=union` for `*.jsonl`. When two branches
both append lines, git keeps the lines of both sides. Because events are only
appended and readers do not depend on line order, keeping both sides is always
correct.

### Resolving a conflict in journal.jsonl

Some merges do not use `.gitattributes` merge drivers (for example merging in a
web UI). Then `journal.jsonl` can get ordinary conflict markers. To resolve:

- **Keep every line from both sides.** Remove only the marker lines
  (`<<<<<<<`, `=======`, `>>>>>>>`).
- **Never choose one side.** Choosing one side deletes records.
- The order of the lines does not matter.

While conflict markers remain, mtqg warns when reading and refuses to write.

### Lines that come back

`mtqg undo` removes a line, and `mtqg archive` moves lines to `archive/`. If
those lines had already been merged into another branch, a later union merge
can bring them back into `journal.jsonl`. Identical lines are one event
(see above); archived items that come back are simply in view again and can be
archived again. They also come back when the other branch changed the lines
next to them, as when two branches archive lines that are neighbors in
`journal.jsonl`: after the merge, the lines of both are in `journal.jsonl` and
in the archive file of the branch that moved them, so none is lost.

## Archive

`mtqg archive <start>..<end>` moves the lines of items whose **last event**
falls in the date range (local dates, both ends inclusive) from `journal.jsonl`
to `archive/<start>..<end>.jsonl`:

| Kind | Archived when |
|---|---|
| `todo` | state is `done` |
| `qa` question | state is `done`; its answers move with it |
| `bug` | state is `done`; its replies move with it |
| `memo` | always |
| `glossary` | never |
| `rule` | never |

- **The last event of a question or a bug is the latest of its own events and
  those of its answers or replies** (deleted ones included). A question closed in
  2023 that was answered in 2025 is not archived by a range that ends in 2023, so
  that every line that moves is dated within the range.
- **A deleted record is archived whatever its kind and state**, when its last
  event (the `delete`, or a later one) is in the range. It is out of view already.
- An answer or a reply follows its question or bug when that is in
  `journal.jsonl`. One without it (its `re` names nothing there, or a record of
  another type) is an item of its own, like a memo.
- All lines of an item move together, byte for byte. Lines that cannot be read,
  and events of an `id` that has no `create` in `journal.jsonl`, stay.
- The archive file is made when there is a line to put in it. Archiving the same
  range again appends to it; the lines end in LF like those of `journal.jsonl`.

To restore, append the archive file to `journal.jsonl` and delete it:

```
cat .mtqg/archive/2021-01-01..2024-09-18.jsonl >> .mtqg/journal.jsonl
rm .mtqg/archive/2021-01-01..2024-09-18.jsonl
```

## Versioning

- `.mtqg/version` declares the format of the directory as one integer.
- A reader that sees a `version` newer than it knows **must refuse to read or
  write**.
- Writers write in the format declared by `version`, even if they know a newer
  one. The format is raised only by an explicit upgrade, which rewrites
  `version` and `SCHEMA.md` together. Existing lines are not rewritten.
- Each line carries `v`, the format it was written in. Readers interpret each
  line by its own `v`.
- Adding a field that old readers can ignore without misreading does not change
  the version. Changing the meaning of a field or adding a new `op` does.
- Before mtqg v1 the version was `0`, meaning "not yet stable": the format could
  change freely, and existing lines could be rewritten or discarded. `mtqg
  upgrade` raised it to `1` without touching any line; a line's own `v` is
  still read as written (a line with `v:0` means the same thing a line with
  `v:1` does). `0` will not be reused.
- `SCHEMA.md` also carries a marker comment naming the mtqg release its content
  matches (below). This is separate from the format version: `SCHEMA.md` can
  change (a field's description gets clearer, a section is added) without the
  format itself changing. `mtqg upgrade` rewrites `SCHEMA.md` from this
  document whenever that marker is missing or older than the release this
  mtqg was built from, even if the format version does not change.

<!-- schema as of mtqg 1.1.0 -->

## Writing rules

Tools that write events should produce lines exactly like mtqg does, so that
diffs stay readable and identical events stay identical:

- Compact JSON, no spaces between tokens.
- Keys in this order, omitting absent ones:
  `id, op, type, re, from, status, basis, word, text, v, ts, author, tty`.
  Inside `author`: `kind, name`.
- Do not escape non-ASCII characters, and do not escape `<`, `>`, `&`.
- End every line with LF.
- Append only. Write a whole line in one write.

Readers must accept any valid JSON regardless of key order, spacing or
escaping. Key names are case-sensitive. If a key appears more than once in one
line, the last value is used.
