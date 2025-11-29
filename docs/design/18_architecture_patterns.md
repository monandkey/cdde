# CDDE Architecture Patterns Reference

| **項目** | **内容** |
|---|---|
| **ドキュメント名** | Architecture Patterns Reference |
| **版数** | 1.0 |
| **対象** | アーキテクト, 開発チーム |
| **目的** | CDDE で採用するアーキテクチャパターンの包括的なリファレンス |

---

## 1. Sans-IO Core + Actor Runtime パターン

### 1.1. パターンの概要

**Sans-IO (I/O-free) Core + Actor Runtime** は、ビジネスロジック（判断・状態遷移）と副作用（I/O操作）を明確に分離するアーキテクチャパターンです。

このパターンは、`quinn` (QUIC実装) や `h2` (HTTP/2実装) など、高性能なRust製ネットワークライブラリで広く採用されています。

### 1.2. 構成要素

| レイヤー | 責務 | 技術的特徴 |
|---|---|---|
| **Core (Logic)** | **判断・状態遷移**<br>ビジネスルール、プロトコル状態管理 | **純粋関数**<br>I/O（ソケット、タイマー）や `async/await` に依存しない<br>`Result` 型でエラーを表現 |
| **Runtime (Shell)** | **I/O実行**<br>ソケット読み書き、タイマー発火、プロセス間通信 | **Tokio Actor**<br>Coreからの命令（Action）を受け取り実行<br>イベントをCoreに通知 |

### 1.3. Event/Action パターン

Core と Runtime の間のインターフェースは、**Event** (入力) と **Action** (出力命令) で定義されます。

```rust
// Core への入力
pub enum Event {
    PacketReceived(Bytes),
    TimerExpiry,
    ConnectionUp,
}

// Core からの出力
pub enum Action {
    SendBytes(Vec<u8>),
    ResetTimer,
    NotifyExternal(String),
}

// Core の状態遷移関数
impl Core {
    pub fn step(&mut self, event: Event) -> Vec<Action> {
        // ロジックのみ、I/Oなし
    }
}
```

### 1.4. メリット

| メリット | 詳細 |
|---|---|
| **テスト容易性** | Core ロジックは I/O モック不要で単体テスト可能<br>決定論的なテストが書ける |
| **ポータビリティ** | Core ロジックは WASM や他のランタイムへ移植可能<br>トランスポート層（TCP/SCTP）の切り替えが容易 |
| **並行性** | Actor モデルにより、ロック競合を回避し高スループットを実現 |
| **デバッグ容易性** | Core は I/O タイミングに依存せず、動作が決定論的 |

### 1.5. ネットワークソフトウェアへの適用

| ソフトウェアの種類 | 適用推奨度 | 理由 |
|---|---|---|
| **コントロールプレーン**<br>(SDNコントローラ、設定管理、ハンドシェイク処理) | ★★★★★ (最適) | 状態遷移が複雑で、安全性と正確性が最優先されるため |
| **アプリケーション層プロトコル**<br>(HTTP/2, MQTT, Diameter) | ★★★★☆ (推奨) | 仕様が複雑で、テスト容易性の恩恵が大きい |
| **データプレーン / パケットフォワーダ**<br>(ルーター、ファイアウォール) | ★☆☆☆☆ (非推奨) | パケットごとの数ナノ秒の遅延が命取り |

---

## 2. Functional DDD in Rust

### 2.1. 設計原則

関数型DDDは、以下の3つの柱で構成されます:

1. **型による状態遷移 (Type-State Pattern)**
   - 単一の大きなStruct（例: `Order`）で内部状態フラグ（`status: string`）を持つのではなく、状態ごとに別の型を定義
   - 不正な状態（未払いなのに出荷済みなど）をコンパイルレベルで表現不可能にする

2. **純粋関数によるドメインロジック**
   - メソッド（`&mut self`）ではなく、入力を受け取って新しい出力を返す関数として定義
   - 副作用（DB保存など）はドメイン層から排除し、戻り値として「次にすべきこと（イベント）」を返す

3. **パイプライン処理**
   - 小さな関数をつなぎ合わせてビジネスフロー（ワークフロー）を構築

### 2.2. 型駆動設計の例

```rust
// 状態ごとの型定義
pub struct UnvalidatedOrder { ... }
pub struct ValidatedOrder { ... }
pub struct PricedOrder { ... }

// 状態遷移関数
pub fn validate_order(input: UnvalidatedOrder) 
    -> Result<ValidatedOrder, ValidationError> 
{
    // バリデーションロジック
}

pub fn price_order(order: ValidatedOrder, unit_price: Money) 
    -> PricedOrder 
{
    // 価格計算ロジック
}

// パイプライン
let priced = validate_order(unvalidated)?
    .map(|v| price_order(v, price));
```

**メリット:**
- **型安全性:** `price_order` は `ValidatedOrder` 型しか受け付けない
- **コンパイル時検証:** バリデーションをスキップして価格計算するコードはコンパイルエラー
- **テスト容易性:** 純粋関数なので、入力→出力のテストのみ

### 2.3. Diameter プロトコルへの適用

Diameter の Peer 状態遷移 (RFC 6733) を型で表現:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PeerState {
    Closed,
    WaitICEA,  // CER送信済み、CEA待ち
    Open,      // 通信可能
}

// 状態遷移関数
pub fn step(&mut self, event: FsmEvent) -> Vec<FsmAction> {
    match (self.state, event) {
        (PeerState::WaitICEA, FsmEvent::MessageReceived(msg)) if msg.is_cea() => {
            self.state = PeerState::Open;
            vec![FsmAction::NotifyDflUp, FsmAction::ResetWatchdogTimer]
        }
        // ...
    }
}
```

---

## 3. Rust Workspace + DDD Layer Mapping

### 3.1. DDD層とRust Workspaceの対応

| DDD層 | Rust Workspace対応 | 責務 | 依存方向 |
|---|---|---|---|
| **Domain Layer** | `{component}/src/core/` モジュール | 純粋なビジネスロジック、状態遷移、ドメインモデル | 外部依存なし（`cdde-shared`のみ） |
| **Application Layer** | `{component}/src/app/` モジュール | ドメインロジックのオーケストレーション | Domain Layer |
| **Infrastructure Layer** | `{component}/src/runtime/` モジュール | I/O操作、永続化、外部システム連携 | Application Layer, Domain Layer |
| **Presentation Layer** | `{component}/src/main.rs` | エントリーポイント、DIコンテナ組み立て | すべての層 |

### 3.2. 依存性逆転の原則 (DIP)

**原則:** 高レベルモジュール（Domain）は低レベルモジュール（Infrastructure）に依存してはならない

**実装:**
- Repository の **インターフェース（Trait）** を Application Layer で定義
- **実装** を Infrastructure Layer で提供
- Presentation Layer で具体的な実装を注入 (DI)

```rust
// Application Layer (cdde-dfl/src/core)
pub trait SessionRepository {
    fn save(&self, session: &Session) -> Result<()>;
}

// Infrastructure Layer (cdde-dfl-runtime)
pub struct InMemorySessionRepository { ... }
impl SessionRepository for InMemorySessionRepository { ... }

// Presentation Layer (cdde-dfl)
fn main() {
    let repo = InMemorySessionRepository::new();
    let use_case = SessionUseCase::new(Arc::new(repo));
}
```

---

## 4. Zero-Copy Techniques

### 4.1. `bytes::Bytes` の活用

**目的:** 受信したパケットのメモリをコピーせず、参照カウント付きバッファで共有

```rust
use bytes::Bytes;

pub struct DiameterMessage {
    pub avps: Vec<Avp>,
}

pub struct Avp {
    pub code: u32,
    pub data: Bytes,  // ★ 参照カウント付き
}

// メッセージをクローンしてもデータはコピーされない
let msg2 = msg1.clone();  // 高速
```

### 4.2. Microservices とのトレードオフ

**課題:** DFL ↔ DCR 間は gRPC (Protobuf) で通信するため、シリアライズ/デシリアライズ（コピー）が発生

**対策:**
- **Phase 1:** gRPC で進める（開発効率優先）
- **Phase 2以降:** 同一Node内にPodを配置し、共有メモリや Unix Domain Socket への切り替えを検討

---

## 5. Actor Model for Concurrency

### 5.1. Actor パターンの概要

**原則:** 各 Actor は独立したタスクとして動作し、メッセージパッシングで通信

```rust
pub struct SessionActor {
    core: SessionManagerCore,
    receiver: mpsc::Receiver<ActorMessage>,
}

impl SessionActor {
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(msg) = self.receiver.recv() => {
                    let actions = self.core.step(msg);
                    self.execute_actions(actions).await;
                }
            }
        }
    }
}
```

### 5.2. メリット

| メリット | 詳細 |
|---|---|
| **ロック不要** | Actor内で逐次処理されるため、Mutex不要 |
| **並行性** | 複数のActorが並行動作し、スケーラビリティ向上 |
| **障害分離** | 1つのActorの障害が他のActorに波及しない |

### 5.3. CDDE での適用

| コンポーネント | Actor 単位 | 数 |
|---|---|---|
| **DFL** | セッション管理Actor | 1 per DFL Pod |
| **DPA** | Peer Actor | 1 per Peer |
| **DCR** | gRPC Server Actor | 1 per VR |

---

## 6. Lock-Free Patterns with ArcSwap

### 6.1. 動的設定更新の課題

**課題:** ルーティングテーブルの更新時、トラフィック処理を止めたくない

**従来の方法 (RwLock):**
```rust
let table = Arc::new(RwLock::new(routing_table));

// 読み取り時
let guard = table.read().unwrap();  // ★ Lock発生
let route = guard.find(...);

// 更新時
let mut guard = table.write().unwrap();  // ★ 全読み取りをブロック
*guard = new_table;
```

### 6.2. ArcSwap による解決

```rust
use arc_swap::ArcSwap;

let table = Arc::new(ArcSwap::from_pointee(routing_table));

// 読み取り時
let guard = table.load();  // ★ ロックフリー
let route = guard.find(...);

// 更新時
table.store(Arc::new(new_table));  // ★ アトミック、ブロックなし
```

**メリット:**
- **無停止更新:** トラフィック処理を止めずに設定変更可能
- **高スループット:** Read Lock が発生しないため、パフォーマンス劣化なし

---

## 7. Testing Strategies for Sans-IO

### 7.1. Core Layer のテスト

**特徴:** I/O モック不要、決定論的

```rust
#[test]
fn test_peer_fsm_watchdog_timeout() {
    let mut fsm = PeerFsm::new(config);
    
    // 1. Open状態にする
    fsm.step(FsmEvent::Start);
    fsm.step(FsmEvent::ConnectionUp);
    fsm.step(FsmEvent::MessageReceived(cea_message()));
    assert_eq!(fsm.current_state(), PeerState::Open);
    
    // 2. Watchdogタイムアウトを3回発生させる
    for _ in 0..3 {
        let actions = fsm.step(FsmEvent::WatchdogTimerExpiry);
        assert!(actions.contains(&FsmAction::SendBytes(_)));
    }
    
    // 3. 4回目でDOWN判定
    let actions = fsm.step(FsmEvent::WatchdogTimerExpiry);
    assert!(actions.contains(&FsmAction::NotifyDflDown));
    assert_eq!(fsm.current_state(), PeerState::Closed);
}
```

### 7.2. Property-Based Testing

**目的:** ランダムなイベント列で状態機械の堅牢性を検証

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_fsm_never_panics(events in prop::collection::vec(any::<FsmEvent>(), 0..100)) {
        let mut fsm = PeerFsm::new(config);
        for event in events {
            let _ = fsm.step(event);  // パニックしないことを検証
        }
    }
}
```

---

## 8. まとめ

CDDE は以下のアーキテクチャパターンを組み合わせることで、キャリアグレードの信頼性と高性能を実現します:

1. **Sans-IO Core + Actor Runtime** - ロジックとI/Oの分離
2. **Functional DDD** - 型駆動設計による安全性
3. **Rust Workspace + DDD** - 明確なレイヤー分離
4. **Zero-Copy** - 高性能なメモリ管理
5. **Actor Model** - スケーラブルな並行処理
6. **ArcSwap** - 無停止設定更新

これらのパターンは、個別に適用するのではなく、**統合的に組み合わせる**ことで最大の効果を発揮します。
