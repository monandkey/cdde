
## 1. Manipulation DSL (JSON) 詳細設計

### 1.1. 設計方針

  * **宣言的アプローチ:** 処理手順ではなく、**「もしこうなら、こうする」** というルールを宣言的に記述します。
  * **高速性:** ルールは起動時にRustの構造体 (`enum`, `struct`) にパースされ、正規表現は事前コンパイルされるため、実行時のオーバーヘッドを最小限に抑えます。
  * **柔軟性:** AND/ORの論理演算、正規表現マッチングをサポートします。

### 1.2. ルールエンジン実行フロー

DCRは、ルーティング処理中に、設定されたルールを優先度順に実行します。

-----

## 2\. JSON スキーマ定義 (The Core Schema)

全てのルールは、`rules` 配列内に格納されます。

### A. トップレベル構造 (`Rule`)

| フィールド名 | 型 | 説明 | 必須 |
| :--- | :--- | :--- | :--- |
| `rule_id` | `string` | ルールを一意に識別するID (ログ・監査用) | 必須 |
| `priority` | `u32` | 実行順序 (数字が小さいほど高優先度) | 必須 |
| `direction` | `enum` | **INGRESS** (受信時) または **EGRESS** (送信時) のどちらで適用するか | 必須 |
| `condition` | `Condition` | このルールを実行するための条件オブジェクト | 必須 |
| `actions` | `[Action]` | 条件が満たされた場合に実行する操作リスト | 必須 |

### B. 条件定義 (`Condition` & `Match`)

条件は、複数の**マッチング項目 (`Match`)** を論理演算子で組み合わせる形式をとります。

#### `Condition` スキーマ

```json
"condition": {
  "operator": "AND", // または "OR"
  "matches": [
    { /* Match 1 */ },
    { /* Match 2 */ } 
  ]
}
```

#### `Match` スキーマ (単一のチェック項目)

| フィールド名 | 型 | 説明 | 例 |
| :--- | :--- | :--- | :--- |
| `target` | `enum` | チェック対象: **HEADER**, **AVP** | AVP |
| `avp_code` | `u32` | `target: AVP` の場合: AVPコード (例: 268 = Result-Code) | 268 |
| `field` | `string` | `target: HEADER` の場合: ヘッダフィールド名 (例: `command_code`) | command\_code |
| `match_op` | `enum` | 演算子: **EQ** (一致), **NE** (不一致), **REGEX**, **EXISTS** (存在確認) | REGEX |
| `value` | `string` | 比較対象の値 (REGEXの場合はパターン) | `^.*\\.example\\.com$` |

**例:** Command CodeがCCR (272)で、Origin-Hostが `hss` で始まる場合

```json
"condition": {
  "operator": "AND",
  "matches": [
    {"target": "HEADER", "field": "command_code", "match_op": "EQ", "value": "272"},
    {"target": "AVP", "avp_code": 264, "match_op": "REGEX", "value": "^hss.*"}
  ]
}
```

-----

## 3. アクションタイプ詳細 (`Action`)

条件に合致した際、パケットに対して実行される操作リストです。

| フィールド名 | 型 | 説明 | 必須 |
| :--- | :--- | :--- | :--- |
| `type` | `enum` | **SET\_VALUE**, **ADD\_AVP**, **DELETE\_AVP**, **TOPOLOGY\_HIDE** | 必須 |
| `avp_code` | `u32` | 操作対象のAVPコード (SET, ADD, DELETE時) | 条件付必須 |
| `value` | `string` | SET/ADD時に挿入する値 (文字列、Rustで型変換) | SET/ADD時 |
| `params` | `object` | `TOPOLOGY_HIDE` 専用の引数 | TH時 |

### A. TOPOLOGY_HIDE アクションの定義 (特化ロジック)

トポロジー隠蔽は、複数のAVP（Origin-Host, Origin-Realm, Route-Record）にまたがる操作であるため、専用のスキーマを使用します。

```json
"actions": [
  {
    "type": "TOPOLOGY_HIDE",
    "params": {
      "strategy": "REPLACE_FIXED", // 固定値置換戦略
      "host_avp_code": 264,        // Origin-Host
      "realm_avp_code": 296,       // Origin-Realm
      "replacement_host": "dra.public.net", // 置換後の固定ホスト名
      "replacement_realm": "public.net",    // 置換後の固定Realm名
      "remove_route_record": true  // Route-Recordから自社RealmのAVPを削除するか
    }
  }
]
```

  * **`strategy`**: 初版では **`REPLACE_FIXED`** (固定値置換) のみをサポートします。
  * **`remove_route_record`**: トポロジーを隠蔽するため、パケットが外部へ出る前に、`Route-Record` AVPから自社のホスト/Realmに関する情報を削除します。

### B. 通常の Manipulation アクション例

```json
"actions": [
  // 1. Result-Codeを5001に強制上書き (SET_VALUE)
  {"type": "SET_VALUE", "avp_code": 268, "value": "5001"}, 
  
  // 2. 独自のベンダーAVP (999) を追加 (ADD_AVP)
  {"type": "ADD_AVP", "avp_code": 999, "value": "custom_data_X"}, 
  
  // 3. User-Name (1) を削除 (DELETE_AVP)
  {"type": "DELETE_AVP", "avp_code": 1}
]
```
