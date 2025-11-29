
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

---

## 2. アーキテクチャ: Peer FSM 設計 (Sans-IO Core + Actor Runtime)

DPA は **Sans-IO Core + Actor Runtime** パターンを採用し、RFC 6733 準拠のピア状態遷移ロジックとI/O操作を分離します。

### 2.1. Core Layer: PeerFsm (Finite State Machine)

**責務:** RFC 6733 Sec 5.6 に準拠したピア状態遷移ロジック（純粋関数）

#### 2.1.1. 状態定義 (PeerState)

```rust
// cdde-dpa/src/core/fsm.rs
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PeerState {
    Closed,       // 初期状態、切断済み
    WaitConnAck,  // TCP/SCTP接続試行中
    WaitICEA,     // Initiator: CER送信済み、CEA待ち
    Open,         // 通信可能状態 (UP)
    Closing,      // 切断処理中
}
```

#### 2.1.2. イベント定義 (FsmEvent)

```rust
pub enum FsmEvent {
    Start,                          // 起動指示
    ConnectionUp,                   // TCP/SCTP接続完了
    ConnectionFailed,               // 接続失敗
    MessageReceived(DiameterMessage), // CER, DWR, DWA, DPR等
    WatchdogTimerExpiry,            // Watchdogタイマー発火
    DisconnectRequest,              // 管理者からの切断指示
}
```

#### 2.1.3. アクション定義 (FsmAction)

```rust
pub enum FsmAction {
    ConnectToPeer,              // ソケット接続を開始せよ
    DisconnectPeer,             // ソケットを切断せよ
    SendBytes(Vec<u8>),         // データ送信せよ (CER, DWR, DWA等)
    ResetWatchdogTimer,         // Watchdogタイマーをリセットせよ
    NotifyDflUp,                // DFLへ「Peer UP」を通知せよ
    NotifyDflDown,              // DFLへ「Peer DOWN」を通知せよ
    Log(String),                // ログ出力
}
```

#### 2.1.4. 状態遷移ロジック

```rust
pub struct PeerFsm {
    state: PeerState,
    config: PeerConfig,
    watchdog_failures: u32,  // 連続失敗回数カウンタ
}

impl PeerFsm {
    // ★ Core Logic: 状態遷移関数 (純粋関数)
    pub fn step(&mut self, event: FsmEvent) -> Vec<FsmAction> {
        let mut actions = Vec::new();

        match (self.state, event) {
            // 起動シーケンス
            (PeerState::Closed, FsmEvent::Start) => {
                self.state = PeerState::WaitConnAck;
                actions.push(FsmAction::ConnectToPeer);
            }

            // 接続確立 -> CER送信
            (PeerState::WaitConnAck, FsmEvent::ConnectionUp) => {
                self.state = PeerState::WaitICEA;
                let cer_bytes = build_cer(); // CER構築
                actions.push(FsmAction::SendBytes(cer_bytes));
            }

            // CEA受信 -> Open (UP)
            (PeerState::WaitICEA, FsmEvent::MessageReceived(msg)) if msg.is_cea() => {
                self.state = PeerState::Open;
                self.watchdog_failures = 0;
                actions.push(FsmAction::NotifyDflUp);
                actions.push(FsmAction::ResetWatchdogTimer);
            }

            // Open状態: Watchdogタイマー発火
            (PeerState::Open, FsmEvent::WatchdogTimerExpiry) => {
                if self.watchdog_failures >= self.config.max_watchdog_failures {
                    // タイムアウト上限超過 -> DOWN判定
                    self.state = PeerState::Closed;
                    actions.push(FsmAction::NotifyDflDown);
                    actions.push(FsmAction::DisconnectPeer);
                } else {
                    // DWR送信
                    self.watchdog_failures += 1;
                    let dwr_bytes = build_dwr();
                    actions.push(FsmAction::SendBytes(dwr_bytes));
                    actions.push(FsmAction::ResetWatchdogTimer);
                }
            }

            // Open状態: メッセージ受信 (DWR/DWA)
            (PeerState::Open, FsmEvent::MessageReceived(msg)) => {
                self.watchdog_failures = 0;  // 生存確認
                actions.push(FsmAction::ResetWatchdogTimer);

                if msg.is_dwr() {
                    // DWR受信 -> DWA応答
                    let dwa_bytes = build_dwa(&msg);
                    actions.push(FsmAction::SendBytes(dwa_bytes));
                } else if msg.is_dwa() {
                    // DWA受信 -> 生存確認完了
                    actions.push(FsmAction::Log("DWA received. Peer is healthy.".into()));
                } else if msg.is_dpr() {
                    // DPR受信 -> 切断
                    self.state = PeerState::Closed;
                    let dpa_bytes = build_dpa(&msg);
                    actions.push(FsmAction::SendBytes(dpa_bytes));
                    actions.push(FsmAction::NotifyDflDown);
                    actions.push(FsmAction::DisconnectPeer);
                }
            }

            // 異常系
            _ => {
                actions.push(FsmAction::Log(format!("Invalid transition: {:?} for state {:?}", event, self.state)));
            }
        }

        actions
    }
}
```

**メリット:**
- **RFC 6733 完全準拠:** 状態遷移が仕様通りに実装され、コンパイル時に検証可能
- **テスト容易性:** I/O なしで全ての状態遷移パターンを単体テスト可能
- **型安全性:** 無効な状態遷移はコンパイルエラーまたは明示的なエラーアクション

### 2.2. Runtime Layer: PeerActor

**責務:** SCTP I/O操作とタイマー管理（Tokio Actor）

```rust
// cdde-dpa-runtime/src/peer_actor.rs
pub struct PeerActor {
    core: PeerFsm,
    peer_addr: String,
    socket: Option<TcpStream>,  // または SctpStream
    dfl_notifier: mpsc::Sender<String>,
    watchdog_timer: Interval,
}

impl PeerActor {
    pub async fn run(&mut self) {
        // 最初にStartイベントを投入
        self.handle_event(FsmEvent::Start).await;

        let mut buf = [0u8; 4096];

        loop {
            let event = if let Some(socket) = &mut self.socket {
                tokio::select! {
                    // ソケットからの受信
                    res = socket.read(&mut buf) => {
                        match res {
                            Ok(0) => FsmEvent::ConnectionFailed,
                            Ok(n) => {
                                let msg = parse_diameter(&buf[..n]);
                                FsmEvent::MessageReceived(msg)
                            }
                            Err(_) => FsmEvent::ConnectionFailed,
                        }
                    }
                    // Watchdogタイマー発火
                    _ = self.watchdog_timer.tick() => {
                        FsmEvent::WatchdogTimerExpiry
                    }
                }
            } else {
                // ソケットがない状態の待機
                tokio::select! {
                    _ = self.watchdog_timer.tick() => {
                        FsmEvent::WatchdogTimerExpiry
                    }
                }
            };

            self.handle_event(event).await;
        }
    }

    async fn handle_event(&mut self, event: FsmEvent) {
        // Coreを回す
        let actions = self.core.step(event);

        // アクションを実行 (I/O)
        for action in actions {
            match action {
                FsmAction::ConnectToPeer => {
                    match TcpStream::connect(&self.peer_addr).await {
                        Ok(stream) => {
                            self.socket = Some(stream);
                            self.core.step(FsmEvent::ConnectionUp);
                        }
                        Err(_) => {
                            self.core.step(FsmEvent::ConnectionFailed);
                        }
                    }
                }
                FsmAction::DisconnectPeer => {
                    self.socket = None;  // Dropによる切断
                }
                FsmAction::SendBytes(data) => {
                    if let Some(socket) = &mut self.socket {
                        let _ = socket.write_all(&data).await;
                    }
                }
                FsmAction::ResetWatchdogTimer => {
                    self.watchdog_timer.reset();
                }
                FsmAction::NotifyDflUp => {
                    let _ = self.dfl_notifier.send(format!("UP: {}", self.peer_addr)).await;
                }
                FsmAction::NotifyDflDown => {
                    let _ = self.dfl_notifier.send(format!("DOWN: {}", self.peer_addr)).await;
                }
                FsmAction::Log(msg) => {
                    println!("[DPA Peer={}] {}", self.peer_addr, msg);
                }
            }
        }
    }
}
```

**メリット:**
- **SCTP対応容易:** `TcpStream` を `SctpStream` に置き換えるだけでSCTP化可能
- **決定論的テスト:** Core の状態遷移ロジックを時間に依存せずテスト可能
- **並行性:** Peer毎に独立したActorとして動作し、数千接続をスケール可能

---

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
