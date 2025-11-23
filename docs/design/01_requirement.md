# Product Name
---
- CDDE (Cloud Diameter Distribution Engine)

# Requirement
---
- Architecture
	1. Microservices
- Hostname
	1. OSのHostnameから取得
	2. Global/VirtualRouterごとにRealmを設定でき、Hostnameに使用できる
- Routing
	1. Realm-ID ベースでのルーティング
	2. DiameterのApplication-Commandベースでのルーティング
	3. Destination-Hostベースでのルーティング
- Session
	1. パケット単位でセッションIDを管理して、経過時間を管理できる
	2. 一定時間を経過後に送信ホストへResult-Code: 3002をセットして応答する
	3. 一定時間経過後に宛先ホストから受信したパケットは破棄する
- Routing Protocol
	1. S6a
	2. S13
	3. Gx
	4. Gy
	5. Gz
	6. SGd
	7. SBcAP
	8. Cx
	9. Sh
- Destination
	1. NodeのAddressとHost nameを登録管理できる
	2. NodeをPoolとして管理でき、ルーティングの対象に指定できる
	3. Round-Robinで転送先を設定できる
- Virtual Router 
	1. VirtualRouter毎に複数Peerを持てる
	2. VirtualRouter毎にHostanameを設定できる
	3. VirtualRouter毎にRealmを設定できる
	4. VirtualRouter毎にタイムアウトの時間をmsec単位で設定できる
	5. VirtualRouter毎にAlive Monitoringの設定を可能
	6. VirtualRouter毎にClient/Serverが設定可能
	7. Message単位でタイムアウトの時間を設定できる
- Alive Monitoring
	1. Peer毎に一定間隔でDWRを送信します
	2. Peer毎にDWR/Heartbeatの送信間隔を設定できる
	3. Peer毎にDWR/Heartbeatの再送回数を設定できる
	4. Peer毎にDWR/Heartbeatのタイムアウトを設定できる
	5. Peer毎にDWR/Heartbeatの再送回数、タイムアウトによりPeerの状態を判定
	6. PeerがUpしている状態でDWAを受信したら、Result-Codeに2001をセットして応答する
	7. PeerがUpしている状態では死活監視にDWRを優先的に使用、Downしている状態ではSCTP Heartbeatを使用
- Manipulation
	1. 特定の条件にマッチするAVPの値を書き換えることができる
- Topology Hidden
	1. ネットワーク内の情報を外部から隠蔽するためにHost-nameを書き換えできる
	2. Host-nameの書き換えを一元管理できる
- Performance Monitoring
	1. 受信しているパケット数をRx、Tx毎に1分単位でカウントできる
	2. Success rateをカウントできる
	3. Result-Code毎にカウントできる
	4. Virtual Router毎にカウントできる
	5. Peer毎にカウントできる
- Fault
	1. 各アプリケーションで定義された不正や状態異常を検知した場合にアラートとして記録、送信できる

# Design
## Application Design
---
- Core Application
	- Diameter Frontline (DFL)
		- 外部との始端、終端をする
		- 内部向けのロードバランサー
		- 死活監視以外のパケットを内部の適切なDCRへルーティングする
		- 死活監視パケットをDPAをルーティングする
		- DPAに登録されてないNode/HostからのパケットはRejectする
		- セッション管理の責務を持ち、Session毎にタイムアウトを管理する
		- Requestメッセージのタイムアウト後にResult-Code: 3002で応答する
		- DCRやDPAから受信したパケットの送信元アドレスをPeer向けに設定されているアドレスに変更する
		- PC、FIが収集するためのメタデータを生成する
	- Diameter Core Router (DCR)
		- Virtual Router毎にPODが生成される
		- 転送時にRoute-Record AVPを付与する
		- 次の転送先をルーティング及び、Route-Record AVPを元に判定する
		- Manipulationの責務を持つ
		- Topology Hiddenの責務を持つ
		- ルーティングやその他の処理が完了したらDFLへパケットを転送する
		- PC、FIが収集するためのメタデータを生成する
	- Diameter Peer Agent (DPA)
		- Peerの死活監視を行う
		- PeerのUp/Down状態を元にDFLへNode/Hostを登録する
			- Up状態のみ登録、Downは削除
		- PeerがDown時にVirtualRouterのRollがClientに限りSCTP INITやDiameter CERを送信する
		- PeerがDown状態になった場合にDPRを送信
		- パケットはDFLを中継される
		- PC、FIが収集するためのメタデータを生成する
- Assistance Application
	- Configuration Manager (CM)
		- 各Applicationに設定を共有、更新あればそれを適用するためにPODを再作成を依頼する
	- Composition Operator (CO)
		- CMからの要請でCore PODの再作成を管理する
	- Performance Collector (PC)
		- 一定間隔ごとに各PODから統計情報をPullする
		- 統計情報を一定期間保持
	- Fault Informer (FI)
		- 一定間隔ごとに各PODから状態やログをPullする
		- Pullした情報をもとにアラートを生成
		- アラートを一定期間保持
	- Configuration & Management Service (CMS)
		- 構成やConfig、統計情報、アラートなどの情報に一次元的に管理、接続可能
			- BFFな機能を提供する
- User Interface Application
	- User Interface (UI)
		- CSMへアクセスしてApplicationをグラフィカルに管理する機能をユーザへ提供する

## Detailed Design

