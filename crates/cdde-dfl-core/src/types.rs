use std::time::Duration;
use cdde_shared::DiameterMessage;

// セッションID (今回はHop-by-Hop ID + Connection IDをキーとする想定)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionKey {
    pub connection_id: u64,
    pub hop_by_hop_id: u32,
}

// タイムアウト時の情報（Runtime層でのログ・監視用）
#[derive(Debug, Clone, PartialEq)]
pub struct TimeoutInfo {
    pub key: SessionKey,
    pub elapsed_ms: u64,  // 経過時間（ミリ秒）
    pub error_response: DiameterMessage,  // 3002エラー応答
}

// ドメインイベント: CoreからRuntimeへの命令
#[derive(Debug, PartialEq)]
pub enum SessionAction {
    ForwardToDcr(DiameterMessage),              // DCRへ転送せよ
    ReplyWith3002Error(TimeoutInfo),            // タイムアウトエラー応答を送信せよ (3002 UNABLE_TO_DELIVER)
    Discard,                                    // 破棄せよ  
    RemoveSession(SessionKey),                  // メモリから削除せよ
}

// 設定
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub timeout_duration: Duration,
}
