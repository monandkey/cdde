
## Rust クレート構成案 (Module & Dependency Design)

CDDEのようなマイクロサービスアーキテクチャでは、共通のロジックやプロトコル定義をクレート（ライブラリ）として分離することで、コードの再利用性、保守性、およびコンパイル速度を向上させます。

### A. Coreクレート (共通機能)

| クレート名 | 責務 | 使用技術/データ | 利用元 |
| :--- | :--- | :--- | :--- |
| `cdde-proto` | **プロトコル定義** | gRPC (`tonic`)、Protobuf (`prost`) | 全サービス (DFL, DCR, DPA, CMS) |
| `cdde-core` | **基盤機能** | **SCTP/Diameterソケット抽象化**, エラー定義 (`thiserror`) | DFL, DPA |
| `cdde-diameter-dict` | **Diameter辞書** | **標準AVP/コマンド定義**、パーシングロジック | DCR (ロジック), DFL (ヘッダ) |
| `cdde-dsl-engine` | **操作ルール実行** | **JSON DSLスキーマ**、ルールエンジン (`serde`, `regex`) | DCR |

### B. アプリケーションクレート (実行可能バイナリ)

| クレート名 | 責務 | 利用クレート | 備考 |
| :--- | :--- | :--- | :--- |
| `cdde-dfl` | DFLのメインロジック | `cdde-core`, `cdde-proto`, `cdde-diameter-dict` | **セッション管理** (`DashMap`, `DelayQueue`) を実装 |
| `cdde-dcr` | DCRのメインロジック | `cdde-proto`, `cdde-dsl-engine`, `cdde-diameter-dict` | VRFごとのPODとしてビルド |
| `cdde-dpa` | DPAのメインロジック | `cdde-core`, `cdde-proto` | Alive Monitoringの責務 |
| `cdde-cms` | CMSのメインロジック | `cdde-proto`, `sqlx` (PostgreSQL接続) | gRPC/REST APIの提供 |

-----
