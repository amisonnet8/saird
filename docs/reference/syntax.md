# 文法

契約と組立書の書き方を定める。意味は [契約](contract.md)、[権限](permissions.md)、[組立書と呼び出し](assembly.md) を参照。

## 表記

EBNF で書く。`BLOCK{ x }` は、`:` で終わる行のあとに、1段深く字下げした `x` を1つ以上並べたものを表す。

## 行と字下げ

- 同じブロックの行は、同じ深さに字下げする。
- `#` から行末までは注釈とする(文字列の中を除く)。
- 次の行は、前の行の続きとする。
  - 括弧(`( )`・`[ ]`・`{ }`・`< >`)が閉じていないときの次の行
  - 前の行より深く字下げした行のうち、前の行が(注釈を除いて) `:` で終わらないもの

## 契約

```ebnf
contract    = "box" NAME VERSION NL "summary" ":" TEXT NL
              in_sec out_sec needs_sec tests_sec ;

in_sec      = "in" ":" BLOCK{ field NL } ;
field       = IDENT ":" type [ "where" constraints ] ;
type        = "int" | "string" | "bool" | "timestamp"
            | "list" "<" type ">"
            | "{" field { "," field } "}" ;
constraints = expr { "," expr } ;

out_sec     = "out" ":" BLOCK{ ok_decl ensures_sec errors_sec } ;
ok_decl     = "ok" ":" type NL ;
ensures_sec = "ensures" ":" BLOCK{ expr NL } ;
errors_sec  = "errors" ":" BLOCK{ error_decl NL } ;
error_decl  = NAME [ "{" efield { "," efield } "}" ] "when" expr ;
efield      = IDENT [ ":" expr ] ;

needs_sec   = "needs" ":" BLOCK{ need NL } ;
need        = "kv" STRING ":" type [ "where" constraints ] access [ "scan" ]
            | "clock" "read"
            | "effects" ":" effect { "," effect } ;
access      = "read" | "write" | "read-write" ;
effect      = "atomic" | "idempotent" "by" path "for" DURATION ;

tests_sec   = "tests" ":" BLOCK{ example | generate } ;
example     = "example" STRING ":"
              BLOCK{ [ given NL ] "input" value NL expect NL } ;
given       = binding { "," binding } ;
binding     = "kv" "=" kv_map | "clock" "=" STRING ;
expect      = "expect" ( "ok" value | "error" NAME [ value ] )
              [ "," "kv" "=" kv_map ] ;
kv_map      = "{" [ key ":" value { "," key ":" value } ] "}" ;
key         = WORD | STRING ;
generate    = "generate" ":" BLOCK{ gen_line NL } ;
gen_line    = "state" "kv" "covers" path
            | setting { "," setting } ;
setting     = ( "runs" | "min_hits" ) INT ;
```

## 組立書

```ebnf
assembly    = "assembly" NAME VERSION NL { uses | grant } ;
uses        = "uses" NAME VERSION { "," NAME VERSION } NL ;
grant       = "grant" NAME ":" perm { "," perm } NL ;
perm        = "kv" STRING access [ "scan" ]
            | "clock" "read"
            | "http" METHOD STRING
            | "secret" STRING "as" "header" STRING ;
```

## 式

```ebnf
expr        = quant | or_expr ;
quant       = ( "each" | "any" ) IDENT "in" path ":" expr ;
or_expr     = and_expr { "or" and_expr } ;
and_expr    = not_expr { "and" not_expr } ;
not_expr    = [ "not" ] cmp ;
cmp         = sum { cmp_op sum }
            | sum [ "not" ] "in" kv_state
            | "matches" STRING ;
cmp_op      = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
sum         = term { ( "+" | "-" ) term } ;
term        = INT | DURATION | STRING | "true" | "false"
            | kv_state "[" expr { "," expr } "]"
            | ( "len" | "unique" ) "(" path ")"
            | path | "(" expr ")" ;
kv_state    = "kv_before" | "kv_after" ;
path        = IDENT { "." IDENT } ;
value       = INT | STRING | "true" | "false"
            | "[" [ value { "," value } ] "]"
            | "{" [ IDENT ":" value { "," IDENT ":" value } ] "}" ;
```

## 字句

| 字句 | 形 |
|---|---|
| `NAME` | `[A-Z][A-Za-z0-9]*`(箱とエラーの名前) |
| `IDENT` | `[a-z_][a-z0-9_]*`(フィールドと変数の名前) |
| `VERSION` | `v` と1桁以上の数字 |
| `INT` | `-` を前に付けてよい10進数 |
| `DURATION` | 10進数と単位 `s`・`min`・`h`・`d` |
| `STRING` | `"` で囲む。`\"` と `\\` で `"` と `\` を書く |
| `WORD` | `[A-Za-z0-9_]+` |
| `METHOD` | HTTP のメソッド名(`GET`・`POST` など) |
| `TEXT` | `:` のあとから行末まで |

## 規則

### 型と値

- `int` は64ビットの符号付き整数とする。
- `timestamp` の値は、RFC 3339 の `Z` 付きの文字列で書く(`"2026-11-01T10:00:00Z"`)。
- `timestamp` に `DURATION` を足し引きした結果は `timestamp` とする。
- 入力と出力のフィールドに、同じ名前を使えない(検査器が拒否する)。

### 制約(`where`)

- 制約は、`,` で区切ったすべての式が成り立つことを表す。
- フィールドは名前で指す(`where 1 <= qty <= 99`)。kv の値は `v` で指す。
- `matches` は、主語を省き、`where` を付けたフィールドに掛かる。文字列全体が正規表現に一致することを表す。
- レコード型の中では、`,` のあとに `IDENT ":"` が続けば、次のフィールドとする。
- `path` がリストを通るときは、各要素の値の並びを表す(`unique(items.sku)`)。

### 上限

上限のない値は、検査器が拒否する。次のものを上限とみなす。

| 型 | 上限 |
|---|---|
| `int` | 下限と上限の比較 |
| `string` | `len(x) <= N`、または `matches` |
| `list` | `len(x) <= N` |
| `timestamp` | 下限と上限の比較 |

### 正規表現

`matches` に書ける正規表現は、次のものに限る。一致する文字列の長さは、必ず有限の最大値を持つ。

| 書き方 | 意味 |
|---|---|
| 文字 | その文字 |
| `\` と記号 | その記号(`\.`・`\[` など) |
| `[...]` | 中に並べた文字と範囲(`A-Z`)のどれか1文字 |
| `(...)` | まとまり |
| `\|` | どちらか |
| `?` | 0回か1回 |
| `{n}`・`{m,n}` | n 回、m 回以上 n 回以下 |

`*`・`+`・`.`・否定の文字クラス(`[^...]`)・アンカー(`^`・`$`)は書けない。

### each と any

- `each x in p: e` は、`p` のすべての要素 `x` について `e` が成り立つことを表す。
- `any x in p: e` は、`p` の要素を先頭から調べ、`e` が成り立つ要素 `x` があることを表す。
- エラーの `when` に `any` を書いたとき、エラーのフィールドの値は、最初に `e` を満たした要素から取る。`{ sku }` は `{ sku: x.sku }` と同じ意味とする。

### kv

- `kv_before[a]`・`kv_after[a]` の添字の数は、kv の範囲のテンプレートの変数の数と同じにする。添字は、変数の順に当てはめる。
- 例テストの `kv = { ... }` のキーは、テンプレートの変数の値とする(範囲が `"inventory/{sku}"` なら、`A1` はキー `inventory/A1`)。

### 例テスト

- `given` で `clock` を省いたときは、`1970-01-01T00:00:00Z` とする。
- `given` で `kv` を省いたときは、kv の範囲を空とする。
- `expect` の `kv = { ... }` は、呼び出し後の kv の範囲全体と一致することを表す。
