# 例:呼び出し

実装の中での呼び出しを概念的に表したもの。

```
r = ReserveStock.call({ order_id, items })
match r:
  ok { reserved_until }    -> ...
  error OutOfStock { sku } -> ...
  error UnknownSku { sku } -> ...
  fault f                  -> ...    # 省略すると親へ伝わる
```
