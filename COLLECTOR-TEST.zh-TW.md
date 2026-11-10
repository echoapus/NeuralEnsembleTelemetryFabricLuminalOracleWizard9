# 全欄位 FlowSet：PC → nfcapd → nfdump

[English (en-US)](COLLECTOR-TEST.md) | [繁體中文 (zh-TW)](COLLECTOR-TEST.zh-TW.md)

設定檔：[examples/collector-all-fields.json](examples/collector-all-fields.json)。涵蓋 [field.md](field.zh-TW.md) 全部 **53 個一般欄位、5 種 Options scope**；操作與 CLI 參數見 [README](README.zh-TW.md)。Source ID 為 `424242`，共 14 個 Template；每個 datagram 放一個 FlowSet，包含普通 Template、Options Template 與對應 Data FlowSet。

以下以 PC 執行 Linux 或 Windows WSL2、collector 安裝 nfdump/nfcapd 為前提。這個專案目前建置 Linux executable；Windows 請在 WSL2 內建置、執行。紀錄內的 IPv4／IPv6 位址都是模擬資料，不必配置在 PC 上，也不會對這些位址建立連線。實際 UDP 封包只送到 `--target`。

此範例在 `fields` 明列 MPLS label 與 metadata。修改時保留欄位值與順序；程式不會自動推算 stack metadata 或 BoS。舊設定的調整方式見 [更新舊設定](README.zh-TW.md#更新舊設定)。

## 先啟動 collector

在 collector 執行，保留版本資訊：

```sh
nfcapd -V
nfdump -V
```

在新的專用目錄收集，避免與既有流量混在一起。下列使用 UDP 2055；若已有服務占用此 port，可換成 12055，PC 的 target 也要跟著改。collector 防火牆須允許 PC 到這個 UDP port。

```sh
NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR=$(mktemp -d "$HOME/NeuralEnsembleTelemetryFabricLuminalOracleWizard9-collector.XXXXXX")
printf '%s\n' "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR"
nfcapd -l "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -p 2055 -t 60 -E
```

保留這個終端機，接著去 PC 發送。這個指令在前景執行；`-E` 用於觀察解碼，較新版亦可使用 `-v 3`。nfdump 1.7 系列預設動態儲存支援的 extensions；若使用 1.6 系列，依本機 `nfcapd -h` 確認並加上 `-T all`，避免 Interface、ASN、MPLS 等額外資料未被保存。參數依據 [nfcapd 官方手冊](https://github.com/phaag/nfdump/blob/master/man/nfcapd.1)。

若使用既有 collector，請確認它允許此 PC 的來源 IP；來源白名單、強制 sampling 或 extension 限制都會影響結果。

## PC 驗證與發送

在專案根目錄執行：

```sh
cargo build --release --locked
./target/release/nfv9gen validate examples/collector-all-fields.json
./target/release/nfv9gen preview examples/collector-all-fields.json --kind data --limit 14 --rate 10
```

預期驗證摘要：

```text
Validation passed: Source ID=424242, 14 templates, 22 records, 14 data packets
```

### Collector 每分鐘預期平均速率與 Template

此份範例刻意使用 14 個不同的 template_id，方便逐組核對。一般設定可讓多個 flow 共用同一 ID，只要 scope、欄位名稱、順序與長度相同；Template 初始與每次刷新只送一份，各 flow 的值獨立展開，見 [shared-template.json](examples/shared-template.json)。不要只為減少 Template 包而合併不同結構。

`preview` 與 `send` 發送前會使用表格，每個一般 Template 一列，並排顯示 IN／OUT bps、pps（頻寬優先）；共用 ID 的 flow 合併計算。上方明確指定 rate=10，與下方發送命令一致；若省略 rate，兩個指令都使用 **20 包／秒**，是所有 Template 共用的 Data 發送速率。

這份設定每輪 14 包 Data，因此 rate=10 時 Template 1000 的理論估算約為 IN **5714.29 bps／7.14 pps**、OUT **11428.57 bps／14.29 pps**。其餘一般 Template 各列一組；缺少 OUT 計數器的欄位顯示 `-`，5 個 Options Template 則只列 ID 與 `Options`，不估算模擬流量。Options 的 Data 包仍占發送時間。

send 只保留簡短設定摘要與完整流量表，不逐欄列出隨機範圍或重複列出前幾組 Template 結構；需要細節時使用 `nfv9gen preview examples/collector-all-fields.json --template-id 1000 --kind data --hex --limit 1 --rate 10`。停止後顯示兩行摘要，包含累計時分秒、Data／Template 包數、紀錄數、UDP payload MiB 與平均速率。

上述估算假設持續循環發送、全數接收、按接收時間彙總且不套用取樣倍率；短程發送仍顯示長期平均換算值，不是實際 UDP 頻寬，也不是 collector 依紀錄期間、取樣倍率或聚合算出的速率。limit／template-id 只篩選封包預覽，不縮小估算範圍；發送中的週期統計不重算每分鐘預期。

### 發送十輪

把下列 target 換成實際 collector IP。請使用 `all` 模式，讓 Template 和 Data 在同一次執行中送出；不用 root 或 `--source`。

```sh
./target/release/nfv9gen send examples/collector-all-fields.json \
  --target 192.0.2.20:2055 \
  --kind all --rate 10 --count 140 --template-every 14
```

這是 **10 輪**：每輪 14 包 Data（其中有 5 包 Options Data）。每輪前刷新全部 Template；在上述速率下通常總共送出 140 包 Template、140 包 Data、220 筆紀錄。若因系統延遲觸發 30 秒刷新門檻，Template 包數可能增加。

220 筆包含 **170 筆一般流量、50 筆 Options 資料**。Options 用於描述取樣狀態，不應把它們算成 nfdump 一般流量列。沒有丟包、沒有過濾，而且全部一般 Template 都被 collector 接受時，未聚合的 nfdump 應有 170 筆一般流量。

傳輸層可以全部用 IPv4 UDP，仍同時測到紀錄內的 IPv6 欄位。若 collector 只接受 IPv6，可改用 `--target '[實際IPv6]:2055'`。

### 持續測試與日誌

十輪核對通過後，可另外使用持續發送；以下命令沒有固定輪數，不適用後面的「10 輪一般流量列」數量：

```sh
./target/release/nfv9gen send examples/collector-all-fields.json \
  --target 192.0.2.20:2055 --kind all --rate 20 \
  --duration 0 --template-every 14 >send.log 2>&1
```

只有精確的 `--duration 0` 啟用無限模式，直到 Ctrl-C／SIGTERM；不接受 `0.0`、`-0` 或 `1e-400` 等其他零／過小寫法。count 與 duration 互斥。手動停止退出 130；參數無效退出 2；傳輸或統計寫入失敗退出 1。

stdout 與 stderr 都是終端機時原地更新進度；重新導向時每 5 秒向 stderr 寫出實際發送的累積封包、紀錄、UDP payload bytes 與平均速率。大批量 burst 與初始 Template 發送期間也會檢查進度，回呼間隔至少 250 毫秒。達到 count 或 duration 上限後，不再執行逐包進度回報。可從另一個終端機執行 `tail -f send.log`。統計寫入／flush 失敗會停止發送並保留第一次錯誤；stderr 本身也失效時，訊息可能無法寫出，但不會因此 panic。這與 stdout 管線關閉可正常退出的行為不同。

## 停止收集，再讀取完成的檔案

PC 發送完成後，回到剛才 collector 的終端機按 Ctrl-C，等 nfcapd 結束，再執行下面的指令。不要讀取仍在寫入的 `nfcapd.current.*`。若改用另一個終端機，先將 `NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR` 設為剛才印出的目錄。

```sh
ls -lh "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR"
nfdump -R "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -N -6 -o raw
```

`-o raw` 顯示檔案中保留的 extensions，**不是原始 UDP 封包的逐欄位轉印**；`-N` 不縮寫數值，`-6` 保留完整 IPv6。不要加 `-a` 或 `-A` 聚合，否則重複送出的紀錄會合併。輸出與過濾用法見 [nfdump 官方手冊](https://github.com/phaag/nfdump/blob/master/man/nfdump.1)。

摘要表：

```sh
nfdump -R "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -N -6 \
  -o 'fmt:%sa %da %pr %sp %dp %in %out %sas %das %ipkt %ibyt %opkt %obyt'
```

欄位依序為來源／目的 IP、協定、來源／目的 port、輸入／輸出 ifIndex、來源／目的 ASN、輸入 packets／bytes、輸出 packets／bytes。老版本若不接受某個格式 token，先用 `-o raw`。

## 各組預期結果

以下數量以發送 10 輪計算。用紀錄內的來源 IP 區分組別，因為 nfdump 的檔案不一定保留原 Template ID。

| Template | 測試組 | 紀錄來源 IP | 每輪紀錄 | 10 輪一般流量列 |
|---|---|---|---:|---:|
| 1000 | IPv4 TCP：完整計數、ToS、mask、Interface、ASN、next-hop | `192.0.2.10` | 1 | 10 |
| 1001 | IPv4 UDP：`.0/32` 單一位址 | `1.1.1.0` | 1 | 10 |
| 1002 | IPv6 TCP：mask、Interface、ASN、兩種 IPv6 next-hop | `2001:db8:1002::10` | 1 | 10 |
| 1003 | ICMPv4：type/code `8/0`、`3/1` | `192.0.2.13` | 2 | 20 |
| 1004 | ICMPv6：type/code `128/0`、`2/0` | `2001:db8:1004::10` | 2 | 20 |
| 1005 | Interface：0、1、65535、65536、4294967295 | `192.0.2.15` | 5 | 50 |
| 1006 | MPLS label 1～10 | `192.0.2.16` | 1 | 10 |
| 1007 | IPv4 MPLS：RD 三種編碼、FEC、額外 metadata | `192.0.2.17` | 3 | 30 |
| 1008 | IPv6 MPLS：IPv6 FEC、額外 metadata | `2001:db8:1008::10` | 1 | 10 |
| 1100 | Options，scope `SYSTEM=1` | 無一般流量 IP | 1 | 不計入 |
| 1101 | Options，scope `INTERFACE=70001` | 無一般流量 IP | 1 | 不計入 |
| 1102 | Options，scope `LINE_CARD=2` | 無一般流量 IP | 1 | 不計入 |
| 1103 | Options，scope `NETFLOW_CACHE=1` | 無一般流量 IP | 1 | 不計入 |
| 1104 | Options，scope `TEMPLATE=1000` | 無一般流量 IP | 1 | 不計入 |

幾個可直接執行的檢查：

```sh
# .0/32：應保留 1.1.1.0，不是零筆，也不會展開成整個 /24
nfdump -R "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -N -o raw 'src ip 1.1.1.0'

# Interface：同一來源有五種 INPUT_SNMP，OUTPUT_SNMP 固定 70002
nfdump -R "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -N \
  -o 'fmt:%sa %in %out %ipkt %ibyt' 'src ip 192.0.2.15'

# 完整 IPv4 基準：逐筆 packets/bytes 應為 10/1000、20/2000
nfdump -R "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -N -o raw 'src ip 192.0.2.10'

# MPLS：第一層 16000、TC=3；第十層 16009、BoS=1
nfdump -R "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -N \
  -o 'fmt:%sa %mpls1 %mpls2 %mpls3 %mpls4 %mpls5 %mpls6 %mpls7 %mpls8 %mpls9 %mpls10' \
  'src ip 192.0.2.16'

# ICMPv4／v6：用 raw 觀察 type/code；不要要求 TCP/UDP port 意義
nfdump -R "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -N -6 -o raw 'proto 1 or proto 58'
```

1000 的 SRC_AS 應為 `65546`（`AS1.10` 的等值整數），DST_AS 為 `4200000000`；兩種 next-hop 分別為 `192.0.2.254`、`198.51.100.254`，INPUT/OUTPUT_SNMP 為 `70001/70002`。1002 的兩種 IPv6 next-hop 為 `2001:db8:1002::ffff`、`2001:db8:2002::ffff`，ASN 為 `65002/65556`。

所有一般流量的 FIRST_SWITCHED、LAST_SWITCHED 都為 0，表示 exporter 啟動時間點，紀錄自身的持續時間（LAST−FIRST）為 0 毫秒；這與 CLI 的 `--duration 0` 無限發送不同。這些值重送時不會自動更新，PKTS／BYTES 也不會累加。使用有效的相對 uptime 可避免剛啟動就填入未來時間。collector 顯示的絕對時間由封包時間與 uptime 換算，可能因秒級標頭精度有小幅差異。

## Options 與 MPLS 額外欄位怎麼判讀

五種 scope 分別建立獨立 Options Template，以測試 collector 接受的範圍；collector 可能只使用其中部分 scope。SAMPLING_INTERVAL 設為 **1**、SAMPLING_ALGORITHM 設為 **1**，避免此輪驗證的計數被取樣倍率放大。Options 不一定出現在 `nfdump -o raw`；先看 nfcapd 解碼訊息，並可針對某個完成的 `nfcapd.YYYY...` 檔案執行：

```sh
nfdump -E /實際路徑/nfcapd.YYYYMMddhhmm
```

這會查看該檔案可用的 exporter/sampler 資訊。不要僅以一般流量列中沒有 Options 欄位判定封包有問題。取樣倍率的影響見 [nfdump 官方說明](https://github.com/phaag/nfdump#sampling)。

MPLS label 1～10 與額外 metadata 分在不同 Template，便於定位問題。RD（90）、FEC prefix（91）、IPv6 FEC（140）、TTL/depth/length/EXP（200～203）等欄位是本工具依 IANA 定義放入 v9；**實際 nfdump 版本可能略過或不保存它們**。nfdump upstream 的 [NetFlow v9 欄位轉換表](https://github.com/phaag/nfdump/blob/master/src/netflow/netflow_v9.c) 可用來核對你安裝版本的實作；看到 label 不表示 RD 或 metadata 也有保存。

若要確認缺少的欄位是否真的存在於網路上，在 collector 另開終端機先抓包，再重新發送：

```sh
sudo tcpdump -ni any -s 0 -w /tmp/NeuralEnsembleTelemetryFabricLuminalOracleWizard9-all-fields.pcap 'udp dst port 2055'
```

發送完按 Ctrl-C，使用 Wireshark 開啟 pcap，必要時將 UDP 2055 Decode As 為 CFLOW/NetFlow，對照 Template ID、欄位 ID、長度及 Data 值。只有 nfdump 沒顯示，而 pcap 中有正確的欄位和值時，應記錄為該 collector 版本未保存／未顯示；若 nfcapd 報長度錯誤、丟棄 Data 或欄位解碼錯位，再對照 pcap 定位。

## 53 個一般欄位的覆蓋對照

以下是設定中的完整欄位 ID 與所在 Template；同一欄位可能出現在多組。

| 欄位 | ID | 長度（bytes） | Template |
|---|---:|---:|---|
| IN_BYTES | 1 | 4 | 1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008 |
| IN_PKTS | 2 | 4 | 1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008 |
| FLOWS | 3 | 4 | 1000 |
| PROTOCOL | 4 | 1 | 1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008 |
| SRC_TOS | 5 | 1 | 1000 |
| TCP_FLAGS | 6 | 1 | 1000, 1002, 1006, 1007, 1008 |
| L4_SRC_PORT | 7 | 2 | 1000, 1001, 1002, 1005, 1006, 1007, 1008 |
| IPV4_SRC_ADDR | 8 | 4 | 1000, 1001, 1003, 1005, 1006, 1007 |
| SRC_MASK | 9 | 1 | 1000, 1001 |
| INPUT_SNMP | 10 | 4 | 1000, 1001, 1002, 1005 |
| L4_DST_PORT | 11 | 2 | 1000, 1001, 1002, 1005, 1006, 1007, 1008 |
| IPV4_DST_ADDR | 12 | 4 | 1000, 1001, 1003, 1005, 1006, 1007 |
| DST_MASK | 13 | 1 | 1000, 1001 |
| OUTPUT_SNMP | 14 | 4 | 1000, 1001, 1002, 1005 |
| IPV4_NEXT_HOP | 15 | 4 | 1000 |
| SRC_AS | 16 | 4 | 1000, 1002 |
| DST_AS | 17 | 4 | 1000, 1002 |
| BGP_IPV4_NEXT_HOP | 18 | 4 | 1000 |
| LAST_SWITCHED | 21 | 4 | 1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008 |
| FIRST_SWITCHED | 22 | 4 | 1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008 |
| OUT_BYTES | 23 | 4 | 1000 |
| OUT_PKTS | 24 | 4 | 1000 |
| IPV6_SRC_ADDR | 27 | 16 | 1002, 1004, 1008 |
| IPV6_DST_ADDR | 28 | 16 | 1002, 1004, 1008 |
| IPV6_SRC_MASK | 29 | 1 | 1002 |
| IPV6_DST_MASK | 30 | 1 | 1002 |
| ICMP_TYPE | 32 | 2 | 1003 |
| SAMPLING_INTERVAL | 34 | 4 | 1100, 1101, 1102, 1103, 1104 |
| SAMPLING_ALGORITHM | 35 | 1 | 1100, 1101, 1102, 1103, 1104 |
| MPLS_TOP_LABEL_TYPE | 46 | 1 | 1007, 1008 |
| MPLS_TOP_LABEL_IP_ADDR | 47 | 4 | 1007 |
| DST_TOS | 55 | 1 | 1000 |
| IP_PROTOCOL_VERSION | 60 | 1 | 1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008 |
| IPV6_NEXT_HOP | 62 | 16 | 1002 |
| BGP_IPV6_NEXT_HOP | 63 | 16 | 1002 |
| MPLS_LABEL_1 | 70 | 3 | 1006, 1007, 1008 |
| MPLS_LABEL_2 | 71 | 3 | 1006 |
| MPLS_LABEL_3 | 72 | 3 | 1006 |
| MPLS_LABEL_4 | 73 | 3 | 1006 |
| MPLS_LABEL_5 | 74 | 3 | 1006 |
| MPLS_LABEL_6 | 75 | 3 | 1006 |
| MPLS_LABEL_7 | 76 | 3 | 1006 |
| MPLS_LABEL_8 | 77 | 3 | 1006 |
| MPLS_LABEL_9 | 78 | 3 | 1006 |
| MPLS_LABEL_10 | 79 | 3 | 1006 |
| MPLS_VPN_RD | 90 | 8 | 1007 |
| MPLS_TOP_LABEL_PREFIX_LEN | 91 | 1 | 1007, 1008 |
| MPLS_TOP_LABEL_IPV6_ADDR | 140 | 16 | 1008 |
| ICMP_TYPE_IPV6 | 139 | 2 | 1004 |
| MPLS_TOP_LABEL_TTL | 200 | 1 | 1007, 1008 |
| MPLS_LABEL_STACK_LENGTH | 201 | 4 | 1007, 1008 |
| MPLS_LABEL_STACK_DEPTH | 202 | 4 | 1007, 1008 |
| MPLS_TOP_LABEL_EXP | 203 | 1 | 1007, 1008 |

離線已檢查完整欄位覆蓋、設定驗證與預期發送計數；尚未在你的 PC／collector 上實際收送。請保留 nfdump/nfcapd 版本、發送摘要、raw 輸出與有問題時的 pcap，供比對。
