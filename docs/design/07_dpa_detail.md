
## 1. Diameter Peer Agent (DPA) 実装詳細

### 1.1. DPAの責務と動作モデル

DPAは、PeerとのSCTP接続を確立し、定期的な監視を行い、状態の変化を検出する責務を持ちます。

  * **状態管理:** Peerごとに有限状態機械（FSM）を持ち、`DOWN` $\to$ `CONNECTING` $\to$ `UP` の状態遷移を管理します。
  * **クライアントロールの処理:** VR（Virtual Router）のロールが **`Client`** の場合、Peerが `DOWN` であれば、SCTPの `INIT` および Diameterの `CER` (Capability-Exchange-Request) を送信して接続確立を試みます。
  * **サーバーロールの処理:** VRのロールが **`Server`** の場合、接続待ち受けに専念します。
  * **パケット処理:**
      * 外部から `DWR` (Device-Watchdog-Request) を受信した場合、`Result-Code: 2001 (DIAMETER_SUCCESS)` を含む `DWA` (Answer) を返します。
      * その他のパケットは **DFLへルーティング** されます（ただし、基本設計で「死活監視パケットをDPAにルーティングする」とあるため、DFLは DWR/DWA の Command-Code を見てDPAに振り分けることになります）。

### 1.2. Alive Monitoring (AM) ロジック

要件にある、状態に応じた監視方法の切り替えを実装します。

| 状態 | 監視手段 | タイマー | 判定ロジック |
| :--- | :--- | :--- | :--- |
| **UP** | **DWR/DWA** の往復 | `interval_sec` ごとにDWRを送信 | `timeout_ms` 以内にDWAが返ってこなければ `retry_count` のカウントを開始。カウントが上限を超えたら **DOWN** へ遷移。 |
| **DOWN** | **SCTP Heartbeat** または **CER** | `interval_sec` ごとにSCTPまたはCERを送信 | `max_retries` の試行回数内に接続確立または応答がなければ、再試行間隔を延ばすなどの処理を行う。 |
| **切断時** | Diameter `DPR` | Peer切断時には **`DPR` (Disconnect-Peer-Request)** を送信する。 | |

### 1.3. DFLへの状態通知ロジック (Service Discovery)

DPAがPeerの状態を更新した際、DFLのルーティング対象テーブルをリアルタイムで更新する必要があります。

  * **プロトコル:** gRPCによる **RPC** または **メッセージキュー (Kafka/NATS)** を推奨します。ここでは **gRPCのUnary RPC** を採用します。
  * **インターフェース:** DFL内に **`RoutingUpdateService`** を定義します。

<!-- end list -->

```protobuf
// dfl_api.proto (DFLが公開するAPI)
service RoutingUpdateService {
    // Peerの状態変化をDFLへ通知し、ルーティングテーブルを更新させる
    rpc UpdatePeerStatus (PeerStatusRequest) returns (UpdateResponse);
}

message PeerStatusRequest {
    string peer_node_id = 1;
    enum Status { UP = 0; DOWN = 1; }
    Status current_status = 2;
    repeated string virtual_router_ids = 3; // 影響するVR IDのリスト
}
```

  * **DFLの処理:** DFLは `PeerStatusRequest` を受け取ると、内部のPeerテーブルを更新し、そのPeerへの新規トラフィックの転送を即時開始（UP時）または停止（DOWN時）します。
