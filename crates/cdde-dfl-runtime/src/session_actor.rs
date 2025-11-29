use cdde_shared::DiameterMessage;
use cdde_dfl_core::types::*;
use cdde_dfl_core::session::SessionManagerCore;
use tokio::sync::mpsc;
use tokio_util::time::DelayQueue;
use futures::stream::StreamExt;
use std::time::Instant;

// Actorへの入力メッセージ
pub enum ActorMessage {
    IngressRequest { conn_id: u64, msg: DiameterMessage },
    IngressAnswer { conn_id: u64, msg: DiameterMessage },
}

pub struct SessionActor {
    core: SessionManagerCore,
    receiver: mpsc::Receiver<ActorMessage>,
    
    // タイムアウト管理用キュー: キーを入れておくと、指定時間後に取り出せる
    timeout_queue: DelayQueue<SessionKey>,
    
    // 外部出力用チャネル (SCTP送信やgRPC送信へ)
    outbound_tx: mpsc::Sender<SessionAction>,
}

impl SessionActor {
    pub fn new(
        config: SessionConfig,
        receiver: mpsc::Receiver<ActorMessage>,
        outbound_tx: mpsc::Sender<SessionAction>,
    ) -> Self {
        Self {
            core: SessionManagerCore::new(config),
            receiver,
            timeout_queue: DelayQueue::new(),
            outbound_tx,
        }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                // 1. 外部からのメッセージ受信 (Request/Answer)
                Some(msg) = self.receiver.recv() => {
                    self.handle_message(msg).await;
                }

                // 2. タイムアウト発火
                Some(expired) = self.timeout_queue.next() => {
                    let key = expired.into_inner();
                    let actions = self.core.on_timeout(key);
                    self.execute_actions(actions).await;
                }
                
                else => break, // チャネルが閉じたら終了
            }
        }
    }

    async fn handle_message(&mut self, msg: ActorMessage) {
        let actions = match msg {
            ActorMessage::IngressRequest { conn_id, msg } => {
                let key = SessionKey {
                    connection_id: conn_id,
                    hop_by_hop_id: msg.hop_by_hop_id,
                };
                
                // タイムアウトタイマーをセット (Coreの設定値を使用)
                let timeout_duration = self.core.config.timeout_duration;
                self.timeout_queue.insert(key, timeout_duration);

                self.core.on_request_received(conn_id, msg, Instant::now())
            },
            ActorMessage::IngressAnswer { conn_id, msg } => {
                // Answerが来たのでコアで処理（成功すればエントリが消える）
                self.core.on_answer_received(conn_id, msg)
            }
        };

        self.execute_actions(actions).await;
    }

    async fn execute_actions(&self, actions: Vec<SessionAction>) {
        for action in actions {
            // 実際はチャネルに送信する
            if let Err(e) = self.outbound_tx.send(action).await {
                eprintln!("Failed to send action: {:?}", e);
            }
        }
    }
}
