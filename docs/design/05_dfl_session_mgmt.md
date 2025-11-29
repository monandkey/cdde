
-----

## 1. 採用技術とアーキテクチャ

DFLセッション管理は、外部DBへのアクセスを避け、プロセス内のメモリで完結させる **インメモリ戦略** を採用します。

  * **トランザクションストア:** **`dashmap::DashMap`**
      * Rustにおける高性能な並行ハッシュマップです。非同期処理環境でロック競合を最小限に抑え、トランザクションの検索・挿入・削除を高速に行います。
  * **タイムアウトスケジューラ:** **`tokio_util::time::DelayQueue`**
      * 大量のタイマーを効率的に管理するための非同期ユーティリティです。それぞれのトランザクションに対して個別のタイマーを設定し、期限が切れたキーだけを効率的に取り出すことができます。
  * **キー構造:** `(u64, u32)` $\rightarrow$ **`(Connection ID, Hop-by-Hop ID)`**
      * これにより、システム全体でトランザクションを一意に識別します。

### 1.5. アーキテクチャ: Core/Runtime 分離

DFL セッション管理は **Sans-IO Core + Actor Runtime** パターンを採用し、ロジックとI/Oを分離します。

#### 1.5.1. Core Layer: SessionManagerCore

**責務:** セッション状態の管理と判断ロジック（純粋関数）

```rust
// cdde-dfl-core/src/domain/session.rs
pub struct SessionManagerCore {
    sessions: HashMap<SessionKey, SessionState>,
    config: SessionConfig,
}

impl SessionManagerCore {
    // イベント1: Request受信
    pub fn on_request_received(
        &mut self, 
        conn_id: u64, 
        msg: DiameterMessage,
        now: Instant
    ) -> Vec<SessionAction> {
        let key = SessionKey { connection_id: conn_id, hop_by_hop_id: msg.hop_by_hop_id };
        self.sessions.insert(key, SessionState { created_at: now, original_msg: msg.clone() });
        vec![SessionAction::ForwardToDcr(msg)]
    }

    // イベント2: Answer受信
    pub fn on_answer_received(
        &mut self, 
        conn_id: u64, 
        msg: DiameterMessage
    ) -> Vec<SessionAction> {
        let key = SessionKey { connection_id: conn_id, hop_by_hop_id: msg.hop_by_hop_id };
        if self.sessions.remove(&key).is_some() {
            vec![SessionAction::RemoveSession(key)]
        } else {
            vec![SessionAction::Discard]  // タイムアウト済み
        }
    }

    // イベント3: タイムアウト発火
    pub fn on_timeout(&mut self, key: SessionKey) -> Vec<SessionAction> {
        if self.sessions.remove(&key).is_some() {
            vec![SessionAction::ReplyWith3002(key)]
        } else {
            vec![]  // 既に処理済み
        }
    }
}
```

**Event/Action パターン:**

| Event (入力) | Action (出力命令) |
|---|---|
| `on_request_received` | `ForwardToDcr(msg)` - DCRへ転送せよ |
| `on_answer_received` | `RemoveSession(key)` - セッション削除<br>`Discard` - パケット破棄 |
| `on_timeout` | `ReplyWith3002(key)` - エラー応答送信 |

#### 1.5.2. Runtime Layer: SessionActor

**責務:** I/O操作とタイマー管理（Tokio Actor）

```rust
// cdde-dfl-runtime/src/actor/session_actor.rs
pub struct SessionActor {
    core: SessionManagerCore,
    receiver: mpsc::Receiver<ActorMessage>,
    timeout_queue: DelayQueue<SessionKey>,
    outbound_tx: mpsc::Sender<SessionAction>,
}

impl SessionActor {
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                // 外部からのメッセージ受信
                Some(msg) = self.receiver.recv() => {
                    self.handle_message(msg).await;
                }
                // タイムアウト発火
                Some(expired) = self.timeout_queue.next() => {
                    let key = expired.into_inner();
                    let actions = self.core.on_timeout(key);
                    self.execute_actions(actions).await;
                }
            }
        }
    }

    async fn handle_message(&mut self, msg: ActorMessage) {
        let actions = match msg {
            ActorMessage::IngressRequest { conn_id, msg } => {
                let key = SessionKey { connection_id: conn_id, hop_by_hop_id: msg.hop_by_hop_id };
                // タイマーセット
                self.timeout_queue.insert(key, self.core.config.timeout_duration);
                // Coreロジック実行
                self.core.on_request_received(conn_id, msg, Instant::now())
            },
            ActorMessage::IngressAnswer { conn_id, msg } => {
                self.core.on_answer_received(conn_id, msg)
            }
        };
        self.execute_actions(actions).await;
    }

    async fn execute_actions(&self, actions: Vec<SessionAction>) {
        for action in actions {
            match action {
                SessionAction::ForwardToDcr(msg) => {
                    // gRPC Client呼び出し
                },
                SessionAction::ReplyWith3002(key) => {
                    // SCTP Socket書き込み
                },
                _ => {}
            }
        }
    }
}
```

**メリット:**
- **テスト容易性:** `SessionManagerCore` は I/O モック不要で単体テスト可能
- **決定論的:** タイムアウトロジックを時間に依存せずテスト可能
- **スレッド安全:** Actor内で逐次処理されるため、Mutex不要


## 2. データ構造の定義 (Rust Structs)

`DashMap` に保存される、トランザクションのコンテキスト情報です。

### A. トランザクションコンテキスト (`TransactionContext`)

```rust
// SessionManagement/src/context.rs

use std::time::Instant;
use tokio_util::time::delay_queue::Key;

pub struct TransactionContext {
    // ------------------------------------------------
    // 1. タイムアウト制御用
    // ------------------------------------------------
    // DelayQueueのKeyを保持し、Answer受信時にタイマーをキャンセル可能にする
    pub delay_queue_key: Key, 
    pub ingress_timestamp: Instant,             // 受信時刻 (経過時間計測用)

    // ------------------------------------------------
    // 2. エラー応答 (3002) 生成用
    // ------------------------------------------------
    pub source_connection_id: u64,              // 送信元のSCTP接続ID
    pub original_command_code: u32,             // 元のコマンドコード (CCR, ULRなど)
    pub original_end_to_end_id: u32,            // 元のEnd-to-End ID (必須)
    
    // ------------------------------------------------
    // 3. 監査・ログ用
    // ------------------------------------------------
    pub session_id: String,                     // Diameter Session-ID AVP値
}
```

### B. ストア定義 (`TransactionStore`)

```rust
// SessionManagement/src/store.rs
use dashmap::DashMap;

// Key: (Connection ID, Hop-by-Hop ID)
// Value: TransactionContext
pub type TransactionStore = DashMap<(u64, u32), TransactionContext>; 
```

-----

## 3. トランザクション処理フローの詳細

DFLのセッション管理ロジックは、以下の3つの非同期タスクに分かれます。

### 3.1. Request受信時 (Ingress Logic)

DFLは外部PeerからRequestを受信した際、以下の処理を行います。

1.  **キー取得:** Requestヘッダから `Hop-by-Hop ID` と `End-to-End ID` をパース。
2.  **コンテキスト作成:** `TransactionContext` を作成し、応答に必要な情報（コマンドコード、各種ID、送信元接続ID）を格納。
3.  **タイマー設定:**
      * 設定されたタイムアウト時間（例：5秒）に基づき、**`DelayQueue`** に `(ConnectionID, Hop-by-Hop ID)` の複合キーをエンキューする。
      * エンキュー時に返される **`Key`** を `TransactionContext` に保存。
4.  **ストア保存:** `TransactionStore` にキーとコンテキストを挿入。
5.  **DCRへ転送:** DCR ServiceへgRPC (`DiameterPacketRequest`) を送信。

### 3.2. Answer受信時 (Egress Logic)

Answer（応答）が外部Peerから、またはDCRからの指示 (`ActionType::REPLY`) でDFLに戻ってきた際の処理です。

1.  **キー検索:** Answerヘッダから `Hop-by-Hop ID` を取得し、ストアを検索。
2.  **トランザクション完了判定:**
      * **Hit:** トランザクションは有効。
          * `TransactionContext` を取得し、`delay_queue_key` を用いてタイマーを**キャンセル**する。
          * `TransactionStore` からエントリを削除。
          * Answerを適切なPeerへ転送。
      * **Miss:** トランザクションが存在しない。
          * **要件:** トランザクションが失われた、または既にタイムアウト処理が実行されたことを意味します。
          * ログを記録し、Answerパケットを**サイレントに破棄**する。

### 3.3. タイムアウトイベント発火時 (Timeout Handling)

`DelayQueue` の専用タスクは、タイマーが期限切れになったキーを非同期に処理します。

1.  **イベント取得:** `DelayQueue` から期限切れのキー (`(ConnectionID, Hop-by-Hop ID)`) が排出される。
2.  **ストア確認:** `TransactionStore` を検索し、エントリが存在するか確認。
3.  **タイムアウト処理実行:**
      * **Hit:** 応答が来る前に時間が切れた。
          * `TransactionStore` からエントリを削除。
          * 格納された `original_command_code` や `original_end_to_end_id` を使用して、**`Result-Code: 3002 (DIAMETER_UNABLE_TO_DELIVER)`** を含むエラー応答パケットを生成。
          * エラー応答を `source_connection_id` に向けて送信元Peerへ返信。
      * **Miss:** Answer受信時に既に削除されている（正常完了）。
          * 何もしない。
