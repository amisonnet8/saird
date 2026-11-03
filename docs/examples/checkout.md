# 例:組立書(Checkout)

```
assembly Checkout v1
  uses ReserveStock v1, ChargeCard v2
  grant ReserveStock: kv "inventory/{sku}" read-write, clock read
  grant ChargeCard:   http POST "pay.example.com/charge", secret "pay-key" as header "Authorization"
```
