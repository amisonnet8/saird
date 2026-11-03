# CLAUDE.md

このファイルは、このプロジェクトで作業する際、Claude Codeが常に踏まえるべき前提を示す。

## このプロジェクトについて

Saird は AI 専用プログラミング言語。仕様を作りながら、そのツール群を Rust で実装する。

- 仕様の版：**v0.5.3**。仕様の本体は`docs/`にある
  - `design/`：考え方、信頼モデル、ビルドと実行、粒度、入れないもの、弱点
  - `reference/`：文法、契約、effects、生成テスト、権限、組立書、エラー、用語集
  - `examples/`：ReserveStock、Checkout、呼び出し
- Saird は**記述形式**（契約と組立書を書くための形式）と**ツール群**（それらを検査し、実行するもの）でできている。このリポジトリの Rust のコードはツール群。箱の実装は任意の言語で書き、Wasm にコンパイルする。この2つを混同しない
- Cargo は workspace にし、信頼基盤の層に合わせて crate を分ける：`saird-core`（小さく監査する層）、`saird-runtime`（広く使われる層）、`saird`（コマンドと表示。実行ファイルも`saird`）。置き場所の判断基準は`.claude/rules/directory-structure.md`
- 経緯と実装の判断は mtqg に書く（`docs/`には経緯を書かない。`.claude/rules/documentation.md`）

## 最初に作る範囲

`docs/`には置き場所が無いので、ここに書く。

- 対応言語、WASI の版、ランタイムの版を固定して始める：箱の実装言語は Rust（`wasm32-wasip2`）、WASI 0.2（Component Model）、ランタイムは Wasmtime の最新の安定版（正確な版は依存を足すときに決め、`Cargo.lock`で固定する）
- MVP：純粋計算の箱、kv（範囲テンプレート、atomic。箱ごとに範囲は1つ）、clock（read）、契約の解析、生成テスト、封印、レシート、契約承認、適合証明、権限サマリ
  - 完了条件：人間が実装を読まずに契約を承認でき、kv の範囲と atomic がホストによって強制される
  - min_hits 不足を例テストで代替できるのは契約承認時だけとし、生成テストの不足は警告にする
  - seed は、封印物のハッシュとテスト実行器のローカルな記録から導出し、その導出をレシートに記録する。透明性ログへの登録は後の段階
- 段階2：idempotent、観測プロファイルによる承認の引き継ぎ
  - 完了条件：再実行・同時実行で二重に処理されない
- 段階3：http、秘密情報、外部への書き込みと補償
  - 完了条件：秘密情報が範囲外の宛先・応答・ログに出ない

## 未決事項（MVP の実装で決める）

- http の許可クエリ：未知のクエリは拒否、値は文字列の上限だけ、という案
- 名前の確認：GitHub、npm、crates.io、PyPI、ドメイン
- 読み方

## 守ること（設計の芯）

実装の細部より優先する。これを崩す変更は、実装する前に必ず相談すること。

- **人間はコードではなく、契約と証拠を信頼する。実装は外に出られない箱に閉じ込め、確かめるのは信頼基盤に任せる**（`docs/design/overview.md`）。箱の実装の出来に頼って、検査や強制を省かない
- **仕様（`docs/`）が正。** 実装を仕様に合わせる。仕様が曖昧・矛盾している・実装できないと分かったら、実装で埋めずに人間に聞く（`.claude/rules/working-with-human.md`の「重要な判断」）
- **保証の担い手を混ぜない**（`docs/design/trust.md`「保証の分担」）。静的検査は検査器、範囲外の操作を止めるのはホストと標準箱、記録はホスト。effects はテストでなくホストが保証する（`docs/reference/effects.md`）
- **検査器とレシート検証器は、小さく保つ**（「信頼基盤」の小さく監査する層）。広く使われる層（ホスト・標準箱・テスト実行器）や表示のコードを混ぜない。信頼基盤に入る依存は人間に聞く
- **箱には、範囲を限定した capability だけを渡す。** 実行時の設定は権限を狭められるが、広げられない（`docs/design/build-and-runtime.md`）
- **Saird は実装の正しさを証明しない。** レシートが示すのは「指定した信頼基盤が、指定した封印物と契約に対して、記録された条件でテストを実行し、この結果を得たこと」だけ。これを超える主張を、出力・レシートでしない
- **`docs/design/non-goals.md`の「入れないもの」を足さない。** 「後回しにするもの」は、人間が決めるまで作らない。機能を足すより、曖昧さを消す定義を優先する

## ドキュメントの言語

- **コード・コメント・CLI の出力（メッセージ・エラー）は英語**
- `CLAUDE.md`・`.claude/rules/`・mtqg の記録は日本語
- `docs/`と`README.md`は、**今は日本語だけ**。公開の前に、英語を正本（`*.md`）にし、日本語を`*_ja.md`に移して整える（いつするかは人間が決める）
- crube の説明は英語（`structure.txt`）と日本語（`structure_ja.txt`）の両方
- 詳しい書き方は`.claude/rules/documentation.md`

## 参照すべきファイル

作業を始める前に、以下を確認すること。

- **`.claude/rules/bcuz-usage.md`** — bcuzの使い方。**Bashで実行するコマンドは、必ず`bcuz "理由" コマンド…`の形で打つ**（フックが、そうでないものを断る）。理由は「なぜ」を書く。**コマンドを打つ前に読む**
- **`.claude/rules/mtqg-usage.md`** — mtqgの使い方（記録者、種類の使い分け、こまめに記録する）
- **`.claude/rules/srwr-usage.md`** — srwrの使い方（**厳格モード**）。**`Edit`・`Write`は禁止されており、ファイルの変更は`look`／`edit`／`new`（理由`why`つき）で行う。作業のまとまりごとに`session`で題つきのテープを始め、コミットの前にテープを閉じる。ファイルを変える前に読む**
- **`.claude/rules/qsoku-usage.md`** — qsokuの使い方（`qsokufile`の名前の一覧、呼び方、書式、`//`、シェル連携）。**ビルド・テスト・lintは`cargo`などを直接打たず、ここの名前で実行する**
- **`.claude/rules/crube-usage.md`** — crubeの使い方（MCPの`crube`・`crube-ja`とCLI、場所の探し方、英語と日本語の説明の保ち方）。**場所を探す前、ファイルを足す・動かす・消す前に読む**
- **`.claude/rules/testing.md`** — テスト方針（開発環境の前提、動作確認の粒度、期待値の決め方、壊して確かめる（Skill`mutation-check`）、cargo-deny、ShellCheck、Bashサンドボックス・CIの落とし穴）
- **`.claude/rules/directory-structure.md`** — ディレクトリ構成と、ファイルの置き場所の判断基準。**新しいファイルを追加するときに読む**
- **`.claude/rules/datetime.md`** — 日時の扱い（データは UTC で持ち、表示だけ利用者のタイムゾーンに直す。瞬間で比べる、時計を差し替える）。**日時を保存・表示・比較するコードを書くときに読む**
- **`.claude/rules/working-with-human.md`** — 人間とのやり取り（頼み方の3種類、小さなつまずきの頼み方、確認の自動化、見え方を変えるときの了承）。**人間に何かを頼む前に読む**
- **`.claude/rules/naming.md`** — 命名規則（未記入。決まったら書く）
- **`.claude/rules/distribution.md`** — 配布方法（コマンド`saird`を GitHub Releases で配る、実装の版と仕様の版、goreleaserでの配布物の作り方とリリースの流れ）
- **`.claude/rules/documentation.md`** — ドキュメントの書き方（文書ごとの言語、文体、`docs/`の直し方、例の値は道具で求める、経緯は mtqg に書く）

これらのルールファイルは、実装中に得た細かい気づき・教訓をClaude Code自身が育てていくものである。新しく気づいたルール・踏んだ落とし穴があれば、該当するファイルに追記すること。どのファイルにも当てはまらない新しい種類の気づきであれば、新しいルールファイルを作ってよい（作ったらこの一覧にも足すこと）。

## 文書と今のコードが食い違うとき

- **文書（`CLAUDE.md`・`.claude/rules/`・`docs/`・コメント）の書き方と、すでにあるコードの書き方が食い違うときは、コードの書き方に従う。** 命名・構成・エラーの扱い・テストの形などは、文書の例より、今のコードで実際に使われている形に合わせて書く
- **見つけた食い違いは、その作業の最後の返答で全部挙げる。** 文書のどこと、コードのどこが、どう違うかを1件ずつ書く。どちらに合わせて直すかは人間が決めるので、文書もコードも、その場では直さない（`.claude/rules/working-with-human.md` 1章「別の作業の流れで見つけた問題は、先に直さず提案する」）

## 開発の進め方

- **Bashのコマンドは、すべて`bcuz "理由" コマンド…`で実行する**（`bcuz "lintの確認" qsoku lint`）。理由は「何をするか」でなく「なぜするか」。`bcuz`で始まらない・`&&`や`;`でつなぐコマンドは、`.claude/settings.json`の`PreToolUse`フック（`.claude/hooks/require-bcuz.sh`）が実行前に断る。**`git commit`だけは`bcuz`を通さない**（通すとコミットの後に記録が増えるため。理由はコミットメッセージに書く）。実行の記録は`.bcuz/history.jsonl`に残り（コミットする）、`bcuz -H`で見られる。**トークンなどの秘密を、コマンドの引数・`env`の指定・理由に書かない**（記録に残り、コミットされる）（`.claude/rules/bcuz-usage.md`）
- **ファイルの変更は、srwrの道具（`look`で見て`edit`、新規は`new`）で行い、すべてに理由（`why`）を添える。** 組み込みの`Edit`・`Write`は`.claude/settings.json`で禁止されている（厳格モード）。Bashでの書き換え（`sed -i`・リダイレクト）もしない
- **作業のまとまり（todo・bug・人間の依頼・作業の種類の変わり目）ごとに、最初に`session`で題つきのテープを始める。** 1つの作業＝1本のテープ、コミットも同じ区切り
- **コミットの前に、テープを閉じる**（`srwr tapes check`のあと`srwr tapes new`）。コミットまでの手順はSkill`commit-with-tape`。書き込み中の生のテープは`.gitignore`で対象外で、閉じて圧縮した`.srwr/tapes/*.tape.jsonl.gz`だけがgitに入る（`.claude/rules/srwr-usage.md`）
- **このリポジトリの中の場所は、まずcrubeで引く**（MCPの`find`・`tree`・`show`、または`crube find …`）。ファイル・ディレクトリを足す・動かす・消すときは、同じ変更の中で英語と日本語の説明を直し、コミットの前に両方の言語で`crube status --require all`を通す（`.claude/rules/crube-usage.md`）
- 実装後は`.claude/rules/testing.md`に従い、ビルド確認に加えて実際の動作確認（`bcuz "…" qsoku check`、必要に応じて`qsoku deny`・`qsoku shellcheck`）を行うこと。判断に迷ったら実行する側に倒すこと
- 依存ライブラリ（crate）は、標準ライブラリ → Rust の公式（`rust-lang`）の crate → 何かの公式のもの → デファクトの順で選ぶ（依存を足すこと自体が人間に聞く判断。`.claude/rules/working-with-human.md`）
- 依存ライブラリを足したら、cargo-denyでライセンス・脆弱性・出どころを確認すること（`qsoku deny`）。許すライセンスは`deny.toml`と`about.toml`の両方にある必要がある（`.claude/rules/distribution.md`）

## gitの扱い（Claude Codeの作業として）

- **pushは人間が行う。** Claude Codeはコミットまでで止めること。`git push`は`.claude/settings.json`で拒否（deny）されている
- 作業の区切りでこまめにコミットしてよい（消してしまったときに戻せるように）
- `git reset --hard`・`git clean`は、コミットしていない作業を消すので、使う前に必ず理由を説明すること
- **コミットの前に`git config user.email`／`user.name`を確かめる。** 正しい値は`65074207+amisonnet8@users.noreply.github.com`／`amisonnet8`（GitHubのプライバシー用noreplyアドレス）。違う値なら、`.git/config`の`[user]`（ローカルの上書き）を疑う。ローカルに設定が無いと、コンテナのglobalの設定（noreplyでないことがある）が使われる

## 権限・自動化について

`.claude/settings.json`（人間が管理する）により、次の設定になっている。

- 拒否（deny）：`git push`、`gh pr merge`、`gh release create`
- **`bcuz`を通さないBashコマンドの拒否**：`PreToolUse`フック（`.claude/hooks/require-bcuz.sh`）が、`bcuz`で始まらないコマンド（`git commit`を除く。逆に`bcuz`を通した`git commit`を断る）と、後ろに別のコマンドをつなぐもの（`&&`・`||`・`;`・`&`・改行・単一引用符の外のコマンド置換）を、実行前に断る。直し方は断りのメッセージと`.claude/rules/bcuz-usage.md`に書いてある
- 実行前に確認（ask）：`git reset --hard`、`git clean`、`sudo`、`gh pr create`
- **Bashコマンドは既定でOSレベルのサンドボックス（Linux bubblewrap）内で実行される。** ファイルシステムの書き込み先とネットワーク接続先は許可リストで絞られている。この範囲内なら`curl`・`wget`・`cargo add`などは確認なしに実行できる。リストに無い宛先・書き込み先は`sandbox_violations`として拒否される。そのときは、`.claude/rules/working-with-human.md` 2章の形（原因と、`.claude/settings.json` のどこに何を足すか）で**短く頼む**（許可を広げるかは人間が判断する）。落とし穴は`.claude/rules/testing.md`「Bashサンドボックスの落とし穴」参照

確認を求められた場合、無理に実行しようとしない。`.claude/settings.json`を変更したいときは、変更案を差分で示すにとどめること（人間が貼るだけで済む形にする）。

**srwrの厳格モード**：`permissions.deny`に`Edit`・`Write`・`MultiEdit`・`NotebookEdit`があり、`.mcp.json`にsrwrのMCPサーバー、`PostToolUse`に`srwr hook`（読み取りと外部変更の記録）がある。テープ（`.srwr/tapes/`）はgitで共有する（鍵`.srwr/key`は入れない）。許可を広げる（緩いモードにする）のは人間の判断。詳細は`.claude/rules/srwr-usage.md`。

ビルドの自動フック（`PostToolUse`）が設定されている。srwrの`edit`・`new`で`.rs`・`Cargo.toml`・`Cargo.lock`を変えると`.claude/hooks/build.sh`が`qsoku build`を実行し、失敗すると理由がClaude Codeに返る。複数ファイルにまたがる編集の途中は、まだ書いていないファイルを参照して一時的に失敗することがある。

**crubeの配線**：`.mcp.json`に、crubeのMCPサーバーが2つある（`crube`は英語、`crube-ja`は日本語。`crube mcp --allow-write`）。srwrの`new`のあとには`.claude/hooks/crube-untracked.sh`が、説明の無いパスを言語ごとに知らせる（止めはしない）。詳細は`.claude/rules/crube-usage.md`。

## mtqgでの記録について

作業の中で生まれる「気づき・やること・質問と答え・不具合とそのやり取り・用語の合意・この先も効き続ける決まり事」は、コードにもコミットにも残らず、チャットの履歴やメモアプリに散って失われやすい。`mtqg`（`m`emo & rules・`t`odo・`q`a & bugs・`g`lossary）はそれを`.mtqg/journal.jsonl`に追記し、gitに乗せて共有する。使い方は`.claude/rules/mtqg-usage.md`。`.mtqg/`にはテンプレート（rust-template）から引き継いだ決まり事（rule）を10件登録済み——`mtqg context`で確認できる。AIエージェントからは、フック（`.claude/settings.json`の`mtqg hook`）とMCPサーバー（`.mcp.json`の`mtqg mcp`）の両方で使える。

## ルール・スキルの提案

作業を進める中で、以下に気づいたら、都度こちらから提案すること（提案するだけで、勝手に作成・適用はしない。判断はこちらが行う）。

- **新しいルールにした方がよさそうな知見**: 同じ種類の判断や落とし穴に複数回遭遇した、既存の`.claude/rules/`のどれにも当てはまらない、といった場合。どのファイルに追記すべきか、あるいは新規ファイルが必要かも合わせて提案する
- **Skill化した方が効率的そうな作業**: 「同じ手順を3回目繰り返している」など、パターン化できそうな一連の作業に気づいた場合。まだ実装が薄い段階では無理に提案する必要はない。パターンが実際に繰り返されてから提案すること

提案は気づいたタイミングで随時行ってよく、まとめて報告するために貯めておく必要はない。

This repository records its development with mtqg: run `mtqg context` at the start of a session, and see `.mtqg/SCHEMA.md` for the data format.
