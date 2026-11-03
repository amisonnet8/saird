# 配布方法

## このプロジェクトの配布

- **コマンド`saird`を、GitHub Releases で配る**（下の goreleaser の流れ。6つの OS・アーキテクチャ）。crates.io には出さない（`Cargo.toml`の`workspace.package`の`publish = false`。どの crate も）
- **実装の版と仕様の版は別のもの。** 実装の版は`Cargo.toml`の`workspace.package`の`version`（全部の crate で共通）とタグ（`vX.Y.Z`）。仕様の版は今 v0.5.3（`CLAUDE.md`）。コマンドがどの仕様の版を実装しているかを見せるかどうか・その見せ方は、決めるときに人間に聞く（利用者の目に触れる見え方のため）

## 配布物の作り方（goreleaser）

実行ファイルは、`.goreleaser.yaml`・`.github/workflows/release.yml`・`about.toml`・`about.hbs`で配る（rust-template から入れたもの）。Go のテンプレート（go-template）と同じ流れで、ビルドだけが goreleaser の rust の builder（`cargo zigbuild`）になる。

- **始めるとき**：`.goreleaser.yaml`の`project_name`・`binary`と、コマンドの crate（`crates/saird/Cargo.toml`）の`name`は、もう`saird`にしてある。ルートが workspace なので、goreleaser の`flags`に`--package=saird`が要る（無いと goreleaser が止まる。実行ファイルはルートの`target/`から取る）。`crates/saird/src/main.rs`を置くまでは、`qsoku snapshot`とリリースは、作るものが無く失敗する（`goreleaser check`は、GitHub の remote があるリポジトリなら通る）
- **1台の Linux から、全部のターゲットを作る**：`cargo zigbuild`（cargo-zigbuild と、リンカーの zig）で、`x86_64`・`aarch64`の linux（musl。システムの libc に頼らない静的なバイナリ）・apple-darwin・windows（`x86_64-pc-windows-gnu`・`aarch64-pc-windows-gnullvm`）。ターゲットは`rustup target add`で足す（`release.yml`がする。手元の`qsoku snapshot`は、足していないと rustup がコマンドを示して止まる）。C のライブラリに頼る crate（`-sys`）は、ターゲットごとのビルドが通るかを`qsoku snapshot`で確かめてから足す
- 配布物は linux・darwin・windows の amd64・arm64。linux・darwin は`.tar.gz`、windows は`.zip`。名前は`<名前>_<版>_<OS>_<アーキテクチャ>`（版は`v`なし）。`checksums.txt`を添える。アーカイブには、実行ファイル・`LICENSE`・`README.md`・`licenses/`（依存のライセンス文）を入れる
- **版**：コマンドが出す版は`Cargo.toml`の`workspace.package`の`version`（`release.yml`は`cargo metadata`で package `saird`の版を読む。`env!("CARGO_PKG_VERSION")`）。**タグから`v`を除いたものと同じにする**。`release.yml`の最初のジョブが、タグと`Cargo.toml`の版を比べ、違えば何も作らずに止める（長い検査の前に）。タグを打つ前に、`Cargo.toml`の`version`を次の版にし、`Cargo.lock`も合わせて（`qsoku build`で書き直される）コミットする。`qsoku snapshot`の版は`0.0.1-snapshot`の形でも、バイナリが出すのは`Cargo.toml`の版のまま
- **依存のライセンス文**を配布物に同梱する（goreleaser の`before`の手順）。`licenses/`には次の2つが入る。`about.toml`に無いライセンスの crate があれば失敗し、リリースが止まる
  - `THIRD_PARTY_LICENSES.txt`：cargo-about（`about.toml`・`about.hbs`）が、配布するターゲットで実行ファイルに組み込まれる crate を求め、ライセンス文を書き出したもの
  - `RUST_STD_COPYRIGHT.html`：Rust の標準ライブラリの著作権表示（ツールチェインの`share/doc/rust/COPYRIGHT-library.html`）
  - 依存を足すときは、そのライセンスが`deny.toml`の`allow`と`about.toml`の`accepted`にあるかを確かめる（`qsoku deny`も通す）。足すなら両方に足す（人間に聞く判断）

## リリースの流れ

ビルド済みのバイナリなどを GitHub Releases で配るときは、次の流れにする。どのプロジェクトも同じ。

1. **人間が**タグ（`v0.1.0`の形）を打って push する。人間がするのはこれだけ
2. タグで動く workflow（`.github/workflows/release.yml`）が、検査を通してから配布物を作り、Release に添付して**公開する**（本文は goreleaser が前のタグからのコミットで作る）

- **Claude Codeは、タグも Release も作らない。** `gh release create`は`.claude/settings.json`で拒否（deny）されている。Release を作るのは workflow の中だけ
- `release.yml`は、まず`ci.yml`をそのまま呼んで（`workflow_call`）普段の CI の検査をすべて通し、そのあと goreleaser（`.goreleaser.yaml`。`goreleaser-action`の`version: "~> v2"`で、v2 の最新）で配布物を作る。書き込みの権限（`contents: write`）は、Release を作るジョブだけに与える
- goreleaser はいったん**下書き**に添付し、すべて添付し終えてから、最後の手順（`gh release edit --draft=false`）で公開する。途中で落ちると下書きだけが残り、公開された Release に配布物が欠けることはない。同じタグでもう一度動かすと、その下書きを置き換える（`replace_existing_draft`）。落ちたときは、原因を直したコミットに新しいタグを打つか、同じコミットのままなら workflow を再実行する
- **公開済みの Release があるタグでは、何も変えずに失敗する**（最初の手順）。出し直すときは、新しいタグを打つ
- **タグを打つ前に、手元で`qsoku snapshot`を動かして中身を見る**。タグなしで`dist/`に同じ配布物を作る（版は`0.0.1-snapshot`の形）。goreleaser・zig・cargo-zigbuild・cargo-about は`.devcontainer/postCreate.sh`が入れる。コンテナを作り直す前に要るときは、`GOTOOLCHAIN=auto go install github.com/goreleaser/goreleaser/v2@latest`（goreleaser がコンテナより新しい Go を求めるとき、その Go を取ってきてビルドする。Bashサンドボックスの中では、`proxy.golang.org`と`storage.googleapis.com`への接続が要る）。`.goreleaser.yaml`・`about.toml`を変えたら`qsoku snapshot`で確かめる
- ビルド済みのバイナリは、リポジトリにコミットしない（`.claude/rules/directory-structure.md`「配布物」）

## VS Code の拡張を配るとき

VS Code の拡張（`.vsix`）を持つプロジェクトも、同じ流れにする。

- **拡張の版（`package.json`の`version`）は、タグから`v`を除いたものと同じにする**。Rust のコマンドも配るプロジェクトでは、コマンドと拡張を同じ版で出す（Release に版の違う`.vsix`が並ぶと、どれが対か分からない）。拡張が変わらない版でも上げる。タグを打つ前に、`package.json`と`package-lock.json`（2か所）を次の版にしてコミットする
- `release.yml`の**最初のジョブ**で、タグと`package.json`の版を比べ、違えば何も作らずに止める（長い検査の前に。`jq -r .version`。見本は srwr。`Cargo.toml`の版を比べる手順と同じジョブに並べる）

- **Rust のコマンドも配るプロジェクト**（goreleaser を使う）：`.vsix`は goreleaser の`before`の手順で作り（`bin/`の下へ）、`.goreleaser.yaml`の`release.extra_files`で同じ下書きに添付し、`checksum.extra_files`で`checksums.txt`にも入れる。`release.yml`は template のまま（`before`の手順が使う道具を入れる手順だけ足す）。`qsoku snapshot`でも`.vsix`ができる。見本は srwr（Go のプロジェクト）
- **拡張だけのプロジェクト**（goreleaser を使わない）：`release.yml`で、公開済みならば失敗する最初の手順と`ci.yml`の検査のあと、`.vsix`を作り、`gh release create --draft`で下書きを作って添付し（本文は`CHANGELOG.md`のその版の節）、最後に公開する
- **Marketplace への公開は、人間が手で行う**（Marketplace の Web から`.vsix`を上げる。トークンを用意していないので、workflow からは公開しない）。GitHub Release に添付された`.vsix`を使う
