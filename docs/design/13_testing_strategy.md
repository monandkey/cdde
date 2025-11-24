## テスト戦略

---

## 1. テスト方針

CDDEシステムのテストは、以下の3層構造で実施します。

| テストレベル | 目的 | 実施タイミング | 担当 |
|:---|:---|:---|:---|
| **単体テスト** | 個別関数・モジュールの正確性検証 | コミット毎 | 開発者 |
| **統合テスト** | コンポーネント間連携の検証 | PR作成時 | 開発者 + CI |
| **システムテスト** | エンドツーエンドシナリオ検証 | リリース前 | QAチーム |
| **負荷テスト** | 性能要件の達成確認 | リリース前 | QAチーム |

---

## 2. 単体テスト (Unit Tests)

### 2.1. Rustテストフレームワーク

```rust
// diameter-dict/src/standard.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avp_code_lookup() {
        let code = StandardAvpCode::OriginHost;
        assert_eq!(code as u32, 264);
        assert_eq!(code.name(), "Origin-Host");
        assert_eq!(code.data_type(), AvpDataType::DiameterIdentity);
    }

    #[test]
    fn test_avp_parsing_unsigned32() {
        let data = vec![0x00, 0x00, 0x07, 0xD1]; // 2001
        let result = AvpDataType::Unsigned32.parse(&data).unwrap();
        
        match result {
            AvpValue::Unsigned32(val) => assert_eq!(val, 2001),
            _ => panic!("Expected Unsigned32"),
        }
    }

    #[test]
    fn test_invalid_utf8_string() {
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        let result = AvpDataType::Utf8String.parse(&invalid_utf8);
        
        assert!(result.is_err());
    }
}
```

### 2.2. モックとスタブ

```rust
// dfl/src/session_mgmt.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_session_timeout() {
        let store = Arc::new(TransactionStore::new());
        let mut delay_queue = DelayQueue::new();

        // タイムアウト100msでトランザクション登録
        let key = (1, 12345);
        let context = TransactionContext {
            delay_queue_key: delay_queue.insert(key, Duration::from_millis(100)),
            source_connection_id: 1,
            original_command_code: 316,
            original_end_to_end_id: 999,
            session_id: "test-session".to_string(),
            ingress_timestamp: Instant::now(),
        };
        store.insert(key, context);

        // タイムアウト待機
        sleep(Duration::from_millis(150)).await;

        // DelayQueueからイベント取得
        let expired = delay_queue.next().await.unwrap();
        assert_eq!(expired.into_inner(), key);

        // ストアから削除確認
        assert!(store.remove(&key).is_some());
    }
}
```

### 2.3. テストカバレッジ

```bash
# カバレッジ測定 (tarpaulin使用)
cargo install cargo-tarpaulin
cargo tarpaulin --out Html --output-dir ./coverage

# 目標カバレッジ: 80%以上
```

---

## 3. 統合テスト (Integration Tests)

### 3.1. gRPC通信テスト

```rust
// tests/integration/dfl_dcr_communication.rs

use tonic::transport::Server;
use cdde_proto::core_router_service_server::CoreRouterServiceServer;

#[tokio::test]
async fn test_dfl_to_dcr_packet_forwarding() {
    // 1. モックDCRサーバー起動
    let dcr_addr = "127.0.0.1:50051".parse().unwrap();
    let dcr_service = MockDcrService::new();
    
    tokio::spawn(async move {
        Server::builder()
            .add_service(CoreRouterServiceServer::new(dcr_service))
            .serve(dcr_addr)
            .await
            .unwrap();
    });

    // 2. DFLクライアント作成
    let dfl_client = DflClient::connect("http://127.0.0.1:50051").await.unwrap();

    // 3. Diameterパケット送信
    let packet = create_test_diameter_packet();
    let response = dfl_client.process_packet(packet).await.unwrap();

    // 4. レスポンス検証
    assert_eq!(response.action_type, ActionType::Forward);
    assert!(response.response_payload.len() > 0);
}

struct MockDcrService;

#[tonic::async_trait]
impl CoreRouterService for MockDcrService {
    async fn process_stream(
        &self,
        request: Request<Streaming<DiameterPacketRequest>>,
    ) -> Result<Response<Streaming<DiameterPacketAction>>, Status> {
        // モック実装
        // ...
    }
}
```

### 3.2. データベーステスト (CMS)

```rust
// tests/integration/cms_database.rs

use sqlx::PgPool;

#[tokio::test]
async fn test_cms_vr_crud_operations() {
    // テスト用DBコンテナ起動 (testcontainers使用)
    let postgres = testcontainers::clients::Cli::default()
        .run(testcontainers::images::postgres::Postgres::default());
    
    let connection_string = format!(
        "postgres://postgres:postgres@localhost:{}/postgres",
        postgres.get_host_port_ipv4(5432)
    );

    let pool = PgPool::connect(&connection_string).await.unwrap();

    // マイグレーション実行
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    // VR作成
    let vr = VirtualRouter {
        id: "vr001".to_string(),
        hostname: "dcr-vr001.test".to_string(),
        realm: "test.realm".to_string(),
    };

    let inserted_id = cms::create_vr(&pool, &vr).await.unwrap();
    assert_eq!(inserted_id, "vr001");

    // VR取得
    let fetched_vr = cms::get_vr(&pool, "vr001").await.unwrap();
    assert_eq!(fetched_vr.hostname, "dcr-vr001.test");

    // VR削除
    cms::delete_vr(&pool, "vr001").await.unwrap();
    assert!(cms::get_vr(&pool, "vr001").await.is_err());
}
```

---

## 4. システムテスト (E2E Tests)

### 4.1. Diameterシナリオテスト

```python
# tests/e2e/test_s6a_ulr_ula.py

import diameter
import pytest

@pytest.mark.e2e
def test_s6a_update_location_request():
    """S6a ULR/ULA シナリオテスト"""
    
    # 1. DFLへ接続
    client = diameter.Client(
        host="192.168.1.10",
        port=3868,
        origin_host="test-mme.operator.net",
        origin_realm="operator.net"
    )
    client.connect()

    # 2. ULR (Update-Location-Request) 送信
    ulr = diameter.Message(
        command_code=316,
        application_id=16777251,  # S6a
        avps=[
            diameter.AVP(code=1, data="user@operator.net"),  # User-Name
            diameter.AVP(code=1407, data=b"\x12\x34\x56"),   # Visited-PLMN-ID
            diameter.AVP(code=1405, data=1),                 # ULR-Flags
        ]
    )
    
    # 3. ULA (Update-Location-Answer) 受信
    ula = client.send_and_wait(ulr, timeout=5.0)
    
    # 4. 検証
    assert ula.command_code == 316
    assert ula.is_answer()
    
    result_code = ula.get_avp(268)  # Result-Code
    assert result_code.data == 2001  # DIAMETER_SUCCESS
    
    subscription_data = ula.get_avp(1400)  # Subscription-Data
    assert subscription_data is not None

    client.disconnect()
```

### 4.2. Kubernetes環境でのE2Eテスト

```yaml
# tests/e2e/k8s-test-job.yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: cdde-e2e-test
  namespace: cdde-system
spec:
  template:
    spec:
      containers:
      - name: test-runner
        image: cdde/e2e-tests:latest
        env:
        - name: DFL_HOST
          value: "dfl-svc.cdde-system.svc.cluster.local"
        - name: DFL_PORT
          value: "3868"
        command: ["pytest", "/tests/e2e", "-v"]
      restartPolicy: Never
  backoffLimit: 3
```

```bash
# E2Eテスト実行
kubectl apply -f tests/e2e/k8s-test-job.yaml
kubectl logs -n cdde-system job/cdde-e2e-test
```

---

## 5. 負荷テスト (Load Tests)

### 5.1. 負荷テストツール

**推奨ツール**: `seagull` (Diameter負荷テストツール)

```xml
<!-- seagull-scenario.xml -->
<scenario>
  <traffic>
    <send channel="channel-1">
      <action>
        <send>
          <diameter>
            <header command="316" application="16777251"/>
            <avp code="1" data="[user_name]"/>
            <avp code="1407" data="123456"/>
          </diameter>
        </send>
      </action>
    </send>
    
    <receive channel="channel-1">
      <action>
        <receive>
          <diameter>
            <header command="316" flag="answer"/>
          </diameter>
        </receive>
      </action>
    </receive>
  </traffic>
</scenario>
```

### 5.2. 負荷テストシナリオ

| シナリオ | TPS目標 | 同時接続数 | 継続時間 | 成功率目標 |
|:---|---:|---:|---:|---:|
| **軽負荷** | 1,000 | 100 | 10分 | 99.9% |
| **中負荷** | 10,000 | 500 | 30分 | 99.5% |
| **高負荷** | 50,000 | 2,000 | 1時間 | 99.0% |
| **ピーク負荷** | 100,000 | 5,000 | 10分 | 95.0% |

### 5.3. 負荷テスト実行

```bash
# Seagull起動
seagull -conf seagull.conf -scen seagull-scenario.xml -dico diameter-dict.xml

# メトリクス収集 (Prometheus)
kubectl port-forward -n cdde-system svc/prometheus 9090:9090

# Grafanaでリアルタイム監視
kubectl port-forward -n cdde-system svc/grafana 3000:3000
```

### 5.4. パフォーマンス指標

```promql
# TPS (Transactions Per Second)
rate(cdde_dfl_requests_total[1m])

# レイテンシ (P50, P95, P99)
histogram_quantile(0.50, cdde_dfl_latency_seconds_bucket)
histogram_quantile(0.95, cdde_dfl_latency_seconds_bucket)
histogram_quantile(0.99, cdde_dfl_latency_seconds_bucket)

# エラー率
rate(cdde_dfl_errors_total[1m]) / rate(cdde_dfl_requests_total[1m])
```

---

## 6. CI/CD パイプライン

### 6.1. GitHub Actions ワークフロー

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    
    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        components: rustfmt, clippy
    
    - name: Cache cargo
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/bin/
          ~/.cargo/registry/index/
          ~/.cargo/registry/cache/
          ~/.cargo/git/db/
          target/
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Run tests
      run: cargo test --all --verbose
    
    - name: Run clippy
      run: cargo clippy -- -D warnings
    
    - name: Check formatting
      run: cargo fmt -- --check
    
    - name: Coverage
      run: |
        cargo install cargo-tarpaulin
        cargo tarpaulin --out Xml
    
    - name: Upload coverage
      uses: codecov/codecov-action@v3

  integration-test:
    runs-on: ubuntu-latest
    needs: test
    steps:
    - uses: actions/checkout@v3
    
    - name: Start test environment
      run: docker-compose -f tests/docker-compose.test.yml up -d
    
    - name: Run integration tests
      run: cargo test --test '*' --verbose
    
    - name: Cleanup
      run: docker-compose -f tests/docker-compose.test.yml down
```

---

## 7. テストデータ管理

### 7.1. テストフィクスチャ

```rust
// tests/fixtures/mod.rs

pub fn create_test_diameter_packet() -> Vec<u8> {
    // Diameter Header (20 bytes)
    let mut packet = vec![
        0x01,       // Version
        0x00, 0x00, 0x64, // Length: 100 bytes
        0x80,       // Flags: Request
        0x00, 0x01, 0x3C, // Command Code: 316 (ULR)
        0x01, 0x00, 0x00, 0x28, // Application-ID: 16777251 (S6a)
        0x00, 0x00, 0x30, 0x39, // Hop-by-Hop ID
        0x00, 0x00, 0x00, 0x01, // End-to-End ID
    ];

    // AVPs
    // Session-ID (263)
    packet.extend_from_slice(&[
        0x00, 0x00, 0x01, 0x07, // AVP Code: 263
        0x40,       // Flags: Mandatory
        0x00, 0x00, 0x20, // Length: 32
        // Data: "test-session-12345"
    ]);

    packet
}
```

---

## 8. テスト環境

### 8.1. ローカル開発環境

```yaml
# docker-compose.test.yml
version: '3.8'
services:
  postgres:
    image: postgres:15-alpine
    environment:
      POSTGRES_DB: cdde_test
      POSTGRES_USER: test
      POSTGRES_PASSWORD: test
    ports:
      - "5432:5432"

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
```

### 8.2. Kubernetes テスト環境

```bash
# Kind (Kubernetes in Docker) でテストクラスタ作成
kind create cluster --name cdde-test --config tests/kind-config.yaml

# CDDEデプロイ
helm install cdde-test ./helm/cdde -n cdde-system --create-namespace

# テスト実行
kubectl apply -f tests/e2e/k8s-test-job.yaml
```
