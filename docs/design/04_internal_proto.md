## 内部インターフェース詳細 (`internal.proto`)

### A. サービス定義

**`CoreRouterService`** は、DFLがDCRへパケットを送り、DCRが結果を返すインターフェースです。双方向ストリーミングを使用することで、効率的なパケットのパイプライン処理を実現します。

```protobuf
syntax = "proto3";
package cdde.internal;

// DFLとDCR間の通信サービス
service CoreRouterService {
    // 双方向ストリーミングにより、DFLからDCRへ連続的にRequestを送り、
    // DCRからDFLへ連続的にActionを返す。
    rpc ProcessStream (stream DiameterPacketRequest) returns (stream DiameterPacketAction);
}
```

### B. DFLからDCRへのリクエストメッセージ

| フィールド名 | 型 | 説明 | 必須責務 |
| :--- | :--- | :--- | :--- |
| `connection_id` | `uint64` | **DFL内のSCTP接続を識別するユニークID** (返信時に利用) | DFL |
| `vr_id` | `string` | DFLが受信IPから判定した **Virtual Router ID** | DFL |
| `reception_timestamp` | `uint64` | DFLでの受信時刻 (ナノ秒) | DFL |
| `raw_payload` | `bytes` | **生のDiameterパケット** (バイナリデータ) | DFL |
| `session_tx_id` | `uint64` | DFLが割り当てた**セッショントラッキングID** | DFL |

```protobuf
message DiameterPacketRequest {
    uint64 connection_id = 1;
    string vr_id = 2;
    uint64 reception_timestamp = 3;
    bytes raw_payload = 4;
    uint64 session_tx_id = 5;  // DFLが割り当てたセッショントラッキングID
    // ...その他ログ用メタデータ
}
```

### C. DCRからDFLへのアクションメッセージ

DCRは論理的な処理を終えた後、DFLに対し「次に何をすべきか」を指示します。

| フィールド名 | 型 | 説明 | 必須責務 |
| :--- | :--- | :--- | :--- |
| `action_type` | `enum ActionType` | **FORWARD** (転送), **REPLY** (即時応答), **DISCARD** (破棄) | DCR |
| `target_host_name` | `string` | `FORWARD` 時に指定する**次の宛先ホスト名** (DFLがPeer Tableを検索するキー) | DCR |
| `response_payload` | `bytes` | 送信する最終的なDiameterパケット (操作済み) | DCR |
| `original_connection_id` | `uint64` | `REPLY` 時に使用。元リクエストの `connection_id` を指定 | DCR |

```protobuf
message DiameterPacketAction {
    enum ActionType {
        FORWARD = 0;
        REPLY = 1;
        DISCARD = 2;
    }
    ActionType action_type = 1;
    string target_host_name = 2;
    bytes response_payload = 3;
    uint64 original_connection_id = 4;
}
```
