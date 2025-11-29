use std::time::Duration;
pub use cdde_shared::DiameterMessage;

// RFC 6733 Sec 5.6 Peer State Machine
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PeerState {
    Closed,
    WaitConnAck, // TCP/SCTP接続待ち
    WaitICEA,    // Initiator: CERを送ってCEA待ち
    WaitIOpen,   // (今回は省略可能だがRFC準拠のため記載)
    Open,        // 通信可能 (UP状態)
    Closing,     // 切断処理中
}

// FSMへの入力 (Input Event)
#[derive(Debug)]
pub enum FsmEvent {
    Start,                      // 起動指示
    ConnectionUp,               // TCP/SCTP接続完了
    ConnectionFailed,           // 接続失敗
    MessageReceived(DiameterMessage),
    WatchdogTimerExpiry,        // Tw (Watchdog Timer) 発火
    DisconnectRequest,          // 管理者からの切断指示
}

// FSMからの出力 (Output Action)
#[derive(Debug, PartialEq)]
pub enum FsmAction {
    ConnectToPeer,              // ソケット接続を開始せよ
    DisconnectPeer,             // ソケットを切断せよ
    SendBytes(Vec<u8>),         // データを送信せよ
    ResetWatchdogTimer,         // Watchdogタイマーをリセットせよ
    NotifyDflUp,                // DFLへ「Peer UP」を通知せよ
    NotifyDflDown,              // DFLへ「Peer DOWN」を通知せよ
    Log(String),                // ログ出力
}

// 設定
#[derive(Debug, Clone)]
pub struct PeerConfig {
    pub watchdog_interval: Duration, // Tw
    pub max_watchdog_failures: u32,  // 許容するDWRタイムアウト回数
}
