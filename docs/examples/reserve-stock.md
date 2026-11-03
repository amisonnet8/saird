# 例:在庫を引き当てる箱(ReserveStock)

```
box ReserveStock v1
summary: 注文に含まれる商品の在庫を引き当てる

in:
  order_id: string  where matches "ORD-[0-9]{8}"
  items:    list<{ sku: string where matches "[A-Z0-9]{1,16}",
                   qty: int    where 1 <= qty <= 99 }>
            where 1 <= len(items) <= 50, unique(items.sku)

out:
  ok: { reserved_until: timestamp }
  ensures:
    each item: item.sku in kv_before
    each item: kv_after[item.sku] == kv_before[item.sku] - item.qty
    reserved_until == clock + 15min
  errors:                                # 契約のエラー選択規則に従う
    UnknownSku { sku }  when any item: item.sku not in kv_before
    OutOfStock { sku }  when any item: item.qty > kv_before[item.sku]

needs:
  kv "inventory/{sku}": int where 0 <= v <= 100000   read-write
  clock read
  effects: atomic, idempotent by order_id for 24h

tests:
  example "在庫あり":
    given  kv = { A1: 10 }, clock = "2026-11-01T10:00:00Z"
    input  { order_id: "ORD-00000001", items: [{sku: "A1", qty: 3}] }
    expect ok { reserved_until: "2026-11-01T10:15:00Z" }, kv = { A1: 7 }

  example "存在しない商品":
    given  kv = { A1: 10 }
    input  { order_id: "ORD-00000002", items: [{sku: "B2", qty: 1}] }
    expect error UnknownSku { sku: "B2" }

  generate:
    state kv covers items.sku
    runs 1000, min_hits 50
```

失敗時に在庫が変わらないこと、二重に引き当てないこと、期限切れの在庫を戻すことは、この箱の `ensures` には含めない。最初の2つは effects としてホストが保証し、最後の1つは別の箱が担う。
