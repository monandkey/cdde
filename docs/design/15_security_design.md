## セキュリティ設計

---

## 1. セキュリティ方針

CDDEシステムのセキュリティは、以下の原則に基づきます:

| 原則 | 説明 | 実装方法 |
|:---|:---|:---|
| **Defense in Depth** | 多層防御 | ネットワーク層、アプリケーション層、データ層での保護 |
| **Least Privilege** | 最小権限の原則 | RBAC、Pod Security Policy |
| **Zero Trust** | ゼロトラストネットワーク | mTLS、認証・認可の徹底 |
| **Encryption** | データ暗号化 | TLS/DTLS、保存データ暗号化 |

---

## 2. TLS/DTLS 対応

### 2.1. Diameter over TLS (RFC 6733)

```rust
// network/src/tls.rs

use rustls::{ServerConfig, ClientConfig};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use std::sync::Arc;

pub struct DiameterTlsConfig {
    server_config: Arc<ServerConfig>,
    client_config: Arc<ClientConfig>,
}

impl DiameterTlsConfig {
    pub fn new(
        cert_path: &str,
        key_path: &str,
        ca_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // サーバー証明書ロード
        let certs = load_certs(cert_path)?;
        let key = load_private_key(key_path)?;

        // CA証明書ロード (クライアント認証用)
        let ca_certs = load_certs(ca_path)?;
        let mut root_store = rustls::RootCertStore::empty();
        for cert in ca_certs {
            root_store.add(&cert)?;
        }

        // サーバー設定 (mTLS有効化)
        let mut server_config = ServerConfig::builder()
            .with_safe_defaults()
            .with_client_cert_verifier(
                rustls::server::AllowAnyAuthenticatedClient::new(root_store.clone())
            )
            .with_single_cert(certs.clone(), key.clone())?;

        // クライアント設定
        let client_config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_client_auth_cert(certs, key)?;

        Ok(Self {
            server_config: Arc::new(server_config),
            client_config: Arc::new(client_config),
        })
    }

    pub fn acceptor(&self) -> TlsAcceptor {
        TlsAcceptor::from(self.server_config.clone())
    }

    pub fn connector(&self) -> TlsConnector {
        TlsConnector::from(self.client_config.clone())
    }
}

// TLS接続の確立
pub async fn accept_tls_connection(
    stream: TcpStream,
    acceptor: &TlsAcceptor,
) -> Result<TlsStream<TcpStream>, std::io::Error> {
    acceptor.accept(stream).await
}
```

### 2.2. 証明書管理 (cert-manager)

```yaml
# certificate.yaml
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: dfl-tls-cert
  namespace: cdde-system
spec:
  secretName: dfl-tls-secret
  issuerRef:
    name: ca-issuer
    kind: ClusterIssuer
  commonName: dfl.cdde-system.svc.cluster.local
  dnsNames:
  - dfl.cdde-system.svc.cluster.local
  - "*.dfl.cdde-system.svc.cluster.local"
  - 192.168.1.10
  ipAddresses:
  - 192.168.1.10
  - 192.168.2.10
  duration: 8760h  # 1年
  renewBefore: 720h  # 30日前に更新
---
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: ca-issuer
spec:
  ca:
    secretName: ca-key-pair
```

### 2.3. Pod への証明書マウント

```yaml
spec:
  containers:
  - name: dfl
    volumeMounts:
    - name: tls-certs
      mountPath: /etc/cdde/tls
      readOnly: true
  volumes:
  - name: tls-certs
    secret:
      secretName: dfl-tls-secret
```

---

## 3. 認証・認可

### 3.1. Diameter CER/CEA による Peer 認証

```rust
// dpa/src/auth.rs

use std::collections::HashSet;

pub struct PeerAuthenticator {
    // 許可されたOrigin-Hostのリスト
    allowed_hosts: HashSet<String>,
    // 許可されたOrigin-Realmのリスト
    allowed_realms: HashSet<String>,
}

impl PeerAuthenticator {
    pub fn authenticate_cer(&self, cer: &CapabilityExchangeRequest) -> Result<(), AuthError> {
        // 1. Origin-Host検証
        let origin_host = cer.get_avp(AVP_ORIGIN_HOST)
            .ok_or(AuthError::MissingOriginHost)?;
        
        if !self.allowed_hosts.contains(&origin_host) {
            return Err(AuthError::UnauthorizedHost(origin_host));
        }

        // 2. Origin-Realm検証
        let origin_realm = cer.get_avp(AVP_ORIGIN_REALM)
            .ok_or(AuthError::MissingOriginRealm)?;
        
        if !self.allowed_realms.contains(&origin_realm) {
            return Err(AuthError::UnauthorizedRealm(origin_realm));
        }

        // 3. Application-ID検証
        let supported_apps = cer.get_all_avps(AVP_AUTH_APPLICATION_ID);
        if supported_apps.is_empty() {
            return Err(AuthError::NoSupportedApplications);
        }

        Ok(())
    }

    pub fn generate_cea(&self, result: Result<(), AuthError>) -> CapabilityExchangeAnswer {
        let result_code = match result {
            Ok(_) => 2001,  // DIAMETER_SUCCESS
            Err(AuthError::UnauthorizedHost(_)) => 5012,  // DIAMETER_UNKNOWN_PEER
            Err(AuthError::NoSupportedApplications) => 5010,  // DIAMETER_NO_COMMON_APPLICATION
            _ => 3010,  // DIAMETER_UNABLE_TO_COMPLY
        };

        CapabilityExchangeAnswer {
            result_code,
            origin_host: self.my_host.clone(),
            origin_realm: self.my_realm.clone(),
            // ...
        }
    }
}
```

### 3.2. CMS API 認証 (JWT)

```rust
// cms/src/auth.rs

use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,  // ユーザーID
    role: String,  // admin, operator, viewer
    exp: usize,  // 有効期限
}

pub fn generate_token(user_id: &str, role: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .unwrap()
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        role: role.to_string(),
        exp: expiration,
    };

    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
}

pub fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )?;

    Ok(token_data.claims)
}

// Axum middleware
pub async fn auth_middleware(
    headers: HeaderMap,
    request: Request<Body>,
    next: Next<Body>,
) -> Result<Response, StatusCode> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = validate_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // リクエストにクレームを追加
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}
```

---

## 4. アクセス制御 (RBAC)

### 4.1. Kubernetes RBAC

```yaml
# rbac.yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: dfl-sa
  namespace: cdde-system
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: dfl-role
  namespace: cdde-system
rules:
- apiGroups: [""]
  resources: ["configmaps"]
  verbs: ["get", "list", "watch"]
- apiGroups: [""]
  resources: ["secrets"]
  verbs: ["get"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: dfl-rolebinding
  namespace: cdde-system
subjects:
- kind: ServiceAccount
  name: dfl-sa
roleRef:
  kind: Role
  name: dfl-role
  apiGroup: rbac.authorization.k8s.io
```

### 4.2. Pod Security Policy

```yaml
# pod-security-policy.yaml
apiVersion: policy/v1beta1
kind: PodSecurityPolicy
metadata:
  name: cdde-restricted
spec:
  privileged: false
  allowPrivilegeEscalation: false
  requiredDropCapabilities:
  - ALL
  allowedCapabilities:
  - NET_ADMIN  # SCTP用
  - NET_RAW
  volumes:
  - 'configMap'
  - 'secret'
  - 'emptyDir'
  - 'persistentVolumeClaim'
  runAsUser:
    rule: 'MustRunAsNonRoot'
  seLinux:
    rule: 'RunAsAny'
  fsGroup:
    rule: 'RunAsAny'
  readOnlyRootFilesystem: true
```

---

## 5. ネットワークセキュリティ

### 5.1. Network Policy (Ingress/Egress制限)

```yaml
# network-policy.yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: dfl-network-policy
  namespace: cdde-system
spec:
  podSelector:
    matchLabels:
      app: dfl
  policyTypes:
  - Ingress
  - Egress
  ingress:
  # 外部Peerからの接続許可
  - from:
    - ipBlock:
        cidr: 192.168.0.0/16
    ports:
    - protocol: TCP
      port: 3868
    - protocol: SCTP
      port: 3868
  egress:
  # DCRへのgRPC通信許可
  - to:
    - podSelector:
        matchLabels:
          app: dcr
    ports:
    - protocol: TCP
      port: 50051
  # DNS許可
  - to:
    - namespaceSelector:
        matchLabels:
          name: kube-system
    ports:
    - protocol: UDP
      port: 53
```

### 5.2. Service Mesh (Istio) による mTLS

```yaml
# istio-mtls.yaml
apiVersion: security.istio.io/v1beta1
kind: PeerAuthentication
metadata:
  name: default
  namespace: cdde-system
spec:
  mtls:
    mode: STRICT
---
apiVersion: security.istio.io/v1beta1
kind: AuthorizationPolicy
metadata:
  name: dfl-authz
  namespace: cdde-system
spec:
  selector:
    matchLabels:
      app: dfl
  action: ALLOW
  rules:
  - from:
    - source:
        principals: ["cluster.local/ns/cdde-system/sa/dcr-sa"]
    to:
    - operation:
        methods: ["POST"]
        paths: ["/cdde.internal.CoreRouterService/*"]
```

---

## 6. データ保護

### 6.1. Secrets 暗号化 (Sealed Secrets)

```bash
# Sealed Secrets インストール
kubectl apply -f https://github.com/bitnami-labs/sealed-secrets/releases/download/v0.18.0/controller.yaml

# Secret暗号化
kubeseal --format yaml < secret.yaml > sealed-secret.yaml
```

```yaml
# sealed-secret.yaml (暗号化済み)
apiVersion: bitnami.com/v1alpha1
kind: SealedSecret
metadata:
  name: cdde-secrets
  namespace: cdde-system
spec:
  encryptedData:
    postgres_password: AgBx7Hn...encrypted...
```

### 6.2. 保存データ暗号化 (etcd)

```yaml
# encryption-config.yaml
apiVersion: apiserver.config.k8s.io/v1
kind: EncryptionConfiguration
resources:
- resources:
  - secrets
  providers:
  - aescbc:
      keys:
      - name: key1
        secret: <BASE64_ENCODED_SECRET>
  - identity: {}
```

---

## 7. 監査ログ

### 7.1. Kubernetes Audit Policy

```yaml
# audit-policy.yaml
apiVersion: audit.k8s.io/v1
kind: Policy
rules:
# Secrets アクセスログ
- level: RequestResponse
  resources:
  - group: ""
    resources: ["secrets"]
  namespaces: ["cdde-system"]

# ConfigMap変更ログ
- level: Metadata
  resources:
  - group: ""
    resources: ["configmaps"]
  namespaces: ["cdde-system"]
  verbs: ["create", "update", "patch", "delete"]
```

### 7.2. アプリケーション監査ログ

```rust
// cms/src/audit.rs

use tracing::info;

pub fn log_config_change(user: &str, resource: &str, action: &str, details: &str) {
    info!(
        event_type = "config_change",
        user = user,
        resource = resource,
        action = action,
        details = details,
        timestamp = chrono::Utc::now().to_rfc3339(),
        "Configuration change detected"
    );
}

// 使用例
log_config_change(
    "admin@example.com",
    "VirtualRouter/vr001",
    "UPDATE",
    "Changed timeout from 5000ms to 10000ms"
);
```

---

## 8. 脆弱性管理

### 8.1. コンテナイメージスキャン (Trivy)

```yaml
# .github/workflows/security-scan.yml
name: Security Scan

on:
  push:
    branches: [main]
  pull_request:

jobs:
  trivy-scan:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    
    - name: Build image
      run: docker build -t cdde/dfl:test .
    
    - name: Run Trivy vulnerability scanner
      uses: aquasecurity/trivy-action@master
      with:
        image-ref: 'cdde/dfl:test'
        format: 'sarif'
        output: 'trivy-results.sarif'
        severity: 'CRITICAL,HIGH'
    
    - name: Upload results to GitHub Security
      uses: github/codeql-action/upload-sarif@v2
      with:
        sarif_file: 'trivy-results.sarif'
```

### 8.2. 依存関係監査

```bash
# Rust依存関係監査
cargo install cargo-audit
cargo audit

# 脆弱性修正
cargo update
```

---

## 9. インシデント対応

### 9.1. セキュリティインシデント検知

```yaml
# falco-rules.yaml
- rule: Unauthorized Process in Container
  desc: Detect unauthorized process execution
  condition: >
    spawned_process and
    container.name startswith "dfl" and
    not proc.name in (dfl, sh, bash)
  output: >
    Unauthorized process in DFL container
    (user=%user.name command=%proc.cmdline container=%container.name)
  priority: WARNING
```

### 9.2. インシデント対応手順

1. **検知**: Falco/Prometheusアラート
2. **隔離**: 影響を受けたPodの即時削除
3. **調査**: 監査ログ、トレースの分析
4. **復旧**: クリーンなイメージで再デプロイ
5. **事後対応**: 根本原因分析、再発防止策
