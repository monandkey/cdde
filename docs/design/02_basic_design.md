# CDDE Basic Design Document

| **項目**      | **内容**                                                |
| ----------- | ----------------------------------------------------- |
| **プロジェクト名** | CDDE (Cloud Diameter Distribution Engine) Development |
| **版数**      | 1.0 (Initial Draft)                                   |
| **ステータス**   | **基本設計完了 (Basic Design Frozen)**                      |
| **対象**      | アーキテクト, 開発チーム, インフラ運用チーム                              |

---

## 1. システム概要 (System Overview)

### 1.1. システムの目的

本システム CDDE は、クラウドネイティブ環境 (Kubernetes) 上で動作するキャリアグレードの Diameter Signaling Controller (DSC) である。

4G/5Gモバイルコアネットワークにおいて、Diameter信号の ルーティング (Routing)、トラフィック制御 (Traffic Management)、プロトコル変換 (Manipulation)、および セキュリティ (Topology Hiding) 機能を提供する。

### 1.2. 設計方針 (Design Principles)

1. **Microservices Architecture:** データプレーンとコントロールプレーンを分離し、機能ごとの独立したスケーリングと保守性を確保する。
    
2. **High Performance:** Rust言語を採用し、ゼロコピーパースやロックフリーデータ構造を駆使して、低レイテンシ・高スループットを実現する。
    
3. **Cloud Native Networking:** Multus CNIを活用し、コンテナ環境においてもSCTPマルチホーミングや物理ネットワーク直結性能を提供する。
    

---

## 2. システムアーキテクチャ (System Architecture)

### 2.1. 全体構成

システムは、信号処理を行う **Core Application** 群と、管理・監視を行う **Assistance Application** 群で構成される。

### 2.2. アプリケーション構成一覧

|**カテゴリ**|**名称**|**略称**|**役割概要**|
|---|---|---|---|
|**Core**|**Diameter Frontline**|**DFL**|外部接続終端, 負荷分散, セッション(Tx)管理|
|**Core**|**Diameter Core Router**|**DCR**|ルーティングロジック, AVP操作, VRF機能|
|**Core**|**Diameter Peer Agent**|**DPA**|Peer死活監視 (Active Monitoring)|
|**Assist**|Configuration Manager|CM|設定変更検知・配信|
|**Assist**|Composition Operator|CO|Kubernetesリソース(Pod)のライフサイクル管理|
|**Assist**|Performance Collector|PC|統計情報(Metrics)の収集|
|**Assist**|Fault Informer|FI|アラート監視・通知|
|**Assist**|**Config & Mgmt Service**|**CMS**|統合管理API, UIバックエンド, データ永続化|

### 2.3. 技術スタック

- **開発言語:** Rust (Edition 2021/2024)
    
- **非同期ランタイム:** `tokio`
    
- **通信プロトコル:**
    
    - **External:** SCTP / TCP (via `sctp`, `tokio::net`)
        
    - **Internal:** gRPC (via `tonic`)
        
- **インフラ基盤:** Kubernetes (K8s)
    
- **データストア:** PostgreSQL (CMS用), In-memory (Core用)
    

---

## 3. ネットワーク設計 (Network Design)

### 3.1. 外部接続ネットワーク (External Interface)

Diameter Peerとの接続には、Kubernetesの標準ネットワークではなく、専用の物理ネットワークインターフェースを使用する。

- **CNI Plugin:** **Multus CNI** + **Macvlan** (または IPVlan L2)
    
- **IPアドレス管理:** `Whereabouts` 等のIPAMプラグインにより、Podに対して物理セグメントのIPを直接払い出す。
    
- **SCTP Multi-homing:**
    
    - DFL Podには `net1` (Primary), `net2` (Secondary) の2つのインターフェースを割り当てる。
        
    - アプリケーション(Rust)側で `sctp_bindx` を使用し、複数IPでの待ち受け・送信を行う。
        
- **メリット:** NATフリーによる低遅延、透過的な送信元IP維持。
    

### 3.2. 内部通信ネットワーク (Internal Interface)

マイクロサービス間の通信には、K8s標準ネットワークを使用する。

- **プロトコル:** gRPC over HTTP/2
    
- **サービスディスカバリ:** Kubernetes Service (ClusterIP)
    
    - DFL $\to$ DCR: `dcr-svc-<vr_id>` 宛にリクエストを送信。
        

---

## 4. コンポーネント詳細設計 (Component Design)

### 4.1. Diameter Frontline (DFL)

- **VRF識別:** 受信した物理インターフェース(IPアドレス)に基づき、所属する `Virtual Router ID` を特定する。
    
- **セッション管理 (Transaction):**
    
    - **ストア:** プロセス内メモリ (`DashMap`)。キーは `(ConnectionID, Hop-by-Hop ID)`。
        
    - **タイムアウト:** `DelayQueue` を用いた非同期タイマー。タイムアウト時は `Result-Code: 3002` を自動応答し、遅延したAnswerは破棄する。
        
- **ルーティング:** 特定したVR IDに基づき、適切なDCR ServiceへgRPC転送を行う。
    

### 4.2. Diameter Core Router (DCR)

- **Virtual Router (VRF):** VRごとに独立したDeployment/Podとしてデプロイされる。これにより障害影響をVR内に限定する。
    
- **ロジック:**
    
    1. **Routing:** `Realm`, `Application-ID`, `Destination-Host` をキーに宛先Poolを決定。
        
    2. **Manipulation:** JSON DSLで定義されたルールに基づき、AVPの追加・削除・変更を行う。
        
    3. **Topology Hidden:** 外部へ送信する直前に、内部ホスト名を隠蔽・置換する。
        
- **辞書管理:** 3GPP標準辞書(Static)とベンダー拡張辞書(Dynamic/XML)のハイブリッド構成。
    

### 4.3. Diameter Peer Agent (DPA)

- **監視方式:**
    
    - PeerがUP状態: `DWR` (Device-Watchdog-Request) を送信。
        
    - PeerがDOWN状態: `SCTP Heartbeat` または `INIT` を送信。
        
- **状態通知:** Peerの状態遷移 (UP/DOWN) を検知次第、メッセージキュー等を介してDFLへ即時通知し、ルーティングテーブルを更新させる。
    

---

## 5. インターフェース仕様 (Interface Specifications)

### 5.1. 内部メッセージ定義 (`internal.proto`)

DFLとDCR間でやり取りされるgRPCメッセージ構造。

Protocol Buffers

```
message DiameterPacketRequest {
    uint64 connection_id = 1;     // DFLが管理する物理接続ID
    string vr_id = 2;             // VR識別子
    uint64 reception_timestamp = 3; 
    bytes raw_payload = 4;        // Diameterバイナリデータ
    // ...その他メタデータ
}

message DiameterPacketAction {
    enum Type { FORWARD = 0; REPLY = 1; DISCARD = 2; }
    Type action_type = 1;
    uint64 target_connection_id = 2; // FORWARD時の送信先接続ID
    bytes response_payload = 3;      // 送信データ
}
```

### 5.2. Manipulation DSL (JSON Schema)

AVP操作ルールの定義形式。

JSON

```json
{
  "rule_id": "hide_internal_topology",
  "priority": 10,
  "condition": { "op": "AND", "matches": [ ... ] },
  "actions": [
    { "type": "TOPOLOGY_HIDE", "target": "Origin-Host", "value": "gateway.public" }
  ]
}
```

---

## 6. 非機能要件 (Non-Functional Requirements)

### 6.1. パフォーマンス

- **目標:** ノード単体で数万TPS、内部処理レイテンシ 5ms以下。
    
- **実装戦略:**
    
    - **Lazy Parsing:** ヘッダのみ解析し、AVPは必要時のみデコード。
        
    - **Zero Copy:** 受信バッファを可能な限り使い回す `Bytes` / `Cow` の活用。
        

### 6.2. 可用性・信頼性

- **冗長化:** VR単位で `Replicas >= 2` を維持。
    
- **Failover:** Multusによるネットワーク冗長化に加え、Peer Pool内でのラウンドロビン/フェイルオーバーロジックを実装。
    

### 6.3. 運用・保守性

- **設定反映:** CMSでの設定変更は、CO (Composition Operator) を通じてDCR Podへ動的に適用（ConfigMap更新またはgRPC Push）。
    
- **可観測性:** 全コンポーネントが Prometheus Exporter 形式でメトリクスを公開。ログは構造化JSONで出力。
    

---

### 7. 今後の開発ロードマップ
1. **Phase 1 (Prototype):**
    - Multus環境でのSCTP送受信 (DFL) と gRPC通信 (DCR) の疎通確認。
    - 基本的なS6aルーティングの実装。
2. **Phase 2 (Core Logic):**
    - セッション管理 (Timeout) の実装。
    - Manipulationエンジンの実装。
3. **Phase 3 (Management):**
    - CMS/UIの実装と、辞書管理機能の統合。
4. **Phase 4 (Release):**
    - 負荷試験、障害試験、商用リリース。
