---
name: commit-with-tape
description: 作業のまとまりをコミットする手順。srwrのテープを検査して閉じ、変更・mtqgとbcuzの記録・閉じたテープを入れて、テープのIDつきのメッセージでgit commitする。todoやbugの作業が終わり、コミットするときに使う。
---

# テープを閉じてコミットする

このプロジェクトのコミットは、**srwrのテープを閉じてから**、変更と記録（`.mtqg/`・`.bcuz/`）と閉じたテープを一緒に入れる（`.claude/rules/srwr-usage.md`「コミットの前に、テープを閉じる」）。この手順を、毎回同じ順で行う。

Bashのコマンドは、`git commit`を除いて、すべて`bcuz "理由" …`で打つ。1回に1コマンド（`&&`でつながない）。

## 0. コミットの前にそろえるもの

- コードを変えたなら、`bcuz "<何の変更>のあとの確認" qsoku check`が通っている（依存を足したなら`qsoku deny`も）
- mtqgの記録を先に済ませる：終わったtodoの`todo done`、決めたこと・気づいたことの`memo add`。**コミットの後に書くと、そのコミットに入らない**
- このリポジトリの説明がそろっている：`bcuz "<作業>のコミットの前に、英語の説明の漏れを確かめる" crube status --require all`と、`env CRUBE_LANG=ja`を付けた同じものの両方が通る。落ちたら、両方の言語で説明を直す（`.claude/rules/crube-usage.md`）
- 構成を変えたなら`.claude/rules/directory-structure.md`、決めたことを書く文書（`docs/design/`など）があればそれも、同じ変更に入れる

## 1. テープを検査する

```
bcuz "<作業>のコミットの前に、テープに無い変更が無いか確かめる" srwr tapes check
```

「テープにないファイル」が出たら（終了コード1）、1件ずつ判断する。

| 出たもの | 判断 |
|---|---|
| `Cargo.lock`（`cargo`が書いた）・`Cargo.toml`（`cargo add`が書いた） | 生成物なので、そのままでよい |
| `.crube/structure.txt`・`.crube/structure_ja.txt`（crube の CLI・MCP が書いた） | crube が書くものは理由なしでよい（`.claude/rules/crube-usage.md`「落とし穴」）。そのままでよい |
| `D`（`bcuz "…" git rm`で消した） | srwrに削除の道具が無いため。そのままでよい。消した理由をコミットメッセージに書く |
| 手で書いたファイル（Bashで書き換えた、など） | 止まって、人間に理由を伝える（`.claude/rules/srwr-usage.md`） |
| 生成物で、毎回出るもの | `.srwrignore`に足すかを判断する |

`.bcuz/history.jsonl`・`.mtqg/journal.jsonl`は「設定で記録しないファイル」に出る。正常。

## 2. テープを閉じる

```
bcuz "<作業>のコミットに入れるため、テープを閉じて圧縮する" srwr tapes new
```

## 3. 入るものを確かめる

```
bcuz "<作業>のコミットに入るものを確かめる" git status --short
```

- `.srwr/tapes/*.tape.jsonl.gz`の**未追跡のものはすべて**入れる。今閉じたテープのほかに、前のコミットのあとに自動で始まった短いテープ（`session`が閉じたもの）もある
- 生のテープ（`*.tape.jsonl`）は`.gitignore`で出ない。出たらおかしいので止まる
- サンドボックスのダミーファイル（`.bashrc`など）は`.gitignore`で出ない
- 別のPJの変更が混ざっていないか見る（`.claude/rules/mtqg-usage.md`「このPJと関係のないことは記録しない」）

## 4. コミット対象にする

**パスを指定して**足す。`git add -A`は、`--dry-run`で中身を確かめたときだけ使う。

```
bcuz "<作業>の変更・記録・テープをコミット対象にする" git add .bcuz/history.jsonl .mtqg/journal.jsonl .srwr/tapes <変えたファイル・ディレクトリ>
```

足したあと、もう一度`git status --short`で確かめる。`.bcuz/history.jsonl`が`MM`になるのは正常（`git add`自身の記録が、足したあとに書かれるため。次のコミットに入る）。

## 5. コミットする

`git commit`だけは`bcuz`を通さない（フックが断る）。コマンド置換（`$(…)`）も使えないので、メッセージは引用符の中で改行して書く。

```
git commit -m "<何をしたか、1行>（[todoの番号]）

<なぜこの変更をしたか。何を作り・直したか>

テープ: <今回入れたテープのID>, <…>

Co-Authored-By: <システムの指示にある帰属の行>"
```

- 1行目：変更の要約。mtqgのtodoに番号（`[2-3]`など）があれば添える
- 本文：**なぜ**を先に書く。消したファイルがあれば、その理由も書く
- `テープ:`：手順3で入れた`.gz`のID（`20261020-1902-ncmt`の形）を、すべて書く。`git show <commit> | srwr trace`で行から操作と理由を引くため
- 最後に、システムの指示にある帰属の行（`Co-Authored-By: …`）を付ける
- `git config user.email`／`user.name`が`65074207+amisonnet8@users.noreply.github.com`／`amisonnet8`であることを、セッションの最初のコミットの前に1度確かめる（`CLAUDE.md`「gitの扱い」）

## 6. コミットのあと

- 人間には、コミットのハッシュと、入れたものの要約を伝える。**pushはしない**（人間が行う）
- `.bcuz/history.jsonl`の最後の数行だけが未コミットで残る。正常で、次のコミットに入る
- 同じ作業が続くなら、`mcp__srwr__session`で題つきの新しいテープを始める。次の作業に移るときも同じ
