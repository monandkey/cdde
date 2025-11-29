# CDDE Design Enhancement Reference

| **項目** | **内容** |
|---|---|
| **ドキュメント名** | Design Enhancement Reference |
| **版数** | 2.0 (Streamlined) |
| **作成日** | 2025-11-29 |
| **目的** | CDDE設計強化の経緯と採用パターンの要約 |

---

## 1. 設計強化の背景

CDDEの設計を強化するため、以下のアーキテクチャパターンの採用を検討・決定しました:

1. **Rust Workspace + DDD** - モジュール境界の明確化
2. **Functional DDD** - 型駆動設計による安全性向上
3. **Sans-IO Core + Actor Runtime** - ロジックとI/Oの分離

これらのパターンは、Diameterプロトコルのような複雑なネットワークソフトウェアに適しており、高い信頼性とテスト容易性を実現します。

---

## 2. 採用アーキテクチャパターン

### 2.1. Sans-IO Core + Actor Runtime

**概要:** ビジネスロジック（Core）と副作用（Runtime）を分離

| レイヤー | 責務 | 技術 |
|---|---|---|
| **Core** | 判断・状態遷移 | 純粋関数、I/O依存なし |
| **Runtime** | I/O実行 | Tokio Actor、ソケット、タイマー |

**メリット:**
- テスト容易性（I/Oモック不要）
- 決定論的動作
- ポータビリティ（WASM等への移植が容易）

**参照:** [18_architecture_patterns.md](file:///workspace/docs/design/18_architecture_patterns.md) Section 1

### 2.2. Functional DDD

**概要:** 型による状態遷移とpure functionによるドメインロジック

**原則:**
1. 状態ごとに別の型を定義（Type-State Pattern）
2. メソッドではなく純粋関数として実装
3. 小さな関数をパイプラインで組み合わせ

**例:**
```rust
// 状態ごとの型
pub struct UnvalidatedOrder { ... }
pub struct ValidatedOrder { ... }

// 状態遷移関数
pub fn validate_order(input: UnvalidatedOrder) 
    -> Result<ValidatedOrder, ValidationError>
```

**参照:** [18_architecture_patterns.md](file:///workspace/docs/design/18_architecture_patterns.md) Section 2

### 2.3. Rust Workspace + DDD Layer Mapping

**概要:** DDD層をRust Workspaceのクレート構造にマッピング

| DDD層 | Rust Workspace | 依存方向 |
|---|---|---|
| Domain | `{component}-core` | 外部依存なし |
| Application | `{component}-core` 内 | Domain Layer |
| Infrastructure | `{component}-runtime` | Application, Domain |
| Presentation | `{component}` binary | すべての層 |

**参照:** [03_rust_module_plan.md](file:///workspace/docs/design/03_rust_module_plan.md)

---

## 3. CDDE コンポーネントへの適用

### 3.1. DFL (Diameter Frontline)

**Core:** `SessionManagerCore`
- セッション状態管理FSM
- タイムアウト判定ロジック（純粋関数）

**Runtime:** `SessionActor`
- SCTP I/O
- `DelayQueue` タイマー管理
- gRPC Client (DCR呼び出し)

**参照:** [05_dfl_session_mgmt.md](file:///workspace/docs/design/05_dfl_session_mgmt.md) Section 1.5

### 3.2. DPA (Diameter Peer Agent)

**Core:** `PeerFsm`
- RFC 6733 準拠のピア状態遷移マシン
- Watchdogロジック（純粋関数）

**Runtime:** `PeerActor`
- SCTP Heartbeat送信
- DWR/DWA処理
- DFL通知

**参照:** [07_dpa_detail.md](file:///workspace/docs/design/07_dpa_detail.md) Section 2

### 3.3. DCR (Diameter Core Router)

**Core:** `RouterCore` + `ManipulationEngine`
- ルーティング判断
- AVP書き換え、Topology Hiding

**Runtime:** `DcrService`
- gRPC Server
- `ArcSwap`による動的設定更新

**参照:** 
- [06_dcr_manipulation.md](file:///workspace/docs/design/06_dcr_manipulation.md) Section 1.3
- [09_dcr_routing_logic.md](file:///workspace/docs/design/09_dcr_routing_logic.md) Section 2.2

---

## 4. 主要な技術的決定事項

### 4.1. Event/Action パターン

Core と Runtime の間のインターフェースは Event（入力）と Action（出力命令）で定義:

```rust
// Core への入力
pub enum Event {
    PacketReceived(Bytes),
    TimerExpiry,
}

// Core からの出力
pub enum Action {
    SendBytes(Vec<u8>),
    ResetTimer,
}

// Core の状態遷移関数
pub fn step(&mut self, event: Event) -> Vec<Action>
```

### 4.2. Zero-Copy with `bytes::Bytes`

参照カウント付きバッファを使用してメモリコピーを最小化:

```rust
pub struct Avp {
    pub code: u32,
    pub data: Bytes,  // 参照カウント付き
}
```

### 4.3. Lock-Free Configuration Updates with `ArcSwap`

トラフィック処理を止めずに設定変更を実現:

```rust
pub struct DcrService {
    core: Arc<ArcSwap<RouterCore>>,
}

// 無停止更新
pub fn update_config(&self, new_core: RouterCore) {
    self.core.store(Arc::new(new_core));
}
```

---

## 5. テスト戦略

### 5.1. Core Layer のテスト

**特徴:** I/O モック不要、決定論的

```rust
#[test]
fn test_peer_fsm_watchdog_timeout() {
    let mut fsm = PeerFsm::new(config);
    fsm.step(FsmEvent::Start);
    fsm.step(FsmEvent::ConnectionUp);
    // ... I/Oモック不要で状態遷移をテスト
}
```

### 5.2. Property-Based Testing

ランダムなイベント列で状態機械の堅牢性を検証:

```rust
proptest! {
    #[test]
    fn test_fsm_never_panics(events in vec(arb_event(), 0..100)) {
        let mut fsm = PeerFsm::new(config);
        for event in events {
            let _ = fsm.step(event);
        }
    }
}
```

**参照:** [13_testing_strategy.md](file:///workspace/docs/design/13_testing_strategy.md) Section 2.4

---

## 6. 設計ドキュメントへの反映状況

以下のドキュメントに設計強化内容を反映済み:

| ドキュメント | 更新内容 |
|---|---|
| [02_basic_design.md](file:///workspace/docs/design/02_basic_design.md) | Section 2.4: Sans-IO パターン追加 |
| [03_rust_module_plan.md](file:///workspace/docs/design/03_rust_module_plan.md) | Core/Runtime クレート分離構造 |
| [05_dfl_session_mgmt.md](file:///workspace/docs/design/05_dfl_session_mgmt.md) | Section 1.5: SessionManagerCore/Actor |
| [06_dcr_manipulation.md](file:///workspace/docs/design/06_dcr_manipulation.md) | Section 1.3: ManipulationEngine |
| [07_dpa_detail.md](file:///workspace/docs/design/07_dpa_detail.md) | Section 2: PeerFsm/Actor |
| [09_dcr_routing_logic.md](file:///workspace/docs/design/09_dcr_routing_logic.md) | Section 2.2: RouterCore + ArcSwap |
| [13_testing_strategy.md](file:///workspace/docs/design/13_testing_strategy.md) | Section 2.4: Sans-IO テスト戦略 |
| [18_architecture_patterns.md](file:///workspace/docs/design/18_architecture_patterns.md) | **新規作成** - 包括的リファレンス |

---

## 7. 実装への影響

### 7.1. クレート構造の変更

**変更前:**
```
cdde-dfl/
  └── src/
```

**変更後:**
```
cdde-dfl-core/      # Domain Layer (純粋関数)
cdde-dfl-runtime/   # Infrastructure Layer (I/O)
cdde-dfl/           # Presentation Layer (main)
```

### 7.2. 依存関係の制約

**Core クレートは以下に依存してはならない:**
- ❌ `tokio` (非同期ランタイム)
- ❌ `tonic` (gRPC)
- ❌ `sctp`, `tokio::net` (I/O)

**Core クレートが依存して良いもの:**
- ✅ `cdde-shared` (共通型)
- ✅ `serde`, `thiserror`
- ✅ `std::collections`

---

## 8. 次のステップ

### 8.1. 開発チーム向け

1. **アーキテクチャパターンの理解**
   - [18_architecture_patterns.md](file:///workspace/docs/design/18_architecture_patterns.md) を熟読

2. **既存コードのリファクタリング**
   - Core/Runtime クレート分離
   - Event/Action パターンの実装

3. **テスト戦略の実装**
   - Core コンポーネントの単体テスト
   - Property-Based Testing の導入

### 8.2. 参考資料

**Rust製ネットワークライブラリの実装例:**
- `quinn` (QUIC) - Sans-IO パターンの実装
- `h2` (HTTP/2) - ステートマシンの実装

**関連ドキュメント:**
- RFC 6733 (Diameter Base Protocol) - Peer状態遷移
- DDD (Domain-Driven Design) - Eric Evans

---

## 付録: 用語集

| 用語 | 説明 |
|---|---|
| **Sans-IO** | I/O操作を含まない純粋なロジック層 |
| **Actor Model** | メッセージパッシングによる並行処理モデル |
| **Type-State Pattern** | 状態を型で表現し、不正な状態遷移をコンパイル時に防ぐ |
| **Pure Function** | 副作用がなく、同じ入力に対して常に同じ出力を返す関数 |
| **ArcSwap** | ロックフリーなアトミックポインタ更新 |
| **Property-Based Testing** | ランダムな入力で不変条件を検証するテスト手法 |

---

**変更履歴:**
- v2.0 (2025-11-29): Q&A形式から要約形式に整理、設計ドキュメント反映完了
- v1.0 (2025-11-28): 初版作成（Q&A形式）
