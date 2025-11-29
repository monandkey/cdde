# Rust クレート構成案 (Module & Dependency Design)

CDDEは **Sans-IO Core + Actor Runtime** パターンを採用し、ロジック（Core）とI/O（Runtime）を物理的に分離する。

これにより、ドメイン駆動設計（DDD）の原則に従った、明確なレイヤー分離とテスト容易性を実現する。

---

## 1. アーキテクチャ概要: DDD層とRust Workspace

| DDD層 | Rust Workspace対応 | 責務 | 依存方向 |
|---|---|---|---|
| **Domain Layer** | `{component}/src/core/` モジュール | 純粋なビジネスロジック、状態遷移、ドメインモデル | 外部依存なし（`cdde-shared`のみ） |
| **Application Layer** | `{component}/src/app/` モジュール | ドメインロジックのオーケストレーション | Domain Layer |
| **Infrastructure Layer** | `{component}/src/runtime/` モジュール | I/O操作、永続化、外部システム連携 | Application Layer, Domain Layer |
| **Presentation Layer** | `{component}/src/main.rs` | エントリーポイント、DIコンテナ組み立て | すべての層 |

---

## 2. クレート構成

### A. 共通基盤クレート (Shared Kernel)

| クレート名 | 責務 | 技術スタック | 利用元 |
|---|---|---|---|
| **`cdde-shared`** | **共通型定義**<br>DiameterMessage, AVP, SessionKey等<br>**ゼロコピー基盤** | `bytes::Bytes`, `serde` | 全Core/Runtimeクレート |
| `cdde-proto` | **内部通信プロトコル定義**<br>gRPC service/message定義 | `tonic`, `prost` | DFL, DCR, CMS |
| `cdde-diameter-dict` | **Diameter辞書**<br>標準AVP/コマンド定義、パーサー | `nom`, `lazy_static` | DFL-core, DCR-core |

---

### B. DFL (Diameter Frontline) クレート

**クレート:** `cdde-dfl` (単一クレート、内部モジュールで分離)

| モジュールパス | DDD層 | 責務 | 依存 |
|---|---|---|---|
| **`src/core/`** | **Domain** | **SessionManagerCore**<br>セッション状態管理FSM<br>タイムアウト判定ロジック（純粋関数） | `cdde-shared` |
| **`src/runtime/`** | **Infrastructure** | **SessionActor**<br>SCTP I/O, `DelayQueue`タイマー<br>gRPC Client (DCR呼び出し) | `core`, `tokio`, `tonic`, `tokio-util` |
| **`src/app/`** | **Application** | アプリケーションロジック<br>`network`, `client`, `store` | `core`, `runtime` |
| **`src/main.rs`** | **Presentation** | main関数、DI組み立て | `app`, `runtime`, `core` |

**モジュール依存:** `main` → `app` → `runtime` → `core` → `cdde-shared`

---

### C. DCR (Diameter Core Router) クレート

**クレート:** `cdde-dcr` (単一クレート、内部モジュールで分離)

| モジュールパス | DDD層 | 責務 | 依存 |
|---|---|---|---|
| **`src/core/`** | **Domain** | **RouterCore**<br>ルーティングエンジン<br>**ManipulationEngine**<br>AVP書き換え、Topology Hiding | `cdde-shared`, `cdde-diameter-dict`, `regex` |
| **`src/runtime/`** | **Infrastructure** | **DcrService** (gRPC Server)<br>設定変更監視<br>`ArcSwap`による動的設定更新 | `core`, `tonic`, `arc-swap` |
| **`src/main.rs`** | **Presentation** | main関数、VR設定ロード | `runtime`, `core` |

**モジュール依存:** `main` → `runtime` → `core` → `cdde-shared`

---

### D. DPA (Diameter Peer Agent) クレート

**クレート:** `cdde-dpa` (単一クレート、内部モジュールで分離)

| モジュールパス | DDD層 | 責務 | 依存 |
|---|---|---|---|
| **`src/core/`** | **Domain** | **PeerFsm**<br>RFC 6733 ピア状態遷移マシン<br>Watchdogロジック（純粋関数） | `cdde-shared` |
| **`src/runtime/`** | **Infrastructure** | **PeerActor**<br>SCTP Heartbeat送信<br>DWR/DWA処理<br>DFL通知 | `core`, `tokio`, `sctp` |
| **`src/main.rs`** | **Presentation** | main関数、Peer設定ロード | `runtime`, `core` |

**モジュール依存:** `main` → `runtime` → `core` → `cdde-shared`

---

### E. CMS (Config & Management Service) クレート

| クレート名 | 責務 | 依存クレート |
|---|---|---|
| **`cdde-cms`** | 統合管理API<br>PostgreSQL永続化<br>gRPC/REST API提供 | `cdde-proto`, `sqlx`<br>`axum`, `tonic` |

---

## 3. 依存関係の原則

### 3.1. Core モジュールの制約

**Core モジュール (`src/core/`) は以下に依存してはならない:**
- ❌ `tokio` (非同期ランタイム)
- ❌ `tonic` (gRPC)
- ❌ `sctp`, `tokio::net` (I/O)
- ❌ `sqlx` (データベース)

**Core モジュールが依存して良いもの:**
- ✅ `cdde-shared` (共通型)
- ✅ `serde`, `thiserror` (シリアライズ、エラー定義)
- ✅ `std::collections` (HashMap等)

### 3.2. Runtime モジュールの責務

Runtime モジュールは:
- Core の状態遷移関数 (`step`, `process` 等) を呼び出す
- Core からの **Action** (命令) を実際のI/O操作に変換する
- 外部イベント (ソケット受信、タイマー発火) を Core の **Event** に変換する

---

## 4. メリット

| メリット | 効果 |
|---|---|
| **テスト容易性** | Core モジュールは I/O モック不要で単体テスト可能<br>モジュール境界による明確な責務分離 |
| **依存性の明確化** | モジュール構造で論理的に依存を分離<br>明確なインターフェース定義 |
| **ポータビリティ** | Core ロジックは WASM や他のランタイムへ移植可能 |
| **並行開発** | Core と Runtime モジュールを別担当者で並行開発可能 |

---

## 5. 実装例: DFL Session Manager

```rust
// cdde-dfl/src/core/session.rs (Domain Layer)
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

// cdde-dfl/src/runtime/session_actor.rs (Infrastructure Layer)
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
