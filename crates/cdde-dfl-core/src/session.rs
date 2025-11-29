use super::types::*;
use cdde_shared::DiameterMessage;
use std::collections::HashMap;
use std::time::Instant;

// セッションの状態
#[derive(Debug)]
struct SessionState {
    created_at: Instant,
    original_msg: DiameterMessage,
}

// ★ Sans-IO Core Logic
pub struct SessionManagerCore {
    // 進行中のセッション (Request受信済み、Answer未受信)
    sessions: HashMap<SessionKey, SessionState>,
    pub config: SessionConfig,
}

impl SessionManagerCore {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            sessions: HashMap::new(),
            config,
        }
    }

    // イベント1: クライアント(Peer)からリクエストを受信
    pub fn on_request_received(
        &mut self,
        conn_id: u64,
        msg: DiameterMessage,
        now: Instant
    ) -> Vec<SessionAction> {
        let key = SessionKey {
            connection_id: conn_id,
            hop_by_hop_id: msg.hop_by_hop_id,
        };

        // セッションストアに保存
        self.sessions.insert(key, SessionState {
            created_at: now,
            original_msg: msg.clone(),
        });

        // DCRへの転送を指示
        vec![SessionAction::ForwardToDcr(msg)]
    }

    // イベント2: DCRまたはPeerからアンサーを受信
    pub fn on_answer_received(
        &mut self,
        conn_id: u64,
        msg: DiameterMessage
    ) -> Vec<SessionAction> {
        let key = SessionKey {
            connection_id: conn_id,
            hop_by_hop_id: msg.hop_by_hop_id,
        };

        if self.sessions.remove(&key).is_some() {
            // セッションが存在する = タイムアウトしていない正常なフロー
            vec![SessionAction::RemoveSession(key)] 
        } else {
            // セッションがない = タイムアウト済み or 不正なパケット
            vec![SessionAction::Discard]
        }
    }

    // イベント3: タイムアウト検知 (RuntimeからTickまたは個別通知で呼ばれる)
    pub fn on_timeout(&mut self, key: SessionKey) -> Vec<SessionAction> {
        if self.sessions.remove(&key).is_some() {
            // まだ残っていたなら3002(UNABLE_TO_DELIVER)を返す
            vec![SessionAction::ReplyWith3002(key)]
        } else {
            // 既に処理済みなら何もしない
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_request_received() {
        let config = SessionConfig {
            timeout_duration: Duration::from_secs(5),
        };
        let mut core = SessionManagerCore::new(config);
        let msg = DiameterMessage::new(272, true); // Credit-Control-Request
        let now = Instant::now();

        let actions = core.on_request_received(1, msg.clone(), now);

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            SessionAction::ForwardToDcr(m) => assert_eq!(m.command_code, 272),
            _ => panic!("Unexpected action"),
        }
    }
}
