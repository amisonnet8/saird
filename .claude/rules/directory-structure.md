# ディレクトリ構成

新しいファイルを追加する際は、どこに置くかをここで確かめる。**構成を変えたら、この文書も同じ変更の中で直す。**

**このリポジトリの説明（`.crube/`）も保つ。** ファイル・ディレクトリを足す・動かす・消すときは、同じ変更の中で英語と日本語の説明を直し、コミットの前に両方の言語で`crube status --require all`を通す。やり方（MCPとCLI、`.crube/ignore`の保ち方）は`.claude/rules/crube-usage.md`

```
.
├── CLAUDE.md               ← プロジェクトルール（参照先の案内）
├── LICENSE                 （MIT）
├── README.md
├── qsokufile               ← ビルド・テストの近道（qsoku）
├── Cargo.toml / Cargo.lock ← パッケージの定義・lint の水準（`[lints]`）／依存の版の固定
├── rust-toolchain.toml     ← 使う Rust のツールチェイン（stable。rustfmt・clippy つき）
├── .gitattributes          （`* text=auto eol=lf`）
├── .gitignore
├── .goreleaser.yaml        ← 配布物の作り方（goreleaser。`release.yml`と`qsoku snapshot`が使う。`.claude/rules/distribution.md`）
├── deny.toml               ← 依存の脆弱性・ライセンス・出どころの検査の設定（cargo-deny）
├── about.toml / about.hbs  ← 配布物に入れる依存のライセンス文の作り方（cargo-about）
├── .mcp.json               ← mtqg・srwrとcrube（英語の`crube`・日本語の`crube-ja`）のMCPサーバーの配線
├── .srwrignore             ← srwrがテープに記録しないファイル（`.mtqg/`・`.bcuz/`など）
├── .bcuz/                  ← bcuzの記録（`history.jsonl`）。`postCreate.sh`が`bcuz --init`で作る。**コミットする**（`merge=union`）
├── .crube/                 ← このリポジトリのファイルの説明（crube。`structure.txt`は英語、`structure_ja.txt`は日本語、`ignore`は`.gitignore`に合わせて手で保つ）。**コミットする**
├── .srwr/                  ← srwrの鍵・ロック・テープ（最初のhook・MCPの呼び出しでできる。**閉じた`tapes/*.gz`だけコミットする**）
├── src/                    ← Rust のコード（`lib.rs`がライブラリ、コマンドは`main.rs`か`bin/`）
├── docs/                   ← Saird の仕様（`.claude/rules/documentation.md`）
│   ├── design/             ← 考え方、信頼モデル、ビルドと実行、粒度、入れないもの、弱点、再利用
│   ├── reference/          ← 契約、effects、生成テスト、権限、組立書、エラー、用語集
│   └── examples/           ← ReserveStock、Checkout、呼び出し
├── .devcontainer/          ← 開発環境（devcontainer.json、postCreate.sh）
├── .claude/                ← rules/（ルール）、skills/（commit-with-tape：テープを閉じてコミットする手順。mutation-check：壊して確かめる手順と、仕様の変異の道具`mutants/`（別の Go モジュール）・仕様の変異の一覧`spec.txt`）、hooks/（build.sh、require-bcuz.sh とそのテスト、crube-untracked.sh：srwr の new のあとに未説明を知らせる）、settings.json（人間が管理する）
├── .github/workflows/      ← CI（`ci.yml`）と、タグで動くリリース（`release.yml`。`ci.yml`を呼んでから Release を作って公開する）
└── .mtqg/                  ← mtqgの記録
```

## 配置の判断基準

- **パッケージは`saird`1つ。** ライブラリ（`src/lib.rs`とその下のモジュール）に処理を置き、`src/main.rs`（コマンド`saird`）は引数を解いてライブラリを呼ぶだけにする。workspace に分けるかは、MVP の実装で要るときに人間が決める
- **モジュールは、信頼基盤の層に合わせて分ける**（`docs/design/trust.md`「信頼基盤」）。小さく監査する層（検査器、レシート検証器）と、広く使われる層（ホスト、標準箱、テスト実行器）を、同じモジュールに混ぜない
- 人間に見せるだけの処理（差分や権限サマリの表示など）は、検査器に混ぜない
- 新しいモジュールを作るときは、どの仕様の文書を実装するかを、モジュールの先頭のコメント（英語）に書く（`docs/reference/effects.md`など）

## 各ファイル・ディレクトリの補足

- **`src/main.rs`**: コマンド`saird`（まだ置いていない）。引数を解いて`src/lib.rs`の側を呼ぶだけにする（処理はライブラリに置き、テストしやすくする）
- **`tests/`**: 結合テスト（コマンドを起動するテストなど）。`env!("CARGO_BIN_EXE_<名前>")`でバイナリを起動する（`.claude/rules/testing.md`）
- **`.claude/skills/mutation-check/mutants/`**: 仕様の変異（`-spec`）の道具。`go.mod`を持つ別の Go モジュールで、cargo の対象ではない。go-template と同じもの。変えたら、そのディレクトリで整形・vet・テストを通す（Skill の「気をつけること」）
- **`src/lib.rs`**: 今は、説明のコメントだけのプレースホルダー。ビルドするものが1つも無いと`cargo`が失敗するので、空のプロジェクトでも`qsoku check`が通るように置いてある。最初のモジュールを足したら、中身に合わせて書き直す
- **`Cargo.lock`**: コミットする（依存の版を固定し、`--locked`のビルドとリリースを再現できるようにするため）
- **`target/`**: cargo のビルドの出力。コミットしない（`.gitignore`の`/target/`）。cargo-mutants の`mutants.out/`も同じ
- **`.mtqg/`**: コミットされる。中のファイルは直接編集せず、すべて`mtqg`のコマンド経由で書く（`.claude/rules/mtqg-usage.md`）。`.mtqg/.local/`はコミットされない
- **`.bcuz/`**: bcuzが`.bcuz/history.jsonl`に追記する、コマンドの実行の記録（`.claude/rules/bcuz-usage.md`）。**コミットする**（`.gitignore`の`!/.bcuz/`）。行を足すだけなので、`.gitattributes`の`merge=union`で、マージしても両側の行が残る。コミットの前に`git add .bcuz/history.jsonl`する（`git commit`は`bcuz`を通さない）。秘密をコマンドに書かない（記録に残る）。中のファイルは直接編集しない。`.srwrignore`でテープにも載せない
- **`.claude/hooks/require-bcuz.sh`**: Bashツールのコマンドを、`bcuz`を通っていなければ実行前に断る`PreToolUse`フック。判定の表は`require-bcuz.test.sh`（`bash .claude/hooks/require-bcuz.test.sh`）。判定を変えたら、テストの表も同じ変更の中で直す
- **`.mcp.json`**: `mtqg mcp`と`srwr mcp`（MCPサーバー）の配線。`mtqg init --agent claude-code`・`srwr init`が書くものと同じ。その後ろに、crubeのMCPサーバー（`crube mcp --allow-write`。`crube-ja`は`CRUBE_LANG=ja`）を手で足している（`.claude/rules/crube-usage.md`）
- **`.crube/`**: このリポジトリのファイルとディレクトリの説明（`.claude/rules/crube-usage.md`）。**コミットする**（`.gitignore`の`!/.crube/`）。rust-template から作ったときに、そのファイルの説明を両方の言語で入れてあり、コンテナを作ったあと（`.bcuz/`・`devcontainer-lock.json`・`.srwr/`ができたあと）は、そのまま`status --require all`が通る。閉じたテープ（`*.tape.jsonl.gz`）は、説明を付けず`.crube/ignore`で外す（パターンのエントリにすると、最初のテープを閉じるまで何にも当たらず`orphan`になり、無ければ新しいテープが`untracked`になるため）。`ignore`は`.gitignore`を手で写したもの（crube は`.gitignore`を読まない）
- **`.srwr/`**: 最初のhook・MCPの呼び出しで、srwrが作る（`srwr init`でも作られる。鍵`key`・`lock`・`active`・`tapes/`・`init-backup/`）。**コミットするのは閉じて圧縮したテープ`tapes/*.tape.jsonl.gz`だけ**。書き込み中の生のテープ`*.tape.jsonl`は`.gitignore`で対象外（閉じると`.gz`に置き換わり、生のままコミットすると「削除＋追加」の差分が出るため）。鍵は共有しない
- **`.srwrignore`**: テープに記録しないファイル（`.gitignore`と同じ書き方）。生成物や巨大なファイルはここに足す。`.env`・`*.pem`・`*.key`などはsrwrが元から外す
- **`.devcontainer/devcontainer-lock.json`**: コンテナのビルドで自動で作られる。**できたらコミットする**（この版のfeatureでテストなどが通った証明になる）。featureを新しくしたいときは、lockを消してビルドし直し、新しいlockをコミットする
- **VSCodeの設定**: `.devcontainer/devcontainer.json`の`customizations.vscode`に置く（拡張と設定を1か所にまとめる）。`.vscode/`は作らない
- **`.gitattributes`**: `* text=auto eol=lf`。Windowsランナーでの改行コード変換による誤検知を防ぐ（`.claude/rules/testing.md`）。`.mtqg/`の中の`.gitattributes`は`mtqg init`が置くもので、別物
- **`.gitignore`のドットファイル**: ルート直下のドットファイル・ドットディレクトリは**許可リスト方式**（`/.*`を無視し、追跡するものだけ`!`で戻す。`.claude/`の中は`/.claude/*`を無視して`settings.json`・`rules/`・`skills/`・`hooks/`だけ戻す）。サンドボックスが作業ディレクトリ直下に出すダミーファイルを`git add -A`で混ぜないため（`.claude/rules/testing.md`）。**追跡するドットファイルを新しく足すときは、`.gitignore`に`!`の行を足すこと**。足さないと`git add`が無視する
- **配布物（ビルド済みバイナリ）**: リポジトリにコミットしない。`.gitignore`には**ルート直下に限定して**書く（`/名前`）。名前だけを書くと、同名のソースディレクトリまで無視される
- **`docs/`**: Saird の仕様。直し方は`.claude/rules/documentation.md`「`docs/`は仕様」。仕様でない実装の文書を足すときは、置き場所を人間に聞く
- **シェルスクリプト**: 最小限にする。追跡中のものは`qsoku shellcheck`の対象になる（`.claude/rules/testing.md`）
