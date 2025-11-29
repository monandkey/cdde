use crate::core::fsm::PeerFsm;
use crate::core::types::{PeerConfig, FsmAction, FsmEvent};
use cdde_shared::DiameterMessage;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::time::{self, Interval};

pub struct PeerActor {
    core: PeerFsm,
    peer_addr: String,
    
    // Runtime State
    socket: Option<TcpStream>,
    dfl_notifier: mpsc::Sender<String>, // DFLへの通知チャネル(簡易版)
    watchdog_timer: Interval,
}

impl PeerActor {
    pub fn new(
        peer_addr: String,
        config: PeerConfig,
        dfl_notifier: mpsc::Sender<String>,
    ) -> Self {
        // Watchdogタイマーの初期化 (Tick間隔)
        let mut timer = time::interval(config.watchdog_interval);
        timer.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        Self {
            core: PeerFsm::new(config),
            peer_addr,
            socket: None,
            dfl_notifier,
            watchdog_timer: timer,
        }
    }

    pub async fn run(&mut self) {
        // 最初にStartイベントを投入
        self.handle_event(FsmEvent::Start).await;

        let mut buf = [0u8; 4096];

        loop {
            // Rustの借用チェッカーを回避するための分岐ロジック
            // ソケットがある場合とない場合で select! の対象が変わるため
            let event = if let Some(socket) = &mut self.socket {
                tokio::select! {
                    // パターンA: ソケットからの受信
                    res = socket.read(&mut buf) => {
                        match res {
                            Ok(0) => FsmEvent::ConnectionFailed, // 切断された
                            Ok(_n) => {
                                // ※本来はここでバイナリパースを行う
                                // 簡易的にコマンドコードだけ読み取ったとする
                                let msg = DiameterMessage::new(280, true); // 仮: DWRなどが来たと想定
                                FsmEvent::MessageReceived(msg)
                            }
                            Err(_) => FsmEvent::ConnectionFailed,
                        }
                    }
                    // パターンB: Watchdogタイマー発火
                    _ = self.watchdog_timer.tick() => {
                        FsmEvent::WatchdogTimerExpiry
                    }
                }
            } else {
                // ソケットがない状態の待機 (再接続タイマーなどはここに実装)
                tokio::select! {
                     _ = self.watchdog_timer.tick() => {
                         FsmEvent::WatchdogTimerExpiry 
                     }
                }
            };

            self.handle_event(event).await;
        }
    }

    // イベントを受け取り、Coreを回し、アクションを実行する
    async fn handle_event(&mut self, event: FsmEvent) {
        let actions = self.core.step(event);

        for action in actions {
            match action {
                FsmAction::ConnectToPeer => {
                    println!("Connecting to {}...", self.peer_addr);
                    match TcpStream::connect(&self.peer_addr).await {
                        Ok(stream) => {
                            self.socket = Some(stream);
                            // 再帰的にイベントを呼ぶ (無限ループ注意だが、状態が変わるのでOK)
                            // ここではシンプルに処理を分けるため再帰呼び出しはせず、
                            // 次のループで処理されるようにするか、即時stepを呼ぶ
                            // 簡易実装として再帰呼び出しを避けるパターンで実装
                            // (本来はAction loopを回すべき)
                            // self.core.step(FsmEvent::ConnectionUp); 
                            // TODO: 再帰呼び出しを避けるため、ここではログ出力のみ
                            println!("Connected!");
                        }
                        Err(e) => {
                            println!("Connect failed: {}", e);
                            // self.core.step(FsmEvent::ConnectionFailed);
                        }
                    }
                }
                FsmAction::DisconnectPeer => {
                    self.socket = None; // Dropによる切断
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
