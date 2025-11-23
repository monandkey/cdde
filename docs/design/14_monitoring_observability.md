## 監視・可観測性設計

---

## 1. 可観測性の3本柱

CDDEシステムでは、以下の3つの要素で可観測性を実現します。

| 要素 | 目的 | ツール | 保持期間 |
|:---|:---|:---|---:|
| **メトリクス** | システムの健全性・パフォーマンス監視 | Prometheus + Grafana | 30日 |
| **ログ** | イベント・エラーの詳細追跡 | Loki + Grafana | 7日 |
| **トレース** | リクエストフローの可視化 | Jaeger | 3日 |

---

## 2. メトリクス設計 (Prometheus)

### 2.1. DFL メトリクス

```rust
// dfl/src/metrics.rs

use prometheus::{
    Counter, Histogram, IntGauge, Registry,
    HistogramOpts, Opts,
};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    // リクエスト総数
    pub static ref REQUESTS_TOTAL: Counter = Counter::with_opts(
        Opts::new("cdde_dfl_requests_total", "Total number of Diameter requests")
            .namespace("cdde")
            .subsystem("dfl")
    ).unwrap();

    // レスポンス総数 (Result-Code別)
    pub static ref RESPONSES_TOTAL: prometheus::IntCounterVec = 
        prometheus::IntCounterVec::new(
            Opts::new("cdde_dfl_responses_total", "Total responses by result code"),
            &["result_code"]
        ).unwrap();

    // レイテンシ分布
    pub static ref LATENCY_SECONDS: Histogram = Histogram::with_opts(
        HistogramOpts::new("cdde_dfl_latency_seconds", "Request latency in seconds")
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0])
    ).unwrap();

    // アクティブセッション数
    pub static ref ACTIVE_SESSIONS: IntGauge = IntGauge::with_opts(
        Opts::new("cdde_dfl_active_sessions", "Number of active sessions")
    ).unwrap();

    // タイムアウト数
    pub static ref TIMEOUTS_TOTAL: Counter = Counter::with_opts(
        Opts::new("cdde_dfl_timeouts_total", "Total number of session timeouts")
    ).unwrap();

    // SCTP接続数
    pub static ref SCTP_CONNECTIONS: IntGauge = IntGauge::with_opts(
        Opts::new("cdde_dfl_sctp_connections", "Number of active SCTP connections")
    ).unwrap();
}

pub fn register_metrics() {
    REGISTRY.register(Box::new(REQUESTS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(RESPONSES_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(LATENCY_SECONDS.clone())).unwrap();
    REGISTRY.register(Box::new(ACTIVE_SESSIONS.clone())).unwrap();
    REGISTRY.register(Box::new(TIMEOUTS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(SCTP_CONNECTIONS.clone())).unwrap();
}
```

### 2.2. メトリクスの記録

```rust
// dfl/src/handler.rs

use crate::metrics::*;
use std::time::Instant;

pub async fn handle_diameter_request(packet: DiameterPacket) -> Result<(), Error> {
    let start = Instant::now();
    
    // リクエストカウント
    REQUESTS_TOTAL.inc();
    ACTIVE_SESSIONS.inc();

    // 処理実行
    let result = process_packet(packet).await;

    // レイテンシ記録
    let duration = start.elapsed().as_secs_f64();
    LATENCY_SECONDS.observe(duration);

    // レスポンスカウント
    match result {
        Ok(response) => {
            let result_code = response.get_result_code();
            RESPONSES_TOTAL.with_label_values(&[&result_code.to_string()]).inc();
        }
        Err(_) => {
            RESPONSES_TOTAL.with_label_values(&["error"]).inc();
        }
    }

    ACTIVE_SESSIONS.dec();
    result
}
```

### 2.3. Prometheusエンドポイント

```rust
// dfl/src/main.rs

use axum::{routing::get, Router};
use prometheus::{Encoder, TextEncoder};

async fn metrics_handler() -> String {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

#[tokio::main]
async fn main() {
    // メトリクス登録
    register_metrics();

    // メトリクスエンドポイント
    let app = Router::new()
        .route("/metrics", get(metrics_handler));

    axum::Server::bind(&"0.0.0.0:9090".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

---

## 3. Prometheus 設定

### 3.1. Prometheus ConfigMap

```yaml
# prometheus-config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: prometheus-config
  namespace: cdde-system
data:
  prometheus.yml: |
    global:
      scrape_interval: 15s
      evaluation_interval: 15s

    scrape_configs:
      # DFL
      - job_name: 'dfl'
        kubernetes_sd_configs:
        - role: pod
          namespaces:
            names:
            - cdde-system
        relabel_configs:
        - source_labels: [__meta_kubernetes_pod_label_app]
          action: keep
          regex: dfl
        - source_labels: [__meta_kubernetes_pod_ip]
          target_label: __address__
          replacement: $1:9090

      # DCR
      - job_name: 'dcr'
        kubernetes_sd_configs:
        - role: pod
          namespaces:
            names:
            - cdde-system
        relabel_configs:
        - source_labels: [__meta_kubernetes_pod_label_app]
          action: keep
          regex: dcr
        - source_labels: [__meta_kubernetes_pod_label_vr_id]
          target_label: vr_id

      # DPA
      - job_name: 'dpa'
        kubernetes_sd_configs:
        - role: pod
          namespaces:
            names:
            - cdde-system
        relabel_configs:
        - source_labels: [__meta_kubernetes_pod_label_app]
          action: keep
          regex: dpa
```

---

## 4. Grafana ダッシュボード

### 4.1. DFL ダッシュボード JSON

```json
{
  "dashboard": {
    "title": "CDDE - DFL Monitoring",
    "panels": [
      {
        "title": "Request Rate (TPS)",
        "targets": [
          {
            "expr": "rate(cdde_dfl_requests_total[1m])"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Latency (P50, P95, P99)",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, rate(cdde_dfl_latency_seconds_bucket[5m]))",
            "legendFormat": "P50"
          },
          {
            "expr": "histogram_quantile(0.95, rate(cdde_dfl_latency_seconds_bucket[5m]))",
            "legendFormat": "P95"
          },
          {
            "expr": "histogram_quantile(0.99, rate(cdde_dfl_latency_seconds_bucket[5m]))",
            "legendFormat": "P99"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Active Sessions",
        "targets": [
          {
            "expr": "cdde_dfl_active_sessions"
          }
        ],
        "type": "stat"
      },
      {
        "title": "Response Distribution by Result-Code",
        "targets": [
          {
            "expr": "sum by (result_code) (rate(cdde_dfl_responses_total[5m]))"
          }
        ],
        "type": "piechart"
      }
    ]
  }
}
```

### 4.2. アラートルール

```yaml
# prometheus-alerts.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: prometheus-alerts
  namespace: cdde-system
data:
  alerts.yml: |
    groups:
    - name: cdde_alerts
      interval: 30s
      rules:
      # 高レイテンシアラート
      - alert: HighLatency
        expr: histogram_quantile(0.95, rate(cdde_dfl_latency_seconds_bucket[5m])) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "DFL P95 latency is above 100ms"
          description: "P95 latency: {{ $value }}s"

      # エラー率アラート
      - alert: HighErrorRate
        expr: |
          sum(rate(cdde_dfl_responses_total{result_code!="2001"}[5m])) 
          / 
          sum(rate(cdde_dfl_responses_total[5m])) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Error rate is above 5%"

      # セッションタイムアウト急増
      - alert: HighTimeoutRate
        expr: rate(cdde_dfl_timeouts_total[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Session timeout rate is high"

      # Pod Down
      - alert: PodDown
        expr: up{job="dfl"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "DFL pod is down"
```

---

## 5. ログ設計 (Structured Logging)

### 5.1. tracing を使用した構造化ログ

```rust
// dfl/src/main.rs

use tracing::{info, warn, error, debug, instrument};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() {
    // ログ初期化
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()  // JSON形式で出力
        .init();

    info!(
        service = "dfl",
        version = env!("CARGO_PKG_VERSION"),
        "Starting DFL service"
    );

    // アプリケーション起動
    run_service().await;
}

#[instrument(skip(packet), fields(
    hop_by_hop_id = %packet.header.hop_by_hop_id,
    command_code = %packet.header.command_code
))]
async fn handle_request(packet: DiameterPacket) -> Result<(), Error> {
    debug!("Processing Diameter request");

    match process(packet).await {
        Ok(response) => {
            info!(
                result_code = response.result_code,
                latency_ms = response.latency.as_millis(),
                "Request processed successfully"
            );
            Ok(())
        }
        Err(e) => {
            error!(
                error = %e,
                "Failed to process request"
            );
            Err(e)
        }
    }
}
```

### 5.2. Loki 設定

```yaml
# loki-config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: loki-config
  namespace: cdde-system
data:
  loki.yaml: |
    auth_enabled: false

    server:
      http_listen_port: 3100

    ingester:
      lifecycler:
        ring:
          kvstore:
            store: inmemory
          replication_factor: 1

    schema_config:
      configs:
      - from: 2023-01-01
        store: boltdb-shipper
        object_store: filesystem
        schema: v11
        index:
          prefix: index_
          period: 24h

    storage_config:
      boltdb_shipper:
        active_index_directory: /loki/index
        cache_location: /loki/cache
      filesystem:
        directory: /loki/chunks

    limits_config:
      retention_period: 168h  # 7日間
```

### 5.3. Promtail (ログ収集)

```yaml
# promtail-daemonset.yaml
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: promtail
  namespace: cdde-system
spec:
  selector:
    matchLabels:
      app: promtail
  template:
    metadata:
      labels:
        app: promtail
    spec:
      containers:
      - name: promtail
        image: grafana/promtail:latest
        args:
        - -config.file=/etc/promtail/promtail.yaml
        volumeMounts:
        - name: config
          mountPath: /etc/promtail
        - name: varlog
          mountPath: /var/log
        - name: varlibdockercontainers
          mountPath: /var/lib/docker/containers
          readOnly: true
      volumes:
      - name: config
        configMap:
          name: promtail-config
      - name: varlog
        hostPath:
          path: /var/log
      - name: varlibdockercontainers
        hostPath:
          path: /var/lib/docker/containers
```

---

## 6. 分散トレーシング (Jaeger)

### 6.1. OpenTelemetry 統合

```rust
// Cargo.toml
[dependencies]
opentelemetry = "0.20"
opentelemetry-jaeger = "0.19"
tracing-opentelemetry = "0.21"

// dfl/src/tracing.rs
use opentelemetry::global;
use opentelemetry_jaeger::new_agent_pipeline;
use tracing_subscriber::layer::SubscriberExt;

pub fn init_tracing() {
    let tracer = new_agent_pipeline()
        .with_service_name("dfl")
        .install_simple()
        .unwrap();

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    
    let subscriber = tracing_subscriber::registry()
        .with(telemetry)
        .with(tracing_subscriber::fmt::layer());

    tracing::subscriber::set_global_default(subscriber).unwrap();
}

#[instrument]
async fn handle_request(packet: DiameterPacket) -> Result<(), Error> {
    // トレースコンテキストが自動的に伝播
    let response = forward_to_dcr(packet).await?;
    Ok(())
}
```

---

## 7. 統合監視ダッシュボード

### 7.1. Grafana統合ダッシュボード

```json
{
  "dashboard": {
    "title": "CDDE - System Overview",
    "rows": [
      {
        "title": "Traffic Overview",
        "panels": [
          {"title": "Total TPS", "datasource": "Prometheus"},
          {"title": "Error Rate", "datasource": "Prometheus"},
          {"title": "P99 Latency", "datasource": "Prometheus"}
        ]
      },
      {
        "title": "Recent Errors",
        "panels": [
          {
            "title": "Error Logs",
            "datasource": "Loki",
            "targets": [
              {
                "expr": "{app=\"dfl\"} |= \"ERROR\""
              }
            ]
          }
        ]
      },
      {
        "title": "Distributed Traces",
        "panels": [
          {
            "title": "Trace Timeline",
            "datasource": "Jaeger"
          }
        ]
      }
    ]
  }
}
```

---

## 8. アラート通知

### 8.1. Alertmanager 設定

```yaml
# alertmanager-config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: alertmanager-config
  namespace: cdde-system
data:
  alertmanager.yml: |
    global:
      resolve_timeout: 5m

    route:
      group_by: ['alertname', 'severity']
      group_wait: 10s
      group_interval: 10s
      repeat_interval: 12h
      receiver: 'slack-notifications'

    receivers:
    - name: 'slack-notifications'
      slack_configs:
      - api_url: 'https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK'
        channel: '#cdde-alerts'
        title: 'CDDE Alert: {{ .GroupLabels.alertname }}'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'
```
