use cdde_shared::{DiameterMessage, AVP_DEST_REALM};
use super::manipulation::ManipulationEngine;

// ルーティング結果
#[derive(Debug, PartialEq)]
pub enum RouteAction {
    Forward(String), // 転送先Peer名 (またはPool名)
    Discard,         // 破棄
    ReplyError(u32), // エラーコードを返却 (3002など)
}

// ルーティングテーブルのエントリ
pub struct RouteEntry {
    pub dest_realm: String,
    pub target_peer: String,
}

// ★ Sans-IO Core Logic
pub struct RouterCore {
    routes: Vec<RouteEntry>,
    manipulator: ManipulationEngine,
}

impl RouterCore {
    pub fn new(routes: Vec<RouteEntry>, manipulator: ManipulationEngine) -> Self {
        Self { routes, manipulator }
    }

    // メイン処理: 入力メッセージ -> (加工後メッセージ, アクション)
    pub fn process(&self, msg: DiameterMessage) -> (DiameterMessage, RouteAction) {
        // 1. Manipulation & Topology Hiding 実行
        let processed_msg = self.manipulator.apply(msg);

        // 2. ルーティング決定
        // (Dest-Realmを見て決定する単純な例)
        let dest_realm = processed_msg.get_avp(AVP_DEST_REALM)
            .map(|a| a.as_string())
            .unwrap_or_default();

        let action = if let Some(route) = self.routes.iter().find(|r| r.dest_realm == dest_realm) {
            RouteAction::Forward(route.target_peer.clone())
        } else {
            // ルートが見つからない場合
            RouteAction::ReplyError(3001) // DIAMETER_UNABLE_TO_DELIVER
        };

        (processed_msg, action)
    }
}
