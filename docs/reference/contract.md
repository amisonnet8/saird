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
- 型の骨格は WIT(Wasm Component Model のインターフェース記述言語)に対応させる(下の「WIT との対応」)。制約は、ホストが入口と出口で検査する。
- 契約に書かないもの:アルゴリズム、実装言語、使った部品、資源上限、性能目標。

## WIT との対応

箱の型の骨格は、次の WIT とする。

- パッケージは `saird-box:<箱の名前>@<版>.0.0`、world は `<箱の名前>` とする。world は関数 `call` を1つ export する。
- 入力は、宣言した順に `call` の引数とする。
- 戻り値は、`errors` があれば `result<ok の型, error>`、無ければ ok の型とする。`error` は、宣言したエラーを宣言した順に並べた variant とする。フィールドのあるエラーは、そのフィールドを持つ record を持つ。エラーのフィールドの型は、その値の式の型とする。

| 型 | WIT |
|---|---|
| `int` | `s64` |
| `string` | `string` |
| `bool` | `bool` |
| `timestamp` | `record timestamp { seconds: s64, nanoseconds: u32 }`(UTC の `1970-01-01T00:00:00Z` からの秒と、0 以上 10^9 未満のナノ秒) |
| `list<T>` | `list<T>` |
| `{ ... }` | `record` |

名前は次のように移す。

- フィールドの名前は、`_` を `-` にする(`order_id` は `order-id`)。先頭か末尾の `_`、続いた `__`、数字で始まる語を含む名前は、検査器が拒否する。
- 箱とエラーの名前は、大文字の前で区切って小文字にする(`ReserveStock` は `reserve-stock`)。
- WIT の予約語になる名前には `%` を付ける(`type` は `%type`)。
- 名前の無い record は、欄とパスから名付ける。入力のフィールド `x` の record は `in-x`、ok の record は `out-ok`、kv の値の record は `kv` とする。その中のフィールド `y` の record は、親の名前に `-y` を足す(`in-x-y`)。`list` は名前に何も足さない(`items: list<{ ... }>` の要素は `in-items`)。エラー `E` の record は `error-` に `E` の名前を足す。

関連:[文法](syntax.md)、[effects](effects.md)、[生成テスト](generated-tests.md)、[権限](permissions.md)。例は [ReserveStock](../examples/reserve-stock.md)。
