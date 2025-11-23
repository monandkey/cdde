## エラーハンドリング戦略

---

## 1. エラー分類

CDDEシステムでは、エラーを以下のカテゴリに分類します。

| カテゴリ | 説明 | 例 | 対応方針 |
|:---|:---|:---|:---|
| **プロトコルエラー** | Diameter仕様違反 | 不正なAVP、ヘッダ破損 | Result-Codeで応答 |
| **ルーティングエラー** | ルート解決失敗 | 宛先不明、全Peerダウン | 3002/3003で応答 |
| **タイムアウトエラー** | 応答遅延 | セッションタイムアウト | 3002で応答 |
| **システムエラー** | 内部障害 | メモリ不足、DB接続失敗 | ログ記録、アラート |
| **ネットワークエラー** | 通信障害 | SCTP切断、DNS失敗 | リトライ、フェイルオーバー |

---

## 2. Rust エラー型設計

### 2.1. 統一エラー型

```rust
// common/src/error.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CddeError {
    // プロトコルエラー
    #[error("Invalid Diameter packet: {0}")]
    InvalidPacket(String),
    
    #[error("Missing required AVP: {0}")]
    MissingAvp(u32),
    
    #[error("Invalid AVP value for code {code}: {reason}")]
    InvalidAvpValue { code: u32, reason: String },

    // ルーティングエラー
    #[error("No route found for realm: {0}")]
    NoRoute(String),
    
    #[error("All peers are down for pool: {0}")]
    AllPeersDown(String),
    
    #[error("Routing loop detected")]
    RoutingLoop,

    // タイムアウトエラー
    #[error("Session timeout after {0}ms")]
    SessionTimeout(u64),
    
    #[error("gRPC call timeout")]
    GrpcTimeout,

    // システムエラー
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),

    // ネットワークエラー
    #[error("SCTP connection failed: {0}")]
    SctpError(#[from] std::io::Error),
    
    #[error("gRPC error: {0}")]
    GrpcError(#[from] tonic::Status),
}

impl CddeError {
    /// エラーをDiameter Result-Codeに変換
    pub fn to_result_code(&self) -> u32 {
        match self {
            Self::InvalidPacket(_) => 3008,  // DIAMETER_INVALID_AVP_VALUE
            Self::MissingAvp(_) => 5005,     // DIAMETER_MISSING_AVP
            Self::InvalidAvpValue { .. } => 3008,
            Self::NoRoute(_) => 3003,        // DIAMETER_REALM_NOT_SERVED
            Self::AllPeersDown(_) => 3002,   // DIAMETER_UNABLE_TO_DELIVER
            Self::RoutingLoop => 3005,       // DIAMETER_LOOP_DETECTED
            Self::SessionTimeout(_) => 3002,
            Self::GrpcTimeout => 3002,
            _ => 3010,                       // DIAMETER_UNABLE_TO_COMPLY
        }
    }

    /// エラーの重要度
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::InvalidPacket(_) | Self::MissingAvp(_) => ErrorSeverity::Warning,
            Self::NoRoute(_) | Self::AllPeersDown(_) => ErrorSeverity::Error,
            Self::RoutingLoop => ErrorSeverity::Critical,
            Self::DatabaseError(_) | Self::InternalError(_) => ErrorSeverity::Critical,
            _ => ErrorSeverity::Warning,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}
```

### 2.2. Result型エイリアス

```rust
// common/src/lib.rs

pub type Result<T> = std::result::Result<T, CddeError>;
```

---

## 3. エラーハンドリングパターン

### 3.1. DFL - パケット処理エラー

```rust
// dfl/src/handler.rs

use tracing::{error, warn, info};

pub async fn handle_diameter_packet(
    packet: Vec<u8>,
    connection_id: u64,
) -> Result<()> {
    // 1. パケットパース
    let diameter_packet = match DiameterPacket::parse(&packet) {
        Ok(p) => p,
        Err(e) => {
            warn!(
                connection_id = connection_id,
                error = %e,
                "Failed to parse Diameter packet"
            );
            // パースエラー時はエラー応答を生成
            let error_response = generate_error_response(
                &packet,
                e.to_result_code(),
            )?;
            send_response(connection_id, error_response).await?;
            return Err(e);
        }
    };

    // 2. セッション登録
    if let Err(e) = register_session(&diameter_packet).await {
        error!(
            hop_by_hop_id = diameter_packet.header.hop_by_hop_id,
            error = %e,
            "Failed to register session"
        );
        // システムエラー時は3010で応答
        let error_response = create_error_answer(
            &diameter_packet,
            3010,  // DIAMETER_UNABLE_TO_COMPLY
        );
        send_response(connection_id, error_response).await?;
        return Err(e);
    }

    // 3. DCRへ転送
    match forward_to_dcr(&diameter_packet).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!(
                hop_by_hop_id = diameter_packet.header.hop_by_hop_id,
                error = %e,
                "Failed to forward to DCR"
            );
            // 転送エラー時はセッションをクリーンアップ
            cleanup_session(&diameter_packet).await;
            Err(e)
        }
    }
}
```

### 3.2. DCR - ルーティングエラー

```rust
// dcr/src/routing.rs

pub async fn route_packet(packet: &DiameterPacket) -> Result<RoutingDecision> {
    // ループ検出
    if detect_routing_loop(packet) {
        warn!(
            hop_by_hop_id = packet.header.hop_by_hop_id,
            "Routing loop detected"
        );
        return Err(CddeError::RoutingLoop);
    }

    // ルート検索
    let route = match find_route(packet) {
        Some(r) => r,
        None => {
            let realm = packet.get_avp(AVP_DESTINATION_REALM)
                .unwrap_or("unknown");
            warn!(
                realm = realm,
                "No route found"
            );
            return Err(CddeError::NoRoute(realm.to_string()));
        }
    };

    // Peer選択
    let peer = match select_peer(&route.pool_id).await {
        Ok(p) => p,
        Err(e) => {
            error!(
                pool_id = route.pool_id,
                error = %e,
                "All peers are down"
            );
            return Err(CddeError::AllPeersDown(route.pool_id.clone()));
        }
    };

    Ok(RoutingDecision {
        target_peer: peer,
        route,
    })
}
```

### 3.3. CMS - データベースエラー

```rust
// cms/src/repository.rs

use sqlx::PgPool;

pub async fn create_virtual_router(
    pool: &PgPool,
    vr: &VirtualRouter,
) -> Result<String> {
    sqlx::query!(
        r#"
        INSERT INTO virtual_routers (id, hostname, realm, timeout_ms)
        VALUES ($1, $2, $3, $4)
        "#,
        vr.id,
        vr.hostname,
        vr.realm,
        vr.timeout_ms
    )
    .execute(pool)
    .await
    .map_err(|e| {
        error!(
            vr_id = vr.id,
            error = %e,
            "Failed to create virtual router"
        );
        CddeError::DatabaseError(e)
    })?;

    info!(vr_id = vr.id, "Virtual router created successfully");
    Ok(vr.id.clone())
}
```

---

## 4. リトライ戦略

### 4.1. 指数バックオフ

```rust
// common/src/retry.rs

use tokio::time::{sleep, Duration};

pub struct RetryPolicy {
    max_retries: u32,
    initial_delay_ms: u64,
    max_delay_ms: u64,
    backoff_multiplier: f64,
}

impl RetryPolicy {
    pub fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
        }
    }

    pub async fn execute<F, T, E>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> std::result::Result<T, E>,
        E: Into<CddeError>,
    {
        let mut attempt = 0;
        let mut delay = self.initial_delay_ms;

        loop {
            match operation() {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempt += 1;
                    let error: CddeError = e.into();

                    if attempt >= self.max_retries || !error.is_retryable() {
                        return Err(error);
                    }

                    warn!(
                        attempt = attempt,
                        max_retries = self.max_retries,
                        delay_ms = delay,
                        error = %error,
                        "Operation failed, retrying..."
                    );

                    sleep(Duration::from_millis(delay)).await;
                    delay = (delay as f64 * self.backoff_multiplier) as u64;
                    delay = delay.min(self.max_delay_ms);
                }
            }
        }
    }
}

impl CddeError {
    fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::GrpcTimeout
                | Self::SctpError(_)
                | Self::DatabaseError(_)
        )
    }
}
```

### 4.2. 使用例

```rust
let retry_policy = RetryPolicy::default();

let result = retry_policy.execute(|| {
    // gRPC呼び出し
    grpc_client.call_dcr(request.clone())
}).await?;
```

---

## 5. サーキットブレーカー

### 5.1. 実装

```rust
// common/src/circuit_breaker.rs

use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,   // 正常
    Open,     // 障害検知、リクエスト遮断
    HalfOpen, // 回復試行中
}

pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
}

impl CircuitBreaker {
    pub fn new(
        failure_threshold: u32,
        success_threshold: u32,
        timeout: Duration,
    ) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_threshold,
            success_threshold,
            timeout,
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn call<F, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        // 状態確認
        let state = *self.state.read().await;

        match state {
            CircuitState::Open => {
                // タイムアウト経過確認
                let last_failure = self.last_failure_time.read().await;
                if let Some(time) = *last_failure {
                    if time.elapsed() > self.timeout {
                        // HalfOpenへ遷移
                        *self.state.write().await = CircuitState::HalfOpen;
                        *self.success_count.write().await = 0;
                    } else {
                        return Err(CddeError::InternalError(
                            "Circuit breaker is open".to_string()
                        ));
                    }
                }
            }
            _ => {}
        }

        // 操作実行
        match operation() {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(e)
            }
        }
    }

    async fn on_success(&self) {
        let state = *self.state.read().await;

        match state {
            CircuitState::HalfOpen => {
                let mut success_count = self.success_count.write().await;
                *success_count += 1;

                if *success_count >= self.success_threshold {
                    // Closedへ遷移
                    *self.state.write().await = CircuitState::Closed;
                    *self.failure_count.write().await = 0;
                    info!("Circuit breaker closed");
                }
            }
            CircuitState::Closed => {
                *self.failure_count.write().await = 0;
            }
            _ => {}
        }
    }

    async fn on_failure(&self) {
        let mut failure_count = self.failure_count.write().await;
        *failure_count += 1;

        if *failure_count >= self.failure_threshold {
            // Openへ遷移
            *self.state.write().await = CircuitState::Open;
            *self.last_failure_time.write().await = Some(Instant::now());
            error!(
                failure_count = *failure_count,
                "Circuit breaker opened"
            );
        }
    }
}
```

---

## 6. グレースフルシャットダウン

### 6.1. 実装

```rust
// dfl/src/main.rs

use tokio::signal;
use tokio::sync::broadcast;

#[tokio::main]
async fn main() -> Result<()> {
    // シャットダウンチャネル
    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    // SIGTERMハンドラ
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        info!("Received shutdown signal");
        let _ = shutdown_tx_clone.send(());
    });

    // サービス起動
    let service = DiameterService::new();
    service.run(shutdown_tx.subscribe()).await?;

    Ok(())
}

impl DiameterService {
    pub async fn run(&self, mut shutdown: broadcast::Receiver<()>) -> Result<()> {
        loop {
            tokio::select! {
                // 通常処理
                result = self.accept_connection() => {
                    if let Err(e) = result {
                        error!(error = %e, "Failed to accept connection");
                    }
                }

                // シャットダウンシグナル
                _ = shutdown.recv() => {
                    info!("Starting graceful shutdown");
                    self.graceful_shutdown().await?;
                    break;
                }
            }
        }

        Ok(())
    }

    async fn graceful_shutdown(&self) -> Result<()> {
        info!("Waiting for active sessions to complete");

        // 1. 新規接続を停止
        self.stop_accepting_connections();

        // 2. アクティブセッションの完了を待機 (最大30秒)
        let timeout = Duration::from_secs(30);
        let start = Instant::now();

        while self.active_sessions() > 0 && start.elapsed() < timeout {
            sleep(Duration::from_millis(100)).await;
        }

        let remaining = self.active_sessions();
        if remaining > 0 {
            warn!(
                remaining_sessions = remaining,
                "Forcing shutdown with active sessions"
            );
        }

        info!("Graceful shutdown complete");
        Ok(())
    }
}
```

---

## 7. エラーメトリクス

### 7.1. Prometheusメトリクス

```rust
use prometheus::IntCounterVec;

lazy_static! {
    pub static ref ERROR_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("cdde_errors_total", "Total errors by type and severity"),
        &["error_type", "severity"]
    ).unwrap();
}

// エラー記録
pub fn record_error(error: &CddeError) {
    let error_type = error.error_type();
    let severity = error.severity().to_string();
    
    ERROR_TOTAL
        .with_label_values(&[error_type, &severity])
        .inc();
}
```

---

## 8. エラー通知

### 8.1. アラート設定

```yaml
# prometheus-alerts.yaml
groups:
- name: error_alerts
  rules:
  - alert: HighErrorRate
    expr: |
      rate(cdde_errors_total{severity="critical"}[5m]) > 10
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "High critical error rate detected"
      description: "{{ $value }} critical errors per second"
```
