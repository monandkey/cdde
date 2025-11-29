# Rust クレート構成案 (Module & Dependency Design)

CDDEは **Sans-IO Core + Actor Runtime** パターンを採用し、ロジック（Core）とI/O（Runtime）を物理的に分離する。

これにより、ドメイン駆動設計（DDD）の原則に従った、明確なレイヤー分離とテスト容易性を実現する。

---

## 1. アーキテクチャ概要: DDD層とRust Workspace

| DDD層 | Rust Workspace対応 | 責務 | 依存方向 |
|---|---|---|---|
| **Domain Layer** | `{component}-core` クレート | 純粋なビジネスロジック、状態遷移、ドメインモデル | 外部依存なし（`cdde-shared`のみ） |
| **Application Layer** | `{component}-core` 内のユースケース | ドメインロジックのオーケストレーション | Domain Layer |
| **Infrastructure Layer** | `{component}-runtime` クレート | I/O操作、永続化、外部システム連携 | Application Layer, Domain Layer |
| **Presentation Layer** | `{component}` バイナリクレート | エントリーポイント、DIコンテナ組み立て | すべての層 |

---

## 2. クレート構成

### A. 共通基盤クレート (Shared Kernel)

| クレート名 | 責務 | 技術スタック | 利用元 |
|---|---|---|---|
| **`cdde-shared`** | **共通型定義**<br>DiameterMessage, AVP, SessionKey等<br>**ゼロコピー基盤** | `bytes::Bytes`, `serde` | 全Core/Runtimeクレート |
| `cdde-proto` | **内部通信プロトコル定義**<br>gRPC service/message定義 | `tonic`, `prost` | DFL, DCR, CMS |
| `cdde-diameter-dict` | **Diameter辞書**<br>標準AVP/コマンド定義、パーサー | `nom`, `lazy_static` | DFL-core, DCR-core |

---

### B. DFL (Diameter Frontline) クレート群

| クレート名 | DDD層 | 責務 | 依存クレート |
|---|---|---|---|
| **`cdde-dfl-core`** | **Domain** | **SessionManagerCore**<br>セッション状態管理FSM<br>タイムアウト判定ロジック（純粋関数） | `cdde-shared` |
| **`cdde-dfl-runtime`** | **Infrastructure** | **SessionActor**<br>SCTP I/O, `DelayQueue`タイマー<br>gRPC Client (DCR呼び出し) | `cdde-dfl-core`<br>`tokio`, `tonic`<br>`tokio-util` |
| **`cdde-dfl`** | **Presentation** | main関数、DI組み立て | `cdde-dfl-runtime` |

**依存グラフ:** `cdde-dfl` → `cdde-dfl-runtime` → `cdde-dfl-core` → `cdde-shared`

---

### C. DCR (Diameter Core Router) クレート群

| クレート名 | DDD層 | 責務 | 依存クレート |
|---|---|---|---|
| **`cdde-dcr-core`** | **Domain** | **RouterCore**<br>ルーティングエンジン<br>**ManipulationEngine**<br>AVP書き換え、Topology Hiding | `cdde-shared`<br>`cdde-diameter-dict`<br>`regex` |
| **`cdde-dcr-runtime`** | **Infrastructure** | **DcrService** (gRPC Server)<br>設定変更監視<br>`ArcSwap`による動的設定更新 | `cdde-dcr-core`<br>`tonic`, `arc-swap` |
| **`cdde-dcr`** | **Presentation** | main関数、VR設定ロード | `cdde-dcr-runtime` |

**依存グラフ:** `cdde-dcr` → `cdde-dcr-runtime` → `cdde-dcr-core` → `cdde-shared`

---

### D. DPA (Diameter Peer Agent) クレート群

| クレート名 | DDD層 | 責務 | 依存クレート |
|---|---|---|---|
| **`cdde-dpa-core`** | **Domain** | **PeerFsm**<br>RFC 6733 ピア状態遷移マシン<br>Watchdogロジック（純粋関数） | `cdde-shared` |
| **`cdde-dpa-runtime`** | **Infrastructure** | **PeerActor**<br>SCTP Heartbeat送信<br>DWR/DWA処理<br>DFL通知 | `cdde-dpa-core`<br>`tokio`, `sctp` |
| **`cdde-dpa`** | **Presentation** | main関数、Peer設定ロード | `cdde-dpa-runtime` |

**依存グラフ:** `cdde-dpa` → `cdde-dpa-runtime` → `cdde-dpa-core` → `cdde-shared`

---

### E. CMS (Config & Management Service) クレート

| クレート名 | 責務 | 依存クレート |
|---|---|---|
| **`cdde-cms`** | 統合管理API<br>PostgreSQL永続化<br>gRPC/REST API提供 | `cdde-proto`, `sqlx`<br>`axum`, `tonic` |

---

## 3. 依存関係の原則

### 3.1. Core クレートの制約

**Core クレート (`*-core`) は以下に依存してはならない:**
- ❌ `tokio` (非同期ランタイム)
- ❌ `tonic` (gRPC)
- ❌ `sctp`, `tokio::net` (I/O)
- ❌ `sqlx` (データベース)

**Core クレートが依存して良いもの:**
- ✅ `cdde-shared` (共通型)
- ✅ `serde`, `thiserror` (シリアライズ、エラー定義)
- ✅ `std::collections` (HashMap等)

### 3.2. Runtime クレートの責務

Runtime クレートは:
- Core の状態遷移関数 (`step`, `process` 等) を呼び出す
- Core からの **Action** (命令) を実際のI/O操作に変換する
- 外部イベント (ソケット受信、タイマー発火) を Core の **Event** に変換する

---

## 4. メリット

| メリット | 効果 |
|---|---|
| **テスト容易性** | Core クレートは I/O モック不要で単体テスト可能<br>コンパイル時間短縮（Core変更時にRuntimeは再ビルド不要） |
| **依存性の明確化** | `Cargo.toml` で物理的に依存を制限<br>誤ってCoreからI/Oを呼ぶとコンパイルエラー |
| **ポータビリティ** | Core ロジックは WASM や他のランタイムへ移植可能 |
| **並行開発** | Core と Runtime を別チームで並行開発可能 |

---

## 5. 実装例: DFL Session Manager

```rust
// cdde-dfl-core/src/session.rs (Domain Layer)
pub struct SessionManagerCore {
    sessions: HashMap<SessionKey, SessionState>,
}

impl SessionManagerCore {
    // 純粋関数: I/Oなし
    pub fn on_request_received(&mut self, msg: DiameterMessage) 
        -> Vec<SessionAction> 
    {
        // ロジックのみ
        vec![SessionAction::ForwardToDcr(msg)]
    }
}

// cdde-dfl-runtime/src/actor.rs (Infrastructure Layer)
pub struct SessionActor {
    core: SessionManagerCore,
    timeout_queue: DelayQueue<SessionKey>,
}

impl SessionActor {
    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                Some(msg) = self.receiver.recv() => {
                    // Coreを呼び出し
                    let actions = self.core.on_request_received(msg);
                    // Actionを実行（I/O）
                    self.execute_actions(actions).await;
                }
                // ...
            }
        }
    }
}
```

詳細は [18_architecture_patterns.md](file:///workspace/docs/design/18_architecture_patterns.md) を参照。

---
