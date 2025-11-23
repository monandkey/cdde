
## 2. CMS (Config & Management Service) との連携

CMSは、設定情報の一元管理と、UI/API提供のハブです。

### 2.1. 設定データの永続化と配信モデル

CMSは、すべての設定をPostgreSQLなどのDBに永続化し、CM (Configuration Manager) へ配信します。

| 設定項目 | 管理責務 | 配信先 | 備考 |
| :--- | :--- | :--- | :--- |
| **VR定義** | CMS | CM $\to$ DCR, DFL | DCRは自身が担当するVR定義、DFLは全VRのIngress IPマッピング |
| **Peer定義** | CMS | CM $\to$ DPA, DFL | DPAは監視対象、DFLは接続先情報として使用 |
| **ルーティング/操作ルール** | CMS | CM $\to$ DCR | DCRのメインロジックとして使用 |
| **AM設定** | CMS | CM $\to$ DPA | DPAの監視パラメータとして使用 |

### 2.2. CMSのAPI設計

管理UI（User Interface Application）からのアクセスを受けるためのREST API（またはgRPC API）を定義します。

| APIエンドポイント | HTTP Method | 責務 | 備考 |
| :--- | :--- | :--- | :--- |
| `/v1/config/vr/{id}` | `GET / POST / PUT` | VR定義の CRUD | DCR POD再作成のトリガーとなる |
| `/v1/config/peer/{id}` | `GET / POST / PUT` | Peer Node/Pool定義の CRUD | DPA, DFLの接続情報に影響 |
| `/v1/stats/vr/{id}` | `GET` | 特定VRの統計情報取得 | PCからPullしたデータを統合して提供 |
| `/v1/alerts/active` | `GET` | 現在発生中のアラートリスト | FIからPullしたデータを統合して提供 |

### 2.3. CM (Configuration Manager) の役割詳細

CMは、CMSのDBをポーリング、またはDBの変更通知イベントを受け取ります。

1.  **変更検知:** CMS設定DBの変更を検知。
2.  **影響分析:** 変更された設定がどのアプリケーションに影響するか分析（例: VR設定変更 $\to$ DCR/DFL）。
3.  **配信/デプロイ:**
      * **DCR (VR/ルーティング/操作ルール):** CO (Composition Operator) へ指示を出し、該当VRのDCR PODの**再作成 (Rolling Update)** をトリガー。
      * **DFL/DPA (Peer/AM設定):** gRPCによる設定更新RPCを直接呼び出し、即時適用を試みる（可能な場合）。
