## Kubernetes デプロイメントガイド

---

## 1. 前提条件

### 1.1. Kubernetes クラスタ要件

- **Kubernetesバージョン**: 1.24以上
- **CNI**: Calico または Flannel (デフォルトネットワーク用)
- **追加CNI**: Multus CNI (外部ネットワーク用)
- **ノード要件**:
  - CPU: 最低8コア (推奨16コア以上)
  - メモリ: 最低16GB (推奨32GB以上)
  - ネットワーク: 複数物理NIC (SCTP Multi-homing用)

### 1.2. 必要なツール

```bash
# kubectl
curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"

# Helm
curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash

# Multus CNI
kubectl apply -f https://raw.githubusercontent.com/k8snetworkplumbingwg/multus-cni/master/deployments/multus-daemonset.yml
```

---

## 2. Namespace 作成

```yaml
# namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: cdde-system
  labels:
    name: cdde-system
    app.kubernetes.io/name: cdde
```

```bash
kubectl apply -f namespace.yaml
```

---

## 3. ConfigMap と Secret

### 3.1. 共通設定 ConfigMap

```yaml
# config-common.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: cdde-common-config
  namespace: cdde-system
data:
  log_level: "info"
  metrics_port: "9090"
  grpc_port: "50051"
  
  # PostgreSQL接続情報 (CMS用)
  postgres_host: "postgres-svc.cdde-system.svc.cluster.local"
  postgres_port: "5432"
  postgres_database: "cdde"
```

### 3.2. Secrets (認証情報)

```yaml
# secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: cdde-secrets
  namespace: cdde-system
type: Opaque
stringData:
  postgres_user: "cdde_admin"
  postgres_password: "CHANGE_ME_IN_PRODUCTION"
```

```bash
kubectl apply -f secrets.yaml
```

---

## 4. PostgreSQL デプロイ (CMS用)

### 4.1. StatefulSet

```yaml
# postgres-statefulset.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: postgres
  namespace: cdde-system
spec:
  serviceName: postgres-svc
  replicas: 1
  selector:
    matchLabels:
      app: postgres
  template:
    metadata:
      labels:
        app: postgres
    spec:
      containers:
      - name: postgres
        image: postgres:15-alpine
        ports:
        - containerPort: 5432
          name: postgres
        env:
        - name: POSTGRES_DB
          valueFrom:
            configMapKeyRef:
              name: cdde-common-config
              key: postgres_database
        - name: POSTGRES_USER
          valueFrom:
            secretKeyRef:
              name: cdde-secrets
              key: postgres_user
        - name: POSTGRES_PASSWORD
          valueFrom:
            secretKeyRef:
              name: cdde-secrets
              key: postgres_password
        volumeMounts:
        - name: postgres-storage
          mountPath: /var/lib/postgresql/data
  volumeClaimTemplates:
  - metadata:
      name: postgres-storage
    spec:
      accessModes: ["ReadWriteOnce"]
      resources:
        requests:
          storage: 20Gi
---
apiVersion: v1
kind: Service
metadata:
  name: postgres-svc
  namespace: cdde-system
spec:
  selector:
    app: postgres
  ports:
  - port: 5432
    targetPort: 5432
  clusterIP: None
```

---

## 5. Multus ネットワーク定義

```yaml
# network-attachments.yaml
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
        "gateway": "192.168.1.1"
      }
    }
---
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

---

## 6. Core アプリケーションのデプロイ

### 6.1. DFL (Diameter Frontline)

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
        k8s.v1.cni.cncf.io/networks: |
          [
            {"name": "external-primary", "interface": "net1"},
            {"name": "external-secondary", "interface": "net2"}
          ]
    spec:
      containers:
      - name: dfl
        image: cdde/dfl:v1.0.0
        ports:
        - containerPort: 3868
          name: diameter
          protocol: SCTP
        - containerPort: 50051
          name: grpc
        - containerPort: 9090
          name: metrics
        env:
        - name: LOG_LEVEL
          valueFrom:
            configMapKeyRef:
              name: cdde-common-config
              key: log_level
        - name: PRIMARY_IP
          value: "192.168.1.10"
        - name: SECONDARY_IP
          value: "192.168.2.10"
        resources:
          requests:
            cpu: "2"
            memory: "4Gi"
          limits:
            cpu: "4"
            memory: "8Gi"
        securityContext:
          capabilities:
            add: ["NET_ADMIN", "NET_RAW"]
---
apiVersion: v1
kind: Service
metadata:
  name: dfl-svc
  namespace: cdde-system
spec:
  selector:
    app: dfl
  ports:
  - name: grpc
    port: 50051
    targetPort: 50051
  - name: metrics
    port: 9090
    targetPort: 9090
  type: ClusterIP
```

### 6.2. DCR (Diameter Core Router) - VR毎

```yaml
# dcr-vr001-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: dcr-vr001
  namespace: cdde-system
spec:
  replicas: 2
  selector:
    matchLabels:
      app: dcr
      vr-id: vr001
  template:
    metadata:
      labels:
        app: dcr
        vr-id: vr001
    spec:
      containers:
      - name: dcr
        image: cdde/dcr:v1.0.0
        ports:
        - containerPort: 50051
          name: grpc
        - containerPort: 9090
          name: metrics
        env:
        - name: VR_ID
          value: "vr001"
        - name: LOG_LEVEL
          valueFrom:
            configMapKeyRef:
              name: cdde-common-config
              key: log_level
        volumeMounts:
        - name: vr-config
          mountPath: /etc/cdde/vr
          readOnly: true
        - name: vendor-dict
          mountPath: /etc/cdde/dictionaries
          readOnly: true
        resources:
          requests:
            cpu: "1"
            memory: "2Gi"
          limits:
            cpu: "2"
            memory: "4Gi"
      volumes:
      - name: vr-config
        configMap:
          name: dcr-vr001-config
      - name: vendor-dict
        configMap:
          name: vendor-dictionary
---
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
    port: 50051
    targetPort: 50051
  type: ClusterIP
```

### 6.3. DPA (Diameter Peer Agent)

```yaml
# dpa-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: dpa
  namespace: cdde-system
spec:
  replicas: 2
  selector:
    matchLabels:
      app: dpa
  template:
    metadata:
      labels:
        app: dpa
    spec:
      containers:
      - name: dpa
        image: cdde/dpa:v1.0.0
        ports:
        - containerPort: 50051
          name: grpc
        - containerPort: 9090
          name: metrics
        env:
        - name: LOG_LEVEL
          valueFrom:
            configMapKeyRef:
              name: cdde-common-config
              key: log_level
        resources:
          requests:
            cpu: "500m"
            memory: "1Gi"
          limits:
            cpu: "1"
            memory: "2Gi"
```

### 6.4. CMS (Config & Management Service)

```yaml
# cms-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: cms
  namespace: cdde-system
spec:
  replicas: 2
  selector:
    matchLabels:
      app: cms
  template:
    metadata:
      labels:
        app: cms
    spec:
      containers:
      - name: cms
        image: cdde/cms:v1.0.0
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 50051
          name: grpc
        env:
        - name: POSTGRES_HOST
          valueFrom:
            configMapKeyRef:
              name: cdde-common-config
              key: postgres_host
        - name: POSTGRES_USER
          valueFrom:
            secretKeyRef:
              name: cdde-secrets
              key: postgres_user
        - name: POSTGRES_PASSWORD
          valueFrom:
            secretKeyRef:
              name: cdde-secrets
              key: postgres_password
        resources:
          requests:
            cpu: "500m"
            memory: "1Gi"
          limits:
            cpu: "1"
            memory: "2Gi"
---
apiVersion: v1
kind: Service
metadata:
  name: cms-svc
  namespace: cdde-system
spec:
  selector:
    app: cms
  ports:
  - name: http
    port: 8080
    targetPort: 8080
  - name: grpc
    port: 50051
    targetPort: 50051
  type: LoadBalancer
```

---

## 7. Helm Chart 構成

### 7.1. Chart.yaml

```yaml
apiVersion: v2
name: cdde
description: Cloud Diameter Distribution Engine
type: application
version: 1.0.0
appVersion: "1.0.0"
```

### 7.2. values.yaml

```yaml
global:
  namespace: cdde-system
  imageRegistry: docker.io/cdde
  imagePullPolicy: IfNotPresent

dfl:
  replicas: 2
  image:
    repository: dfl
    tag: v1.0.0
  resources:
    requests:
      cpu: 2
      memory: 4Gi
    limits:
      cpu: 4
      memory: 8Gi
  externalNetwork:
    primary:
      ip: 192.168.1.10
    secondary:
      ip: 192.168.2.10

dcr:
  virtualRouters:
    - id: vr001
      replicas: 2
      resources:
        requests:
          cpu: 1
          memory: 2Gi

postgres:
  enabled: true
  storage: 20Gi
  credentials:
    user: cdde_admin
    password: CHANGE_ME
```

### 7.3. デプロイコマンド

```bash
# Helm Chart インストール
helm install cdde ./helm/cdde -n cdde-system --create-namespace

# アップグレード
helm upgrade cdde ./helm/cdde -n cdde-system

# アンインストール
helm uninstall cdde -n cdde-system
```

---

## 8. 動作確認

### 8.1. Pod 状態確認

```bash
kubectl get pods -n cdde-system
```

期待される出力:
```
NAME                          READY   STATUS    RESTARTS   AGE
dfl-xxxxxxxxxx-xxxxx          1/1     Running   0          5m
dfl-xxxxxxxxxx-xxxxx          1/1     Running   0          5m
dcr-vr001-xxxxxxxx-xxxxx      1/1     Running   0          5m
dpa-xxxxxxxxxx-xxxxx          1/1     Running   0          5m
cms-xxxxxxxxxx-xxxxx          1/1     Running   0          5m
postgres-0                    1/1     Running   0          5m
```

### 8.2. ログ確認

```bash
# DFLのログ
kubectl logs -n cdde-system -l app=dfl --tail=100

# DCRのログ
kubectl logs -n cdde-system -l app=dcr,vr-id=vr001 --tail=100
```

### 8.3. メトリクス確認

```bash
# Prometheusメトリクスエンドポイント
kubectl port-forward -n cdde-system svc/dfl-svc 9090:9090
curl http://localhost:9090/metrics
```

---

## 9. トラブルシューティング

### 9.1. Multus接続確認

```bash
kubectl exec -it -n cdde-system dfl-xxxxxxxxxx-xxxxx -- ip addr show
```

### 9.2. SCTP接続確認

```bash
kubectl exec -it -n cdde-system dfl-xxxxxxxxxx-xxxxx -- ss -ln | grep 3868
```

### 9.3. gRPC通信確認

```bash
kubectl exec -it -n cdde-system dfl-xxxxxxxxxx-xxxxx -- grpcurl -plaintext dcr-svc-vr001:50051 list
```
