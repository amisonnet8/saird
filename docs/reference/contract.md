# 契約

## 4つの欄

| 欄 | 書くこと |
|---|---|
| **入力(in)** | 型と制約(事前条件)。すべての値に上限を付ける |
| **出力(out)** | 正常時の型、`ensures`(成功時に成り立つ性質)、`errors`(各エラーが起きる条件 `when`。上から順に判定する) |
| **権限(needs)** | 使う標準箱、その範囲と操作、effects。書いていないものは禁止する |
| **テスト(tests)** | 例(example)と、生成テストの設定(generate) |

- 振る舞いは `ensures` と `errors … when` で宣言する。これらは機械で検査する。summary は説明であり、保証の対象ではない。
- `ensures` と `when` には、入力、出力、`kv_before` / `kv_after`(箱の kv 範囲の呼び出し前後の状態)、`clock`(呼び出し時の時刻)を使える。
- **存在しないキー**:`kv[x]` は、キーが存在しなければ `none` になる。`none` を含む比較は偽、算術は `none` になる。キーの有無は `x in kv` / `x not in kv` で判定する。
- **エラーの選択**:`errors` は上から順に `when` を評価し、最初に成り立ったエラーを返す。`any item in items:` の `item` は、`items` を先頭から調べて最初に条件を満たした要素とし、その値をエラーのフィールドに入れる。
- 型の骨格は WIT(Wasm Component Model のインターフェース記述言語)に対応させる。制約は、ホストが入口と出口で検査する。
- 契約に書かないもの:アルゴリズム、実装言語、使った部品、資源上限、性能目標。

関連:[文法](syntax.md)、[effects](effects.md)、[生成テスト](generated-tests.md)、[権限](permissions.md)。例は [ReserveStock](../examples/reserve-stock.md)。
