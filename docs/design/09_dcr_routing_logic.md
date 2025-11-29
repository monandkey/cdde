## DCR (Diameter Core Router) ルーティングロジック詳細設計

---

## 1. ルーティングの基本方針

DCRは、受信したDiameterパケットのヘッダおよびAVP情報を元に、次の転送先（Peer Pool）を決定します。

### 1.1. ルーティング優先順位

ルーティングは以下の優先順位で評価されます:

| 優先度 | ルーティングキー | 説明 | 使用AVP/ヘッダ |
|:---:|:---|:---|:---|
| **1** | **Destination-Host** | 特定のホストへの直接ルーティング | AVP Code: 293 |
| **2** | **Application-ID + Command-Code** | アプリケーション種別とコマンドの組み合わせ | Header: Application-ID, Command-Code |
| **3** | **Destination-Realm** | Realmベースのルーティング | AVP Code: 283 |
| **4** | **Default Route** | 上記に該当しない場合のデフォルト転送先 | 設定ファイル |

---

## 2. ルーティングテーブル構造

DCRは起動時に、CMSから取得した設定を元にルーティングテーブルをメモリ上に構築します。

### 2.1. Rustデータ構造

```rust
// routing/src/table.rs

use std::collections::HashMap;

/// ルーティングテーブルのエントリ
#[derive(Debug, Clone)]
pub struct RouteEntry {
    pub priority: u8,
    pub condition: RouteCondition,
    pub target_pool_id: String,
    pub load_balance_strategy: LoadBalanceStrategy,
}

/// ルーティング条件
#[derive(Debug, Clone)]
pub enum RouteCondition {
    DestinationHost(String),
    ApplicationCommand { app_id: u32, command_code: u32 },
    DestinationRealm(String),
    Default,
}

/// 負荷分散戦略
#[derive(Debug, Clone)]
pub enum LoadBalanceStrategy {
    RoundRobin,
    LeastConnection,
    Random,
}

/// ルーティングテーブル
pub struct RoutingTable {
    // 優先度順にソートされたルートエントリ
    routes: Vec<RouteEntry>,
    // Pool ID -> Peer リスト のマッピング
    pools: HashMap<String, PeerPool>,
}

/// Peerプール
#[derive(Debug, Clone)]
pub struct PeerPool {
    pub pool_id: String,
    pub peers: Vec<PeerInfo>,
    pub current_index: std::sync::atomic::AtomicUsize, // Round-Robin用
}

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub host_name: String,
    pub ip_addresses: Vec<std::net::IpAddr>,
    pub status: PeerStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PeerStatus {
    Up,
    Down,
}
```

### 2.2. アーキテクチャ: Stateless Core Design (Sans-IO)

DCR は **Sans-IO** パターンを採用し、ルーティングロジックとI/O操作を分離します。

#### 2.2.1. Core Layer: RouterCore

**責務:** ルーティング判断とManipulation適用（純粋関数）

```rust
// cdde-dcr/src/core/router.rs
use shared::{DiameterMessage, AVP_DEST_REALM};
use super::manipulation::ManipulationEngine;

#[derive(Debug, PartialEq)]
pub enum RouteAction {
    Forward(String),     // 転送先Peer名
    Discard,             // 破棄
    ReplyError(u32),     // エラーコード返却 (3002, 3003等)
}

// ★ Sans-IO Core Logic
pub struct RouterCore {
    routes: Vec<RouteEntry>,
    manipulator: ManipulationEngine,
}

impl RouterCore {
    // ★ 純粋関数: 入力メッセージ -> (加工後メッセージ, アクション)
    pub fn process(&self, msg: DiameterMessage) -> (DiameterMessage, RouteAction) {
        // 1. Manipulation & Topology Hiding 実行
        let processed_msg = self.manipulator.apply(msg);

        // 2. ルーティング決定
        let dest_realm = processed_msg.get_avp(AVP_DEST_REALM)
            .map(|a| a.as_string())
            .unwrap_or_default();

        let action = if let Some(route) = self.routes.iter().find(|r| r.dest_realm == dest_realm) {
            RouteAction::Forward(route.target_peer.clone())
        } else {
            RouteAction::ReplyError(3003)  // DIAMETER_REALM_NOT_SERVED
        };

        (processed_msg, action)
    }
}
```

#### 2.2.2. Runtime Layer: DcrService with ArcSwap

**責務:** gRPC通信と動的設定更新（無停止）

```rust
// cdde-dcr-runtime/src/server.rs
use arc_swap::ArcSwap;  // ★ ロックフリー設定更新
use std::sync::Arc;

pub struct DcrService {
    // ★ ArcSwapによる動的設定更新
    core: Arc<ArcSwap<RouterCore>>,
}

impl DcrService {
    pub fn new(initial_core: RouterCore) -> Self {
        Self {
            core: Arc::new(ArcSwap::from_pointee(initial_core)),
        }
    }

    // 設定変更時に呼ばれる (CMからの通知)
    pub fn update_config(&self, new_core: RouterCore) {
        self.core.store(Arc::new(new_core));
        // トラフィック処理を止めずに設定更新完了
    }

    async fn process_packet(&self, msg: DiameterMessage) -> RouteAction {
        // ★ ロックフリーで現在の設定を取得
        let core = self.core.load();
        let (processed_msg, action) = core.process(msg);
        action
    }
}
```

**メリット:**
- **無停止設定更新:** `ArcSwap` により、トラフィック処理中でも設定変更可能
- **ロックフリー:** Read Lock が発生しないため、高スループットを維持
- **テスト容易性:** `RouterCore` は I/O モック不要で単体テスト可能

---

## 3. ルーティング処理フロー

### 3.1. パケット受信からルーティング決定まで

```rust
// routing/src/engine.rs

impl RoutingEngine {
    pub async fn route_packet(&self, packet: &DiameterPacket) -> Result<RoutingDecision, RoutingError> {
        // 1. パケットからルーティングキーを抽出
        let dest_host = packet.get_avp(AVP_DESTINATION_HOST)?;
        let dest_realm = packet.get_avp(AVP_DESTINATION_REALM)?;
        let app_id = packet.header.application_id;
        let cmd_code = packet.header.command_code;

        // 2. ルーティングテーブルを優先度順に評価
        for route in &self.table.routes {
            if self.matches(&route.condition, dest_host, dest_realm, app_id, cmd_code) {
                // 3. マッチしたルートのPoolから次のPeerを選択
                let peer = self.select_peer(&route.target_pool_id, &route.load_balance_strategy)?;
                
                return Ok(RoutingDecision {
                    target_peer: peer,
                    route_entry: route.clone(),
                });
            }
        }

        Err(RoutingError::NoRouteFound)
    }
}
```

### 3.2. 条件マッチング

```rust
fn matches(
    &self,
    condition: &RouteCondition,
    dest_host: Option<&str>,
    dest_realm: Option<&str>,
    app_id: u32,
    cmd_code: u32,
) -> bool {
    match condition {
        RouteCondition::DestinationHost(host) => {
            dest_host.map_or(false, |h| h == host)
        }
        RouteCondition::ApplicationCommand { app_id: a, command_code: c } => {
            *a == app_id && *c == cmd_code
        }
        RouteCondition::DestinationRealm(realm) => {
            dest_realm.map_or(false, |r| r == realm)
        }
        RouteCondition::Default => true,
    }
}
```

---

## 4. 負荷分散アルゴリズム

### 4.1. Round-Robin実装

```rust
impl RoutingEngine {
    fn select_peer_round_robin(&self, pool: &PeerPool) -> Result<PeerInfo, RoutingError> {
        // UP状態のPeerのみをフィルタリング
        let active_peers: Vec<_> = pool.peers.iter()
            .filter(|p| p.status == PeerStatus::Up)
            .collect();

        if active_peers.is_empty() {
            return Err(RoutingError::NoActivePeer);
        }

        // Atomic操作でインデックスを取得・更新
        let index = pool.current_index.fetch_add(1, Ordering::Relaxed);
        let selected = &active_peers[index % active_peers.len()];

        Ok((*selected).clone())
    }
}
```

### 4.2. Least Connection (将来実装)

```rust
fn select_peer_least_connection(&self, pool: &PeerPool) -> Result<PeerInfo, RoutingError> {
    // 各Peerの現在の接続数を追跡し、最も少ないPeerを選択
    // TODO: Phase 2で実装
    unimplemented!()
}
```

---

## 5. Route-Record AVP処理

### 5.1. Route-Record追加

DCRは転送時に、自身のホスト名を **Route-Record AVP (Code: 282)** に追加します。

```rust
impl RoutingEngine {
    fn add_route_record(&self, packet: &mut DiameterPacket) -> Result<(), RoutingError> {
        let my_hostname = &self.config.hostname;
        
        // Route-Record AVPを作成
        let route_record_avp = AVP {
            code: 282,
            flags: AVP_FLAG_MANDATORY,
            vendor_id: None,
            data: my_hostname.as_bytes().to_vec(),
        };

        // パケットに追加
        packet.add_avp(route_record_avp);
        Ok(())
    }
}
```

### 5.2. Route-Record参照（ループ検出）

```rust
fn detect_routing_loop(&self, packet: &DiameterPacket) -> bool {
    let my_hostname = &self.config.hostname;
    
    // Route-Record AVPを全て取得
    let route_records = packet.get_all_avps(282);
    
    // 自分のホスト名が既に含まれていればループ
    route_records.iter().any(|avp| {
        String::from_utf8_lossy(&avp.data) == my_hostname.as_str()
    })
}
```

---

## 6. エラーハンドリング

### 6.1. ルーティング失敗時の処理

| エラー種別 | Result-Code | 処理 |
|:---|:---:|:---|
| ルートが見つからない | 3003 | `DIAMETER_REALM_NOT_SERVED` を返す |
| 全Peerがダウン | 3002 | `DIAMETER_UNABLE_TO_DELIVER` を返す |
| ループ検出 | 3005 | `DIAMETER_LOOP_DETECTED` を返す |

```rust
pub enum RoutingError {
    NoRouteFound,
    NoActivePeer,
    LoopDetected,
    InvalidPacket(String),
}

impl RoutingError {
    pub fn to_result_code(&self) -> u32 {
        match self {
            RoutingError::NoRouteFound => 3003,
            RoutingError::NoActivePeer => 3002,
            RoutingError::LoopDetected => 3005,
            RoutingError::InvalidPacket(_) => 3008,
        }
    }
}
```

---

## 7. 設定例 (YAML)

```yaml
routing:
  virtual_router_id: "vr-001"
  hostname: "dcr-vr001.internal.net"
  
  routes:
    # 優先度1: 特定ホストへの直接ルーティング
    - priority: 10
      condition:
        type: destination_host
        value: "hss01.operator.net"
      target_pool: "pool-hss-primary"
      load_balance: round_robin

    # 優先度2: S6aアプリケーションのULR
    - priority: 20
      condition:
        type: application_command
        app_id: 16777251  # S6a
        command_code: 316  # ULR
      target_pool: "pool-hss-s6a"
      load_balance: round_robin

    # 優先度3: Realmベース
    - priority: 30
      condition:
        type: destination_realm
        value: "operator.net"
      target_pool: "pool-default"
      load_balance: round_robin

    # デフォルトルート
    - priority: 100
      condition:
        type: default
      target_pool: "pool-fallback"
      load_balance: round_robin

pools:
  - pool_id: "pool-hss-primary"
    peers:
      - host_name: "hss01.operator.net"
        ip_addresses: ["10.0.1.10", "10.0.2.10"]
      - host_name: "hss02.operator.net"
        ip_addresses: ["10.0.1.11", "10.0.2.11"]
```
