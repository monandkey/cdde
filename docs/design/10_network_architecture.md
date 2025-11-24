## ネットワークアーキテクチャ詳細設計

---

## 1. 全体ネットワーク構成

CDDEシステムは、Kubernetes環境において **2つの独立したネットワーク層** を使用します。

```
┌─────────────────────────────────────────────────────────────┐
│                    Kubernetes Cluster                        │
│                                                               │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  External Network (Multus CNI)                       │   │
│  │  - SCTP/TCP with Diameter Peers                      │   │
│  │  - Physical NIC direct attachment                    │   │
│  │  - Multi-homing support                              │   │
│  └──────────────────────────────────────────────────────┘   │
│           ↕                                                   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  DFL Pods (Frontline)                                │   │
│  │  - net0: K8s default (10.244.x.x)                    │   │
│  │  - net1: External Primary (192.168.1.x)              │   │
│  │  - net2: External Secondary (192.168.2.x)            │   │
│  └──────────────────────────────────────────────────────┘   │
│           ↕                                                   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Internal Network (K8s CNI - Calico/Flannel)         │   │
│  │  - gRPC communication (DFL ↔ DCR ↔ DPA)              │   │
│  │  - Service discovery via K8s Service                 │   │
│  └──────────────────────────────────────────────────────┘   │
│           ↕                                                   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  DCR Pods (per VR)                                   │   │
│  │  DPA Pods                                            │   │
│  │  CMS, CM, CO, PC, FI                                 │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Multus CNI による外部ネットワーク接続

### 2.1. Multus CNI とは

**Multus CNI** は、Kubernetes Podに複数のネットワークインターフェースを割り当てることを可能にするCNIプラグインです。

- **デフォルトネットワーク (net0)**: K8sクラスタ内部通信用 (Calico/Flannel等)
- **追加ネットワーク (net1, net2, ...)**: 外部物理ネットワーク接続用

### 2.2. NetworkAttachmentDefinition (NAD)

Multusでは、`NetworkAttachmentDefinition` CRDを使用して追加ネットワークを定義します。

#### 例: Macvlan を使用した外部ネットワーク定義

```yaml
# network-external-primary.yaml
apiVersion: k8s.cni.cncf.io/v1
kind: NetworkAttachmentDefinition
metadata:
  name: external-primary
  namespace: cdde-system
spec:
  config: |
    {
      "cniVersion": "0.3.1",
      "type": "macvlan",
      "master": "eth1",
      "mode": "bridge",
      "ipam": {
        "type": "whereabouts",
        "range": "192.168.1.0/24",
        "gateway": "192.168.1.1",
        "routes": [
          { "dst": "0.0.0.0/0" }
        ]
      }
    }
---
# network-external-secondary.yaml
apiVersion: k8s.cni.cncf.io/v1
kind: NetworkAttachmentDefinition
metadata:
  name: external-secondary
  namespace: cdde-system
spec:
  config: |
    {
      "cniVersion": "0.3.1",
      "type": "macvlan",
      "master": "eth2",
      "mode": "bridge",
      "ipam": {
        "type": "whereabouts",
        "range": "192.168.2.0/24",
        "gateway": "192.168.2.1"
      }
    }
```

### 2.3. DFL Pod への Multus 適用

```yaml
# dfl-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: dfl
  namespace: cdde-system
spec:
  replicas: 2
  selector:
    matchLabels:
      app: dfl
  template:
    metadata:
      labels:
        app: dfl
      annotations:
        # Multus annotations
        k8s.v1.cni.cncf.io/networks: |
          [
            {
              "name": "external-primary",
              "interface": "net1",
              "ips": ["192.168.1.10"]
            },
            {
              "name": "external-secondary",
              "interface": "net2",
              "ips": ["192.168.2.10"]
            }
          ]
    spec:
      containers:
      - name: dfl
        image: cdde/dfl:latest
        securityContext:
          capabilities:
            add: ["NET_ADMIN", "NET_RAW"]  # SCTP用
        env:
        - name: PRIMARY_IP
          value: "192.168.1.10"
        - name: SECONDARY_IP
          value: "192.168.2.10"
```

---

## 3. SCTP Multi-homing 実装

### 3.1. SCTP Multi-homing とは

SCTP Multi-homingは、単一のSCTPアソシエーションが複数のIPアドレスを使用できる機能です。これにより、ネットワーク障害時の自動フェイルオーバーが可能になります。

### 3.2. Rust での SCTP Multi-homing 実装

```rust
// network/src/sctp.rs

use std::net::{IpAddr, SocketAddr};
use libc::{sctp_bindx, SCTP_BINDX_ADD_ADDR};

pub struct SctpMultihomeListener {
    primary_addr: IpAddr,
    secondary_addr: IpAddr,
    port: u16,
}

impl SctpMultihomeListener {
    pub fn new(primary: IpAddr, secondary: IpAddr, port: u16) -> Self {
        Self {
            primary_addr: primary,
            secondary_addr: secondary,
            port,
        }
    }

    pub fn bind(&self) -> Result<SctpSocket, std::io::Error> {
        // 1. プライマリアドレスでソケット作成
        let sock = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::STREAM,
            Some(socket2::Protocol::from(132)), // IPPROTO_SCTP
        )?;

        // 2. プライマリアドレスをバインド
        let primary_sockaddr = SocketAddr::new(self.primary_addr, self.port);
        sock.bind(&primary_sockaddr.into())?;

        // 3. セカンダリアドレスを追加 (sctp_bindx)
        let secondary_sockaddr = SocketAddr::new(self.secondary_addr, self.port);
        unsafe {
            let addrs = [secondary_sockaddr];
            let ret = sctp_bindx(
                sock.as_raw_fd(),
                addrs.as_ptr() as *const libc::sockaddr,
                1,
                SCTP_BINDX_ADD_ADDR,
            );
            if ret < 0 {
                return Err(std::io::Error::last_os_error());
            }
        }

        // 4. Listen開始
        sock.listen(128)?;

        Ok(SctpSocket { inner: sock })
    }
}
```

### 3.3. SCTP Heartbeat 設定

```rust
use libc::{setsockopt, IPPROTO_SCTP, SCTP_PEER_ADDR_PARAMS};

pub fn configure_sctp_heartbeat(sock: &SctpSocket, interval_ms: u32) -> Result<(), std::io::Error> {
    let params = libc::sctp_paddrparams {
        spp_hbinterval: interval_ms,
        spp_flags: libc::SPP_HB_ENABLE,
        ..Default::default()
    };

    unsafe {
        let ret = setsockopt(
            sock.as_raw_fd(),
            IPPROTO_SCTP,
            SCTP_PEER_ADDR_PARAMS,
            &params as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::sctp_paddrparams>() as u32,
        );
        if ret < 0 {
            return Err(std::io::Error::last_os_error());
        }
    }

    Ok(())
}
```

---

## 4. IP アドレス管理 (IPAM)

### 4.1. Whereabouts IPAM

**Whereabouts** は、Kubernetes向けのIPアドレス管理プラグインで、Multusと組み合わせて使用します。

#### インストール

```bash
kubectl apply -f https://raw.githubusercontent.com/k8snetworkplumbingwg/whereabouts/master/doc/crds/daemonset-install.yaml
kubectl apply -f https://raw.githubusercontent.com/k8snetworkplumbingwg/whereabouts/master/doc/crds/whereabouts.cni.cncf.io_ippools.yaml
kubectl apply -f https://raw.githubusercontent.com/k8snetworkplumbingwg/whereabouts/master/doc/crds/whereabouts.cni.cncf.io_overlappingrangeipreservations.yaml
```

#### IP範囲の予約

```yaml
apiVersion: whereabouts.cni.cncf.io/v1alpha1
kind: IPPool
metadata:
  name: external-primary-pool
  namespace: cdde-system
spec:
  range: 192.168.1.10-192.168.1.50
  exclude:
    - 192.168.1.1/32  # Gateway
    - 192.168.1.254/32  # Reserved
```

---

## 5. 内部ネットワーク (gRPC通信)

### 5.1. Kubernetes Service による Service Discovery

```yaml
# dcr-service.yaml (VR毎に作成)
apiVersion: v1
kind: Service
metadata:
  name: dcr-svc-vr001
  namespace: cdde-system
spec:
  selector:
    app: dcr
    vr-id: vr001
  ports:
  - name: grpc
    protocol: TCP
    port: 50051
    targetPort: 50051
  type: ClusterIP
```

### 5.2. DFL から DCR への接続

```rust
// dfl/src/grpc_client.rs

use tonic::transport::Channel;

pub async fn connect_to_dcr(vr_id: &str) -> Result<Channel, tonic::transport::Error> {
    let service_name = format!("dcr-svc-{}", vr_id);
    let endpoint = format!("http://{}:50051", service_name);
    
    Channel::from_shared(endpoint)?
        .connect()
        .await
}
```

---

## 6. ネットワークポリシー (セキュリティ)

### 6.1. DFL への外部アクセス許可

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: dfl-external-ingress
  namespace: cdde-system
spec:
  podSelector:
    matchLabels:
      app: dfl
  policyTypes:
  - Ingress
  ingress:
  # 外部ネットワークからのSCTP/TCP許可
  - from:
    - ipBlock:
        cidr: 192.168.0.0/16
    ports:
    - protocol: SCTP
      port: 3868
    - protocol: TCP
      port: 3868
```

### 6.2. 内部通信の制限

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: dcr-internal-only
  namespace: cdde-system
spec:
  podSelector:
    matchLabels:
      app: dcr
  policyTypes:
  - Ingress
  ingress:
  # DFLからのgRPC通信のみ許可
  - from:
    - podSelector:
        matchLabels:
          app: dfl
    ports:
    - protocol: TCP
      port: 50051
```

---

## 7. パフォーマンス最適化

### 7.1. CPU Pinning (Node Affinity)

```yaml
spec:
  template:
    spec:
      affinity:
        nodeAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
            nodeSelectorTerms:
            - matchExpressions:
              - key: node-role.kubernetes.io/worker
                operator: In
                values:
                - high-performance
      containers:
      - name: dfl
        resources:
          requests:
            cpu: "4"
            memory: "8Gi"
          limits:
            cpu: "4"
            memory: "8Gi"
```

### 7.2. Huge Pages 設定

```yaml
spec:
  containers:
  - name: dfl
    resources:
      requests:
        hugepages-2Mi: 1Gi
      limits:
        hugepages-2Mi: 1Gi
    volumeMounts:
    - name: hugepage
      mountPath: /dev/hugepages
  volumes:
  - name: hugepage
    emptyDir:
      medium: HugePages
```

---

## 8. トラブルシューティング

### 8.1. Multus 接続確認

```bash
# Pod内でネットワークインターフェース確認
kubectl exec -it dfl-pod-xxx -- ip addr show

# 期待される出力:
# 1: lo: ...
# 2: eth0@if123: ... (K8s default network)
# 3: net1@if124: ... (External primary - 192.168.1.10)
# 4: net2@if125: ... (External secondary - 192.168.2.10)
```

### 8.2. SCTP 接続テスト

```bash
# SCTP Listen確認
kubectl exec -it dfl-pod-xxx -- ss -ln | grep 3868

# 期待される出力:
# LISTEN 0 128 192.168.1.10:3868 *:*
# LISTEN 0 128 192.168.2.10:3868 *:*
```
