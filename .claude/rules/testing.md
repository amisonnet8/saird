# テスト方針

## 開発環境の前提

開発者の手元環境は**Linux（devcontainer）のみ**。Windows・macOSでの動作確認は手元では行わず、GitHub Actionsのホステッドランナーに委ねる（`.github/workflows/ci.yml`の3OSマトリクス）。

## 実装後の動作確認

実装したら、ビルド確認に加えて実際の動作確認を行うこと。ロジック上正しそうに見えても、動かして初めて見つかる不具合はある。

- `qsoku check`（rustfmt の差・clippy・テスト。整形の差異は`lint`が報告する）は作業の区切りで必ず通す
- 依存を足したら`qsoku deny`（既知の脆弱性・ライセンス・出どころ）を通す
- シェルスクリプトを足したら`qsoku shellcheck`を通す（`git add`してから走らせる。追跡中のものだけが対象）
- `.claude/hooks/require-bcuz.sh`を変えたら`qsoku hook`（判定の表）を通す。判定を変えたら、表も同じ変更の中で直す

## Rustのテスト

- **期待値は、テストを書く前に決める。** 推測で埋めて、落ちてから直す、をしない
  - **計算で決まる値**（ハッシュ・表示幅・日時の書式など）は、実装とは別の道具（`sha256sum`など）で求めて書く。実装の出力を写すだけでは、実装の誤りを固定してしまう
  - **動かして決まる値**（走査・照合の結果など）は、先に実装を動かして出力を読み、1件ずつ正しいと確かめてから貼る
  - **件数より中身で比べる**（名前の一覧など）。件数は数え間違えても気づきにくく、落ちたときに何が違うか分からない
  - テストが落ちたときは、実装と期待値のどちらが誤りかを先に確かめる
  - 「〇〇と同じ書式・動き」と仕様にあるものは、その本家を道具として使って期待値を求める（gitignore の書式なら`git check-ignore`。パスに末尾の`/`を付けて聞くと、ディレクトリそのものの判定が本来と変わることがある。付けずに聞く）
- **表の形の行（行末のコメント、配列の要素）を手で桁揃えしない。** rustfmt が揃え方を決めるので、`qsoku lint`（`cargo fmt --check`）が落ちる。書いたら`qsoku fmt`で整形してから`qsoku check`する（整形ツールが書いた変更は、テープに理由なしで載るが、それでよい）
- **テストの中で環境変数を変えない。** `std::env::set_var`は edition 2024 で`unsafe`になり（`Cargo.toml`の`unsafe_code = "forbid"`で書けない）、同じプロセスで並んで走るほかのテストにも効く。環境に左右される処理は、環境を引数で受ける形にするか、子プロセスに`Command::env`で渡す
- **gitを使うテストは、開発者の設定の影響を受けないようにする。** 子プロセスの`git`に、`Command::env`で`HOME`と`GIT_CONFIG_GLOBAL`を一時ディレクトリに向けて渡し、`user.name`・`user.email`はテストの中で決める
- **コマンドを起動するテスト（`tests/`）は、`env!("CARGO_BIN_EXE_<名前>")`でそのバイナリのパスを得る。** cargo がテストの前にビルドし、Windows の`.exe`も付いたパスになる。自分で`cargo build`して探さない
- **Rustのソースに、見えない文字（ゼロ幅、U+2028・U+2029、結合文字、文字の向きを変える制御文字、そのほかの制御文字）をそのまま書かない。** 書き込みの途中で本物の文字に置き換わると、見た目では気づけない不具合になる
  - **波かっこつきのエスケープ`\u{2028}`で書く**。何の文字かは、コメントかテストの名前に残す
  - srwrの`edit`・`new`の引数はJSONで渡るので、エスケープが引数の段階で本物の文字になって届くことがある（Go の`\u2028`の形で実測。U+2028・U+2029・U+0301・U+200D）。書いたあとに`grep -P '[\x{2028}\x{2029}\x{200D}\x{0301}]'`で、本物の文字が無いことを確かめる。clippy の`invisible_characters`が見つけるのは一部の文字だけ

## 壊して確かめる（mutation-check）

通るだけのテストは、効いているとは限らない。実装を1か所ずつ壊して、テストが落ちるかを見る。手順・道具（構文の変異は`cargo mutants`、仕様の変異は`mutants/`の`-spec`）・仕様の変異の一覧（`spec.txt`）は、Skill`mutation-check`（`.claude/skills/mutation-check/`）にある。生き残った変異は、握りつぶさずに読み、テストを足すか、同じ意味だと理由を書いて残す。

- **作業ツリーではなく、写しで壊す。** `cargo mutants`は自分で写しを作る。仕様の変異は`$TMPDIR`に写しを作り（書き方は Skill の手順1）、その中で変異を入れる。作業ツリーをBashで書き換えると、srwrのテープに理由のない`external`が残り、戻し忘れの心配もある。足したテストは、写しへコピーしてから同じ変異をもう一度入れ、落ちることを確かめる
- **構文の変異だけで済ませない。** `cargo mutants`は境目（`>=`→`>`）の変異を作らないことがある。境目の約束は`spec.txt`に書く
- **見落としやすい形**：整える処理（書き出しの整形など）を通して比べるテストは、手前の誤り（読み込みで空白を残す、など）を隠す。整える処理がもう一度直すため。読み込んだ結果そのものを比べる。期待値が偶然一致することもある（境目ちょうどの例を足す）
- **意味が同じなので残した変異**：（プロジェクトで、残したものと理由を足す。例：エラーを返すときに添える値、ありえない入力の境界）

## cargo-deny：既知の脆弱性・ライセンス・出どころ

- `cargo deny check`で、`Cargo.lock`の依存の既知の脆弱性（RustSec）・ライセンス・禁止した crate・出どころ（crates.io だけ）を検査する（`qsoku deny`）
- 何を失敗とみなすか（許すライセンス、無視する勧告）は`deny.toml`で固定する。許すライセンスを足すときは、`about.toml`の`accepted`も同じにする（`.claude/rules/distribution.md`）
- `cargo add`で依存を足したら、必ず`qsoku deny`を通す
- RustSec のデータベースの取得に通信が要る（`github.com`。devcontainerのBashサンドボックスを使う場合は下記「Bashサンドボックスの落とし穴」参照）

## ShellCheck：シェルスクリプト

- シェルスクリプトは最小限にする。込み入った処理はRustで書く
- `qsoku shellcheck`は`git ls-files '*.sh' '*.bash'`で追跡中のものを列挙してShellCheckにかける
- シェルスクリプト中のコメントを、行頭が小文字の`# shellcheck`で始まる文にしてはいけない（ShellCheckがインラインディレクティブとして誤解釈し、SC1072/SC1073でパースエラーになる）。「ShellCheck」のように大文字を混ぜて書く
- **`postCreate.sh`の中で`. /etc/os-release`をsourceしない。** ShellCheckのSC1091（追えない形で指定されたファイル）が情報の水準で報告され、終了コードが0でなくなる。ディストリビューションの版は`lsb_release -rs`で取る
- `# shellcheck disable=…`は、`case`の枝の前には置けない（SC1124 のパースエラー）。`case`の文の前に置くと、全部の枝に効く

## シェル向けのもの：補完のスクリプト・シェルとの連携

- devcontainer のシェルは、`postCreate.sh`の`shells`で決める。初めは`(bash)`だけ
- 補完のスクリプトなど、自分のシェル向けのものを配るプロジェクトは、**本物のシェルで動かして確かめる**。`shells`を`(bash zsh fish pwsh)`にし、`devcontainer.json`の`features`に`"ghcr.io/devcontainers/features/powershell:2.0.3": {}`を足して、コンテナを作り直す（bash・zsh はベースイメージ、fish は`postCreate.sh`の apt、pwsh は feature）。`pwsh`を挙げて feature が無いと、`postCreate.sh`が止まる
- テストは、入っていないシェルを飛ばす（CI のランナーにあるのは bash と`pwsh`）。飛んだときは、コンテナを作り直したかを確かめる。何を候補にするかは単体テストで見て、本物のシェルのテストは「スクリプトがシェルとつながること」だけを見る

## Bashサンドボックスの落とし穴

> devcontainer内でClaude Codeが実行するBashコマンドは、既定でOSレベルのサンドボックス（Linux bubblewrap）にかかる。`.claude/settings.json`の`sandbox`でファイルシステムの書き込み先とネットワーク接続先を許可リストで絞っている。

- **cargo は`/usr/local/cargo`（crate の取得とキャッシュ、cargo-deny のデータベース）と`/usr/local/rustup`（`rust-toolchain.toml`のツールチェイン）に書き込む**（Rust のイメージの`CARGO_HOME`・`RUSTUP_HOME`）。`filesystem.allowWrite`にこれが無いと、最初の`cargo`が`read-only file system`で落ちる。接続先は、crate が`index.crates.io`・`static.crates.io`（と`crates.io`）、ツールチェインが`static.rust-lang.org`、RustSec のデータベースが`github.com`
- **zig と cargo-zigbuild は`~/.cache/zig`・`~/.cache/cargo-zigbuild`に書き込む**（`qsoku snapshot`）。`allowWrite`にこれが無いと、`~/.cache`は読み取り専用なので落ちる
- **`allowWrite`に書いたパスは、ディレクトリが既に存在しないと効かない。** 新しいコンテナには`~/.cache/zig`などが無く、`~/.cache`自体は読み取り専用なので、作られないまま書けずに失敗する。`postCreate.sh`が`.claude/settings.json`の`allowWrite`を`jq`で読み、`~/`と絶対パス（`/go`など。`/dev`・`/proc`・`/sys`と相対パスは除く。作れなければ警告だけ）を`mkdir -p`して先に作る（`allowWrite`を変えても`postCreate.sh`は直さなくてよい）。**すでにあるコンテナを直すには、サンドボックスの外で該当のディレクトリを`mkdir -p`する**
- **Linuxでは`bubblewrap`（`bwrap`）と`socat`が無いと、`sandbox.enabled: true`でも黙って無効のままになる。** エラーも出ないので、サンドボックスが効いているつもりで効いていない状態になる。`postCreate.sh`が、無いものだけ`apt`で入れる（ベースイメージが持っていれば何もしない）
- **道具のリリースのタグが打たれた直後にコンテナを作り直すと、前の版が入ることがある**（srwr の v0.3.0 で起きた）。Go のモジュールプロキシは`@latest`の答えをしばらく覚えているので、`postCreate.sh`の`go install …@latest`が前の版を入れ、その版に無い補完も警告を出して飛ばす。作り直したら、新しく出た道具の`--version`を見る。古ければ`go install github.com/amisonnet8/<道具>/cmd/<道具>@<新しい版>`で入れ直し（`proxy.golang.org`・`sum.golang.org`・`storage.googleapis.com`への接続が要る）、補完は人間のターミナルで入れ直す（書き先がサンドボックスの外）
- **許可リストに無いものに当たったら、自分で回り道を探し続けない。** `.claude/rules/working-with-human.md` 2章の形で、何をどこに足すかを短く頼む
- **`git fetch`・`git pull`と`gh pr create`には`github.com`・`api.github.com`への許可が要る**
- **`sudo`は、サンドボックスの中では承認しても失敗する。** サンドボックスのuidの付け替え（特権の無いユーザー名前空間）で、本来rootが持つファイルがnobodyの持ち物に見え、`sudo`自身の所有者の確認に落ちる。`sudo`が要るときは、人間のターミナルで動かすよう頼む（`.claude/rules/working-with-human.md` 2章）
- **`srwr init`は、書き換えるものがあるときは、サンドボックスの中では`.mcp.json`・`.claude/settings.json`を置き換えられない（書き換えが無ければ動く。そもそもテンプレートから作ったプロジェクトでは動かさなくてよい）**（一時ファイルを`rename`する先が書き込み保護で、`device or resource busy`。`.srwr/`の鍵だけが先にできる）。人間のターミナルで動かす（`.claude/rules/srwr-usage.md`「落とし穴」）。`srwr view`は`~/.cache/srwr`に書くので、同じく人間のターミナルで動かす
- **サンドボックスの書き込み保護が、作業ディレクトリ直下に`.bashrc`・`.gitconfig`などのダミーファイルを出現させることがある。** `git status`に大量の未追跡ファイルとして見えて驚くが、実害はない（未追跡のままなのでコミットには影響しない）。これはサンドボックス内のBashからだけ見える見かけ上のファイル（`/dev/null`のbind mount）で、サンドボックスの外の人間側には存在しない。`.gitignore`はルート直下のドットファイルを許可リスト方式にしてあり、`git add -A`で混ざらない

## GitHub Actions CIの落とし穴

- **`windows-latest`で`qsoku`を`go install`しても、内部で呼ぶ`sh`がそのままPATHに乗るとは限らない。** `qsokufile`のコマンドは常に`sh`（Git Bash付属）で実行される。CIの`run:`ステップは`shell: bash`を明示し、Git Bashが通ったPATHでqsokuを実行する
- **`/dev/stderr`等のUnix固有のパスはWindows（Git Bash）で壊れる。** `tee /dev/stderr`などを使わず、標準のリダイレクトだけで書く
- **Windowsのcheckoutで改行がCRLFになると、`cargo fmt --check`が全ファイルを未整形と誤検知する。** ルートの`.gitattributes`（`* text=auto eol=lf`）で防ぐ
- **`uses: owner/repo@TAG`はタグ名と厳密に一致しないと失敗する。** `v`の有無を見落としやすい。書く前に実際のタグ名を確かめる
- **Windowsには実行ビットの概念がない。** 実行ビットを確かめるテストは`#[cfg(unix)]`にする（`std::os::unix::fs::PermissionsExt`も Unix だけ）
- **実行権限の無いファイルで「起動できない」を作るテストは、Windowsでは「見つからない」になる。** 拡張子の無い名前（`script`・`build.sh`）は`PATHEXT`に合わず、コマンドとして探されないため。Windowsでは`.exe`の名前で、中身が実行形式でないファイルにすると、起動に失敗する
- **出力に出るパスと比べるテストは、一時ディレクトリを解決してから比べる。** 同じディレクトリでも、macOSは`/var`→`/private/var`のシンボリックリンク、Windowsは8.3の短縮名（`RUNNER~1`）と長い名前が食い違う。ただし Windows の`std::fs::canonicalize`は`\\?\C:\…`の形を返すので、そのまま比べると今度は食い違う。比べる両方を同じ方法で解決する
- **Windowsでは、開いたままのファイルを置き換え（`rename`）られないことがあり、開いたハンドルのあるディレクトリは消せない。** 一時ファイル経由の置き換えや、失敗したときの後片付け（`std::fs::remove_dir_all`）の前に、先にファイルを閉じる（`drop`する）。Windowsのランナーでだけ落ちる
- **Windowsのシンボリックリンクは、ファイルとディレクトリで作る関数が違い（`std::os::windows::fs::symlink_file`・`symlink_dir`）、宛先の`/`は`\`で保存される。** 宛先を見せる・比べるときは区切りを`/`にそろえる。テストの宛先は、区切りを含むもの（`src/auth`）にする。区切りの無い宛先（`src`）では、この食い違いを捕まえられない（crube の`show`の`link`で、CI の Windows でだけ落ちた）
- **OSで分かれる動き（区切りの`\`など）は、手元のLinuxのテストからも確かめる。** `std::path::MAIN_SEPARATOR`のようなOSの値を直接使わず、引数で受ける形にし、テストでWindowsの値を渡す。CIのWindowsまで待たずに、手元で落ちる
