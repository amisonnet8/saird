# qsokuの使い方

ビルド・テスト・lintなどの決まった作業を、リポジトリ直下の`qsokufile`に**名前を付けて**置き、`qsoku <名前>`で呼ぶツール。Makefileの代わり。手元（devcontainer）とCIで、同じ名前・同じコマンドで動かすために使う。**`qsokufile`に定義した作業は、`cargo`などを直接打たず、`qsoku <名前>`で実行する。**

## このプロジェクトの名前

| 名前 | 内容 | いつ使うか |
|---|---|---|
| `qsoku build` | `cargo build --all-targets` | 編集後のビルド確認（Rustのファイルの編集後は、`.claude/hooks/build.sh`が自動で呼ぶ） |
| `qsoku fmt` | `cargo fmt --all` | 整形を書き直したいとき |
| `qsoku lint` | `cargo fmt --all --check`と`cargo clippy --all-targets --all-features -- -D warnings` | |
| `qsoku unit` | `cargo test --all-features`（ライブラリ・コマンド・`tests/`・doctest。`--all-targets`は doctest を飛ばすので付けない） | |
| `qsoku check` | `lint`・`unit`をまとめて実行 | **作業の区切りで必ず通す**（`.claude/rules/testing.md`） |
| `qsoku deny` | `cargo deny check`（既知の脆弱性・ライセンス・出どころ。`deny.toml`） | 依存を足したとき |
| `qsoku shellcheck` | 追跡中の`*.sh`・`*.bash`をShellCheckにかける | シェルスクリプトを足したとき |
| `qsoku hook` | `.claude/hooks/require-bcuz.test.sh`（bcuz を強制するフックの判定の表。`jq`が要る） | `require-bcuz.sh`を変えたとき |
| `qsoku snapshot` | `goreleaser check`のあと、タグなしで配布物を`dist/`に作る（`.goreleaser.yaml`。goreleaser・zig・cargo-zigbuild・cargo-about が要る） | タグを打つ前、`.goreleaser.yaml`・`about.toml`を変えたとき。中身を見てからタグを打つ（`.claude/rules/distribution.md`） |

**`qsoku cross`（最初は無効）**：Windows・macOSのターゲットで`cargo clippy`を動かす。`qsokufile`にコメントで置いてある。**OSごとのコード（`#[cfg(windows)]`・`#[cfg(unix)]`）を足したら**、行頭の`# `を外して有効にし、`check`に`&& qsoku cross`を足し、上の表にも足す。ターゲットは最初に一度`rustup target add`が要る。Linuxだけで動かしていると、Windows向けの枝が壊れていてもCIのWindowsで初めて落ちる

一覧は、今ある`qsokufile`の内容そのもの。`qsoku .list`（名前とコマンド）か`qsoku .names`（名前だけ）で確かめる。**引数なしで`qsoku`とだけ打っても、使い方と定義済みの名前が出る。**

## 呼び方

```
qsoku <名前> [引数...]
```

- **Claude Codeは、Bashツールでは`bcuz`を通して呼ぶ**：`bcuz "理由" qsoku <名前>`（`.claude/rules/bcuz-usage.md`）。以下の例・表の`qsoku <名前>`は、素の形で書いてある
- 引数は、項目のコマンドの中で`"$@"`・`$1`として受けられる。**今の`qsokufile`の項目は、どれも引数を受けていない**（書いていない引数は捨てられる）。受けたいときは、項目を`unit: (cd //; cargo test "$@")`のように書き換える。そうすれば`qsoku unit parser::`（名前で絞る）が使える
- **qsoku自身のオプションは無い。** `--help`・`--version`は**名前として調べられる**ので、「定義されていない」というエラーになる。使い方は`qsoku .help`、バージョンは`qsoku .version`
- コマンドは、どのOSでも、また呼び出し元がどのシェルでも、常に`sh`で実行される
- 標準入出力・標準エラー出力はそのまま素通し。終了コードは、コマンドのものがそのまま返る

### 終了コード

| コード | 意味 |
|---|---|
| 0 | 成功 |
| 1 | qsokuが頼まれたことをできなかった（`qsokufile`が無い・解析できない、`sh`を起動できない、など） |
| 2 | コマンドラインの誤り（**定義されていない名前**、管理用コマンドの引数の数の誤り） |
| それ以外 | 項目のコマンドが実行された後は、そのコマンドの終了コード（qsokuが差し替えることは無い） |

## 管理用コマンド

`.`で始まる。`qsokufile`の名前は`.`で始められないので、名前とぶつからない。

| コマンド | 内容 |
|---|---|
| `qsoku .list` | 定義されている名前とコマンドを、ファイルの順に出す |
| `qsoku .names` | 名前だけを出す（補完が使う。決して失敗しない） |
| `qsoku .add <名前> <コマンド>` | 項目を足す。**すでにある名前なら、その行をその場で置き換える**（コメントや並び順はそのまま） |
| `qsoku .rm <名前>` | 項目を消す。無い名前はエラー |
| `qsoku .edit` | `$EDITOR`で`qsokufile`を開く |
| `qsoku .where` | `qsokufile`のあるディレクトリを出す |
| `qsoku .init` | カレントディレクトリに空の`qsokufile`を作る |
| `qsoku .version` | qsokuのバージョン |
| `qsoku .shell <shell>` | シェル連携のコードを出す（下記） |
| `qsoku .help` | 使い方と、定義済みの名前 |

## `qsokufile`の書き方

```
# 行頭が#の行はコメント。
build: (cd //; cargo build --all-targets)
unit: (cd //; cargo test "$@")
check: qsoku lint && qsoku unit
```

- **1行が1項目**：`名前: コマンド`。最初の`:`が区切り。**行の継続は無い**（1項目は必ず1行）
- コメントは**行頭が`#`の行だけ**。行の途中・末尾の`#`はqsokuには特別な意味が無く、`sh`がシェルのコメントとして読む
- 名前に使えるのは、英数字（ASCII）・`.`・`_`・`-`。**先頭を`.`にはできない**。大文字・小文字は区別される
- **同じ名前を2回定義するとエラー**（両方の行番号が出る）。何も実行されない
- 別の項目を呼ぶときは、`qsoku <名前>`と書く（`check`の例）
- **項目の中で`exit`を呼ばない。** qsokuが同じスクリプトの末尾に後始末の行を足すので、`exit`があるとそこで終わって届かない。成否は、項目の最後のコマンドの終了コードに任せる

### `//`：`qsokufile`のあるディレクトリ

`//`は、使われている`qsokufile`のあるディレクトリ（環境変数`QSOKU_ROOT`）に置き換わる。**どのディレクトリから`qsoku`を呼んでも、リポジトリ直下で動かすために**、項目は`(cd //; ...)`の形で書く。

- 置き換わるのは、**引用符の外で、単語の先頭**にあるとき（コマンドの先頭、または空白・`;`・`&`・`|`・`(`の直後）
- 置き換わらない：`http://example.com`（`:`の後）、`"//"`・`'//'`（引用符の中）、コメントの中
- **括弧で囲む**（`(cd //; ...)`）と、`cd`が子プロセスの中に閉じるので、呼び出し元のシェルの居場所は動かない。括弧なしの`cd //src && ...`は、呼び出し元のシェルもそこへ動く（シェル連携が居場所を持ち帰るため）
- 引用符の中で場所を使いたいときは、`$QSOKU_ROOT`を直接書く

### 探し方

`qsokufile`は、カレントディレクトリから**親へ順にたどって最初に見つかったもの**が使われる。gitリポジトリの境界では止まらない。サブディレクトリからでも同じ`qsokufile`が使える。見つからなければ、エラー（終了コード1）で`qsoku .init`を案内する。

## 名前を足す・変える

- 新しい作業は、`qsoku .add <名前> '<コマンド>'`で足すか、`qsokufile`を直接編集する。**足したら、上の「このプロジェクトの名前」の表と、`.claude/rules/testing.md`の該当箇所も同じ変更の中で直す**
- CIで走らせたい作業は、`.github/workflows/ci.yml`の`run: qsoku <名前>`に書く（手元とCIでコマンドが食い違わない）
- 例：`#[ignore]`を付けた遅いテストを、別の名前で回す

  ```
  slow: (cd //; cargo test --all-targets -- --ignored)
  ```

## シェル連携

`.devcontainer/postCreate.sh`が、`~/.bashrc`に次の1行を足す。

```
eval "$(qsoku .shell bash)"
```

これで、次の2つが入る。

- **`qsoku`というシェル関数**：項目が`cd`した居場所を、呼び出し元のシェルへ持ち帰る。関数の中は`command qsoku`で本物のバイナリを呼ぶ
- **TAB補完**：`qsoku `の後でTABを押すと、その場で`qsoku .names`を呼んで、今ある`qsokufile`の名前を出す。管理用コマンド（`.add`など）は、シェルのスクリプトに固定で入っている

## OSごとの注意

- **Windows**：`qsokufile`のコマンドは、Git for WindowsのMSYS `sh.exe`で動く。`Git\bin`（`C:\Program Files\Git\bin`）を`PATH`に足すこと。CIの`run:`は`shell: bash`を明示する（`.claude/rules/testing.md`「GitHub Actions CIの落とし穴」）
- **Git Bash（Windows）**：引数が`//`で始まると、MSYSが先に書き換えてしまい、qsokuに届くのは`/`1つになる。`MSYS_NO_PATHCONV=1`で止められる
