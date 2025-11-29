use cdde_dcr_core::router::{RouterCore, RouteAction};
use cdde_shared::DiameterMessage;
use arc_swap::ArcSwap;
use std::sync::Arc;
use tonic::{Request, Response, Status};

// gRPCの定義は cdde-proto にあると仮定 (今回はモック)
// 本来は: use cdde_proto::dcr_server::{Dcr, DcrServer};
// ここでは簡易的に構造体のみ定義

pub struct DcrService {
    // ★ Lock-Free Configuration Update
    // RouterCore全体をArcSwapで包み、リクエスト処理中にロックを取らずに参照可能にする
    core: Arc<ArcSwap<RouterCore>>,
}

impl DcrService {
    pub fn new(initial_core: RouterCore) -> Self {
        Self {
            core: Arc::new(ArcSwap::from_pointee(initial_core)),
        }
    }

    // 設定更新API (管理プレーンから呼ばれる)
    pub fn update_config(&self, new_core: RouterCore) {
        self.core.store(Arc::new(new_core));
    }

    // トラフィック処理 (gRPCハンドラ)
    pub async fn process_message(&self, msg: DiameterMessage) -> Result<DiameterMessage, Status> {
        // 1. 現在の設定(Core)のスナップショットを取得 (Lock-Free)
        let core = self.core.load();

        // 2. Coreロジック実行 (純粋関数)
        let (processed_msg, action) = core.process(msg);

        // 3. アクションに応じた副作用実行 (I/O)
        match action {
            RouteAction::Forward(peer) => {
                println!("Forwarding to {}", peer);
                // ここで実際の転送処理 (gRPC client or channel)
                Ok(processed_msg)
            }
            RouteAction::Discard => {
                println!("Discarding message");
                Err(Status::aborted("Message discarded"))
            }
            RouteAction::ReplyError(code) => {
                println!("Replying with error {}", code);
                Err(Status::unknown(format!("Diameter Error {}", code)))
            }
        }
    }
}
