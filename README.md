# Saird

Saird は AI 専用プログラミング言語である。

> 人間はコードではなく、契約と証拠を信頼する。実装は外に出られない箱に閉じ込め、確かめるのは信頼基盤に任せる。

このリポジトリには、Saird の仕様と、その記述形式を検査し実行するツール群の Rust による実装がある。

## 状態

- 仕様：**v0.5.3**。[docs/](docs/design/overview.md)
- 実装：まだ無い

## 文書

入口は [中心となる考え方](docs/design/overview.md)、用語は [用語集](docs/reference/glossary.md) にある。文書は今は日本語だけで、公開の前に英語版を加える。

## 開発

devcontainer（`.devcontainer/`）を開くと、Rust と開発の道具が入る。ビルド・テスト・lint は `qsokufile` の名前で動かす。

```
qsoku check
```

## ライセンス

MIT（[LICENSE](LICENSE)）
