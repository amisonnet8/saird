# crubeの使い方

このリポジトリは、[crube](https://github.com/amisonnet8/crube)で、自分のファイルとディレクトリに説明を付けている（`.crube/`）。**Claude Codeは、場所を探すときも、ファイルを足す・動かす・消すときも、crubeを使う。** 説明は英語（`.crube/structure.txt`）と日本語（`.crube/structure_ja.txt`）の2つで持つ。

## 入口

| 入口 | 使い方 |
|---|---|
| MCP（`.mcp.json`の`crube`・`crube-ja`） | `find`・`tree`・`ls`・`show`・`status`と、書き込みの`set_description`・`move_description`・`remove_description`。`crube`は英語、`crube-ja`は日本語。言語はサーバーごとに1つ |
| CLI（`crube <引数>`） | `bcuz "理由" crube find …`。日本語は`env CRUBE_LANG=ja crube …` |

- `crube`は`.devcontainer/postCreate.sh`が`@latest`で入れる（ほかの道具と同じ）。`crube --version`で確かめる
- MCPのサーバーは`crube mcp --allow-write`で動く（書き込むのは`.crube/`の中だけ）。`.claude/settings.json`の`enabledMcpjsonServers`に`crube`・`crube-ja`が無いと、MCPは現れない（人間が管理する）。そのときはCLIで進める
- 1つの言語だけで足りるプロジェクトなら、使わない言語のファイルと、`.mcp.json`・`.claude/settings.json`のそのサーバー、`.claude/hooks/crube-untracked.sh`の言語の一覧を消し、この文書を直す

## 場所を探すとき

- **まず crube で引く**：`find <キーワード>`（パス・desc・detail）、`tree <ディレクトリのエントリのパス>`、`show <パス>`。目的の場所の見当をつけてから、`Grep`・`Read`で中身を読む
- `Grep`は、コードの中身（関数名・文字列）を探すときに使う。「何がどこにあるか」は crube で引く
- 引けなかった・説明が分かりにくかったときは、その場で説明を直す（下の「説明を保つ」）

## 説明を保つ

- **ファイル・ディレクトリを足す・動かす・消すときは、同じ変更の中で説明を直す**：足したら`set`（`set_description`）、動かしたら`mv`（`move_description`）、消したら`rm`（`remove_description`）
- **英語と日本語の両方を直す**。言語ごとのファイルは独立で、片方だけ直すともう片方が古くなる
- desc は1行で「何がそこにあるか」。詳しいこと（いつ使うか、ほかとの関係）は detail に書く。長さの上限は crube の文書（https://github.com/amisonnet8/crube/blob/main/docs/reference/file-format.md ）
- `.crube/`の中は、srwrの`edit`で直接直してもよい（タブのインデントを崩さない）
- コミットしないものを`.gitignore`に足したら、`.crube/ignore`にも足す（crube は`.gitignore`を読まない）
- テンプレートから作ったときの説明は、テンプレートのファイルのもの。プロジェクトの中身が決まったら（`CLAUDE.md`の「このプロジェクトについて」など）、ルートや`src/`・`docs/`の説明も合わせて直す

## 気づくための仕組み

- **srwrの`new`のあと**：フック`.claude/hooks/crube-untracked.sh`が、作ったファイルと、その上のディレクトリのうち説明の無いものを、言語ごとに知らせる（止めはしない）。知らされたら、続けて両方の言語で説明を付ける
- **コミットの前**：`bcuz "…" crube status --require all`と`bcuz "…" env CRUBE_LANG=ja crube status --require all`の両方を通す。Bashで作った・動かした・消したもの（`git rm`、ツールが書いたもの）は、フックが見ないので、ここで見つかる

## 落とし穴

- **`tree`・`set`・`rm`・`mv`に渡すのはエントリのパス**で、ディレクトリは末尾に`/`を付ける（`tree .claude/hooks/`）。付けないと`crube: no entry .claude/hooks`になる。`ls`・`show`に渡すのは実在するパス（`/`は要らない）
- crube（MCP・CLI）が書いた`.crube/`の変更は、srwrのテープには理由のない`external`として載る。それでよい（理由は、作業のテープの題とコミットメッセージに書く）
- **`.crube/`は入れ子にできない。** 上のディレクトリに`.crube/`があると、crube はそちらを使う（`init`も「もうある」で止まる）。別のリポジトリの中にこのリポジトリを置いて作業しない
