use super::types::*;
use cdde_shared::{DiameterMessage, CMD_CER, CMD_DWR, CMD_ACR};

pub struct PeerFsm {
    state: PeerState,
    config: PeerConfig,
    watchdog_failures: u32, // 連続失敗回数カウンタ
}

impl PeerFsm {
    pub fn new(config: PeerConfig) -> Self {
        Self {
            state: PeerState::Closed,
            config,
            watchdog_failures: 0,
        }
    }

    pub fn current_state(&self) -> PeerState {
        self.state
    }

    // ★ Core Logic: 状態遷移関数
    pub fn step(&mut self, event: FsmEvent) -> Vec<FsmAction> {
        let mut actions = Vec::new();

        match (self.state, event) {
            // --- 1. 起動シーケンス ---
            (PeerState::Closed, FsmEvent::Start) => {
                self.state = PeerState::WaitConnAck;
                actions.push(FsmAction::Log("Starting connection sequence...".into()));
                actions.push(FsmAction::ConnectToPeer);
            }

            // --- 2. 接続確立 -> CER送信 ---
            (PeerState::WaitConnAck, FsmEvent::ConnectionUp) => {
                self.state = PeerState::WaitICEA;
                // CER (Capabilities-Exchange-Request) を作成して送信
                // ※本来はAVP構築ロジックが入るが、ここではバイト列のみ模擬
                let cer_bytes = vec![0x01, 0x00, 0x00, 0x00]; 
                actions.push(FsmAction::SendBytes(cer_bytes));
            }
            
            (PeerState::WaitConnAck, FsmEvent::ConnectionFailed) => {
                // 再接続ロジック（バックオフ）が必要だが、一旦Closedに戻す
                self.state = PeerState::Closed;
                actions.push(FsmAction::Log("Connection failed. Backing off.".into()));
            }

            // --- 3. CEA受信 -> Open (UP) ---
            (PeerState::WaitICEA, FsmEvent::MessageReceived(msg)) if msg.is_cea() => {
                self.state = PeerState::Open;
                self.watchdog_failures = 0;
                
                actions.push(FsmAction::Log("CEA received. State is OPEN.".into()));
                actions.push(FsmAction::NotifyDflUp); // DFLに通知
                actions.push(FsmAction::ResetWatchdogTimer);
            }

            // --- 4. Open状態 (定常監視) ---
            (PeerState::Open, FsmEvent::WatchdogTimerExpiry) => {
                if self.watchdog_failures >= self.config.max_watchdog_failures {
                    // タイムアウト上限超過 -> DOWN判定
                    self.state = PeerState::Closed;
                    actions.push(FsmAction::Log("Watchdog failed too many times. Closing.".into()));
                    actions.push(FsmAction::NotifyDflDown);
                    actions.push(FsmAction::DisconnectPeer);
                } else {
                    // DWR (Device-Watchdog-Request) 送信
                    self.watchdog_failures += 1;
                    let dwr_bytes = vec![0x02, 0x00, 0x00, 0x00]; 
                    actions.push(FsmAction::SendBytes(dwr_bytes));
                    actions.push(FsmAction::ResetWatchdogTimer); // 次のタイマーセット
                }
            }

            (PeerState::Open, FsmEvent::MessageReceived(msg)) => {
                // 何らかのメッセージを受信したら生存とみなす
                self.watchdog_failures = 0;
                actions.push(FsmAction::ResetWatchdogTimer);

                if msg.is_dwr() {
                    // DWR受信 -> DWA応答
                    let dwa_bytes = vec![0x02, 0x00, 0x00, 0x01];
                    actions.push(FsmAction::SendBytes(dwa_bytes));
                } else if msg.is_dwa() {
                    // DWA受信 -> 生存確認完了
                    actions.push(FsmAction::Log("DWA received. Peer is healthy.".into()));
                } 
                // 通常のRequest/Answerはここでは特にハンドリングせずRouterへ流す設計も可
            }

            // --- 5. 異常系 / その他 ---
            (_, FsmEvent::DisconnectRequest) => {
                self.state = PeerState::Closed;
                actions.push(FsmAction::DisconnectPeer);
                actions.push(FsmAction::NotifyDflDown);
            }
            
            _ => {
                // 無効な遷移
                actions.push(FsmAction::Log(format!("Invalid event for state {:?}", self.state)));
            }
        }

        actions
    }
}
