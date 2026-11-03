# srwrの使い方（厳格モード）

このプロジェクトでは、AIのファイル編集を**srwr**（`github.com/amisonnet8/srwr`）の道具で行う。すべての編集に理由（`why`）が付き、操作が**テープ**（`.srwr/tapes/`）に残り、人間がコマ送りで再生できる。`srwr init`の既定である**厳格モード**で使う。

## 最初に（テンプレートから作ったプロジェクトでは、何も要らない）

- `.devcontainer/postCreate.sh`が`srwr`を`@latest`で入れる（`srwr --version`で確かめる）
- **`srwr init`は動かさなくてよい。** 設定（`.mcp.json`・`.claude/settings.json`・`.gitignore`）はテンプレートに入っていて、`srwr init`が書くものと同じ。**`.srwr/`と鍵は、最初のhookかMCPの呼び出しで自動的にできる**（動かさないままで確かめた）。確かめたいときだけ動かす：設定が揃っていれば`Already set up. Nothing was rewritten.`と出て、何も書き換えない
- **`srwr init`が要るのは**、テンプレートを使わず、すでにあるプロジェクトにsrwrを足すとき、srwrを更新して道具の名前が変わったとき、厳格⇄緩いを切り替えるとき（下の「落とし穴」）
- **Claude Codeを開き直す。** `.mcp.json`のsrwrサーバーと許可は起動時に読まれるので、開き直すまで`mcp__srwr__*`は現れない（コンテナを開き直す場合は、そのとき読まれる）

## 厳格モードとは

`.claude/settings.json`で、Claude Codeの`Edit`・`Write`・`MultiEdit`・`NotebookEdit`を禁止している。**ファイルを変える入口は、srwrの道具だけ**。

| 道具 | 使い方 |
|---|---|
| `mcp__srwr__session` | **作業のまとまりの最初に呼ぶ**。題（`title`）と理由（`why`）つきで新しいテープを始める。ファイルには触れない（下の「セッション」） |
| `mcp__srwr__look` | ファイルの範囲を見る。`search`で探す（ディレクトリも可）。`looks`で複数ファイルを1回で見る。範囲トークン（`sel_…`）が返る |
| `mcp__srwr__edit` | 範囲を書き換える。`selection`（`look`のトークン）、または`file`＋`expect`、行の一部は`old`＋`new`。同じ変更を複数箇所に入れるときは`edits`（`why`は1つ、全部入るか全部入らないか）。**`edits`では`file`を各項目の中に書く**（`{"why": "…", "edits": [{"file": "a.rs", "old": "x", "new": "y"}, …]}`。`edits`の外に書くと断られる） |
| `mcp__srwr__new` | まだ無いファイルを、内容つきで作る |

- **`why`は必須**（空白だけも不可）。「何をするか」でなく**「なぜそうするか」**を1〜2文で書く。人間が再生で読む
- 読むだけなら、組み込みの`Read`・`Grep`・`Bash`の`cat`なども使える（hookが記録する）。**ただし、このあと直すつもりなら最初から`look`を使う**（トークンが得られ、読んだ記録と変更が1つの流れで残る）
- ツールの入力の詳細は、srwrの`docs/reference/mcp.md`（https://github.com/amisonnet8/srwr/blob/main/docs/reference/mcp.md）

## セッション（テープの区切り）を細かく使う

**1つの作業のまとまり＝1本のテープ**にする。人間は`srwr tapes`の一覧（題が出る）から「あの作業」を選んで再生するので、区切りと題が、再生の入口になる。区切らないと、30分空くまで別の作業が1本に混ざる。

### いつ`session`を呼ぶか

- **mtqgのtodoに着手するとき**（`mtqg todo add`のあと、最初の`look`の前）。1つのtodoが1本
- **bugの調査・修正に入るとき**、**人間から新しい依頼を受けたとき**
- **作業の種類が変わるとき**（実装→文書、コード→CI、など。1つのtodoの中でも、種類が変われば区切る）
- **レビューの指摘を直すとき**（元の作業とは別のテープにする。直しの理由だけを読めるように）
- 呼ばない：1つの編集ごと、確認の`look`ごと。**同じ作業の途中では呼び直さない**

### 題（`title`）と理由（`why`）

- `title`：1行・80文字以内。**「何の作業か」が一覧で分かる形**にする。mtqgのIDがあれば入れる。例：`todo 0822a2c167: 起動時に設定を読み込む`、`bug 2d4ad01d30: qsoku testが落ちる`、`docs: ディレクトリ構成を書く`
- `why`：なぜこの作業を始めるか、を1〜2文。todoの本文をそのまま写すのでなく、**今回やる範囲**を書く
- 題は「後から読む人」のために書く。「修正」「作業」「続き」のような、中身のない題にしない
- 空白だけ・複数行・80文字超は`invalid_input`になる

### コミットの前に、テープを閉じる

**コミットするときは、Skill`commit-with-tape`（`.claude/skills/commit-with-tape/SKILL.md`）の手順で行う。** 下はその要点。

```
crube status --require all                  # 説明の漏れが無いか（.claude/rules/crube-usage.md）
env CRUBE_LANG=ja crube status --require all  # 日本語も
srwr tapes check   # テープに無いのに変わったファイルが無いか（閉じる前に）
srwr tapes new     # 今のテープを閉じて、その場で圧縮する
git add … && git commit …
```

- **書き込み中の生のテープ（`*.tape.jsonl`）は`.gitignore`でgitの対象外**。閉じて圧縮した`*.tape.jsonl.gz`だけが入る。**閉じ忘れると、そのコミットにテープが入らない**（`srwr trace`でその行が引けない）
- 生のテープをコミットすると、閉じたときに次のコミットへ「生の削除＋`.gz`の追加」だけが出る。それを避けるための決まり
- 閉じたあとの書き込みは新しいテープを始める。**コミットのあと、同じ作業が続くなら`session`で題つきの新しいテープを始める**
- `srwr tapes check`は、出たものを、生成物なら`.srwrignore`に足すか、そのままでよいと判断し、手で書いたものなら理由を人間に伝える
- **コミットは、テープと同じ区切りにする。** 1本のテープ＝1つか数個のコミット。コミットメッセージにテープのID（`srwr tapes`で分かる）を書くと、`git show <commit> | srwr trace`で「どの行がどの操作・理由か」を引ける

### そのほか

- **作業のまとまりが終わったら閉じる**：次の作業に移る前に、次の`session`を呼ぶ（前のテープを閉じて圧縮し、新しいテープを始める）
- 何も記録しなかったテープ（`session`だけ呼んだ）は、次の`session`で消える。気にせず呼んでよい
- 30分空くと、`session`を呼ばなくても自動で新しいテープになる（題は付かない）。**題つきで始めるのが原則**

## 守ること

- **`Edit`・`Write`を使おうとしない。** 禁止されていて、拒否される。拒否されたら`look`＋`edit`／`new`に切り替える
- **プランモードでは、計画のファイル（`~/.claude/plans/`）を書けない**（`Write`が禁止されているため）。計画は会話に文章で示してから`ExitPlanMode`を呼ぶ。`settings.json`で`~/.claude/plans/`への`Write`だけを許しても、`Write`の拒否（deny）が優先されるので効かない
- **Bashでファイルを書き換えない**（`sed -i`・リダイレクト・`tee`・`cat > file`など）。止められないが、理由のない`external`としてテープに残り、再生が読みにくくなる。ジェネレーターや整形ツール（`cargo build`・`qsoku fmt`など）が書くものは、そのままでよい
- **srwrにはファイルを消す道具が無い。** 消すときは`bcuz "理由" git rm <ファイル>`で消す（理由は`bcuz`の記録に残る）。`srwr tapes check`には「テープにない`D`」として出るが、それでよい。コミットメッセージにも消した理由を書く
- **`.mtqg/`は`mtqg`のコマンド経由だけ**（`.claude/rules/mtqg-usage.md`）。`.srwrignore`で記録から外してあるので、`look`・`edit`は`ignored_file`で断られる
- **`.claude/settings.json`は人間が管理する。** 変更案は差分で示し、`edit`で書き換えない。`.mcp.json`・`.devcontainer/`も、人間の依頼があったときだけ変える
- 人間が事情を聞くときは、テープのIDで答えられる（`srwr tapes`で一覧、`srwr view`で再生）

## このプロジェクトでの取り決め

- **テープは共有する**（閉じて圧縮したテープ`.srwr/tapes/*.tape.jsonl.gz`をgitに入れる。`.gitignore`で`.srwr/`の中は`tapes/`だけ戻し、書き込み中の生のテープ`*.tape.jsonl`は対象外）。clone先で`srwr view`・`srwr trace`が使える。**鍵`.srwr/key`・ロック・`active`・`init-backup/`は入れない**。テープはファイルの全文を含むので、秘密のファイルは記録しない（`.srwrignore`と、`.env`・`*.pem`・`*.key`などの既定の対象）
- **記録しないもの**（`.srwrignore`）：`.mtqg/`・`.bcuz/`。生成物・巨大なファイルは、プロジェクトで足す
- **srwrは`.devcontainer/postCreate.sh`で`@latest`**（qsoku・mtqgと同じ）。0.xの間は道具の名前や入力が変わりうるので、**コンテナを作り直したら`srwr --version`を見て、版が変わっていたら`srwr init`を1回動かす**（許可する道具の名前が`.claude/settings.json`に追従する）
- 厳格⇄緩いの切り替えは`srwr init`／`srwr init --lenient`。**緩いモードにするのは人間の判断**（`Write`が記録されず`external`になる）。AIが勝手に切り替えない
- hookの`build.sh`は、srwrの`edit`・`new`の入力（`file`・`edits[].file`）を見て、`.rs`・`Cargo.toml`・`Cargo.lock`のときだけ`qsoku build`を動かす（組み込みの`Edit`・`Write`は禁止なので、そちらでは動かない）

## 落とし穴

- **`srwr init`は、書き換えるものがあるときは、Bashサンドボックスの中では`.mcp.json`・`.claude/settings.json`を書き換えられない**（一時ファイルから`rename`するが、サンドボックスが`device or resource busy`にする。`.srwr/`の鍵だけが先にできる）。書き換えるものが無ければ（`Already set up`）、サンドボックスの中でも動く。書き換えが要るときは、人間がサンドボックスの外のターミナルで動かす
- `srwr`が`PATH`に無いと、`.mcp.json`もhookも動かない（hookは失敗しても作業は止まらず、記録だけが抜ける）。`srwr --version`で確かめる
- **`session`を呼ぶと、作業場全体のテープが切り替わる**（`srwr tapes new`と同じ）。ほかのセッション（人間が別のClaude Codeを動かしているなど）にも効く。範囲トークンは、閉じたテープのものは使えなくなるので、`session`のあとは`look`し直す
- `srwr view`は`~/.cache/srwr`に書くので、サンドボックスの中では動かさず、人間のターミナルで動かす
