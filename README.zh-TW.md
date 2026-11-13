# NeuralEnsembleTelemetryFabricLuminalOracleWizard9

[English (en-US)](README.md) | [繁體中文 (zh-TW)](README.zh-TW.md)

`nfv9gen` 是用來測試 collector 的 NetFlow v9／IPFIX 流量產生器。用 JSON 描述流量，先離線檢查，再透過 UDP 發送。

- 53 個一般欄位、5 種 Options scope、IPv4／IPv6、ASN、ICMP、MPLS
- 陣列、range、IP prefix 與隨機值可展開成大量紀錄
- `validate` 與 `preview` 不需要 collector，也不會開 socket
- `--ipfix` 改送 IPFIX，包含取樣封包標頭（Junos inline monitoring）

僅支援 Linux（Windows 請用 WSL2）。不支援任意欄位 ID 或企業自訂 PEN。

<a id="build-and-quick-start"></a>

## 編譯與快速開始

從專案的 **Releases** 頁面下載 `nfv9gen-linux-x86_64.tar.gz`（Linux x86-64，glibc 2.35 以上），或從原始碼編譯：

```sh
bash build.sh          # 不要加 sudo；會補裝缺少的工具並編譯出 target/release/nfv9gen
```

接著檢查、預覽、發送：

```sh
nfv9gen validate examples/basic.json
nfv9gen preview examples/basic.json --hex
nfv9gen send examples/basic.json --target 192.0.2.20:7777
```

- `send` 需要 collector 正在監聽目標 UDP port，否則會以 `Connection refused` 停止。
- 預設只發送一個循環（先送全部 Template，再各送一次 Data 封包），速率 20 包／秒。要送更多用 `--count N` 或 `--duration 秒數`（`0` 表示持續到 Ctrl-C）。
- 發送成功只代表本機作業系統收下封包，是否收到要到 collector 確認。

也可以用 [config-builder.html](config-builder.html)（離線、僅支援 NetFlow v9）填表並下載 JSON。

<a id="configuration-format"></a>

## 設定格式

```json
{
  "source_id": 42,
  "flows": [
    {
      "template_id": 256,
      "fields": [
        {"name":"IPV4_SRC_ADDR","value":"192.0.2.1"},
        {"name":"IPV4_DST_ADDR","value":"198.51.100.2"},
        {"name":"PROTOCOL","value":6},
        {"name":"L4_SRC_PORT","value":12345},
        {"name":"L4_DST_PORT","value":443},
        {"name":"IN_PKTS","value":10},
        {"name":"IN_BYTES","value":1000}
      ]
    }
  ]
}
```

| 鍵 | 規則 |
|---|---|
| `source_id` | 可省略，預設 0。不是 IP 位址（IPFIX 時成為 observation domain ID） |
| `template_id` | `256..=65535`；`fields` 的順序就是 Template 的欄位順序 |
| `fields` | 每個欄位都要有 `name`（不分大小寫）與 `value` |
| `scope` | 可省略；非空時會產生 Options template（見 [options.json](examples/options.json)） |

未知的鍵或欄位會驗證失敗。上限：檔案 8 MiB，所有 flow 合計展開後 500,000 筆紀錄。

### value 寫法

| 寫法 | 意義 |
|---|---|
| `443` 或 `"192.0.2.1"` | 單一值 |
| `[80, 443]` | 多個值 |
| `{"start":"192.0.2.1","end":"192.0.2.3","step":1}` | Range（`step` 預設 1） |
| `"192.0.2.0/30"` 或 `{"prefix":"192.0.2.0/30","hosts_only":true}` | IP prefix |
| `{"random_between":[49152,65535]}` | 每次發送都重抽隨機值（L4 port、`IN_PKTS`、`IN_BYTES`） |
| `{"random_file":"ips.txt"}` | 每筆紀錄從檔案隨機取一個 IP，檔案每行一個位址 |

不同欄位的值會相乘：2 個來源 × 2 個 port = 4 筆紀錄，最後一個欄位變化最快。TCP flags 可寫 `"SYN|ACK"`、`"SA"`、`18` 或 `"0x12"`。全部欄位與可用的 value 格式見 [field.zh-TW.md](field.zh-TW.md)（MPLS、ASN、ICMP 與計數器也在那裡）。

### 多組資料共用 Template

`template_id` 相同的 flow 共用同一個 Template，可以把幾組值分開保留、不互相組合。它們的 scope、欄位名稱、順序與長度必須完全一致；每個 flow 各自展開。見 [shared-template.json](examples/shared-template.json)。

### 更新舊設定

- `{"random":true,"min":…,"max":…}` → `{"random_between":[min,max]}`。
- flow 層級的 `mpls_stack` → 在 `fields` 明列 `MPLS_LABEL_n` 與 metadata 欄位（見 [convenience.json](examples/convenience.json)）。
- `random_file` 清單只能放單一 IP；`198.51.100.0/24` 要改成 `198.51.100.0`。

## IPFIX 輸出

在 `validate`、`preview` 或 `send` 加上 `--ipfix`，就送 IPFIX（version 10）而不是 NetFlow v9。IPFIX collector 通常監聽 4739，要自己設定 `--target`。

```sh
nfv9gen send examples/ipfix-inline-monitoring.json --ipfix --target 192.0.2.10:4739
```

- `scope` 使用一般欄位名稱（例如 `FLOW_EXPORTER`），不是 v9 的 scope 名稱。
- `DIRECTION`（61）、`FLOW_EXPORTER`（144）、`DATALINK_FRAME_SIZE`（312）、`DATALINK_FRAME_SECTION`（315）只能搭配 `--ipfix`。
- 序號計算的是資料紀錄數，不是封包數。

`DATALINK_FRAME_SECTION` 放的是被取樣 Ethernet frame 的開頭幾個 bytes。可以直接給十六進位字串，或用 `frame` 物件自動產生標頭：

```json
{"name": "DATALINK_FRAME_SECTION", "value": {"frame": {
  "clip": 126, "length": 310, "protocol": 6,
  "src": {"start": "10.0.1.1", "end": "10.0.1.8"}, "dst": "198.51.100.50",
  "src_port": {"random_between": [1024, 65535]}, "dst_port": [80, 443],
  "tcp_flags": "SYN"
}}}
```

| 鍵 | 預設 | 意義 |
|---|---|---|
| `protocol`、`src`、`dst`、`src_port`、`dst_port` | 必填 | `protocol` 為 6（TCP）或 17（UDP）；位址與 port 可用上面的 value 寫法 |
| `clip` | 126 | 匯出的 bytes 數，從 Ethernet 標頭起算 |
| `length` | `clip` | 原始 frame 長度；`DATALINK_FRAME_SIZE` 要設成相同值 |
| `ipv6` | false | 產生 IPv6 標頭（`src`／`dst` 改為 IPv6） |
| `tcp_flags`、`tos` | SYN、0 | TCP flags（僅 TCP）、TOS（僅 IPv4） |

同一個 flow 內每段 frame 長度必須相同；要用不同的 `clip` 或 IP 版本，請另開一個 flow（可共用 `template_id`）。MAC 位址固定、TTL 為 64、標頭之後的 bytes 為 0，IPv4 標頭 checksum 一律正確。範例：[ipfix-inline-monitoring.json](examples/ipfix-inline-monitoring.json)。

## CLI

```text
nfv9gen validate CONFIG [--packet-size BYTES] [--ipfix]
nfv9gen preview  CONFIG [--packet-size BYTES] [--ipfix] [--kind all|templates|data] [--hex] [--limit N] [--template-id ID] [--rate N]
nfv9gen send     CONFIG [--target HOST:PORT] [OPTIONS]
```

所有指令都可加 `-s/--silent`（只輸出 `done` 或 `fail`）。`preview` 也會列出每個 Template 預期的 collector IN／OUT bps 與 pps。

| `send` 參數 | 預設 | 意義 |
|---|---|---|
| `--target HOST:PORT` | `127.0.0.1:2055` | Collector 位址（IPv6 如 `[::1]:2055`） |
| `--kind all\|templates\|data` | all | `all` 先送 Template 並定期刷新；`data` 需要 collector 已經有對應 Template |
| `--count N` | 一個循環 | 發送 N 包後停止（只有 `templates` 模式才計算 Template 封包） |
| `--duration 秒數` | — | 持續發送這麼久；`0` 表示直到 Ctrl-C／SIGTERM；不可與 `--count` 併用 |
| `--rate N` | 20 | 目標每秒封包數 |
| `--mode stream\|burst` | stream | `burst` 以 `--burst-size`（預設 64）為一批發送 |
| `--template-interval 秒`、`--template-every N` | 30、20 | 每隔幾秒或每發幾包 Data 就刷新 Template |
| `--packet-size BYTES` | 1400 | UDP payload 上限（64–65507）；`validate`／`preview` 也適用 |
| `--bind IP:PORT` | 由系統選擇 | 指定本機發送用的位址與 port |
| `--source IP:PORT` | — | 偽造 UDP 來源位址（raw socket，需 root 或 `CAP_NET_RAW`）；不可與 `--bind` 併用 |
| `--ipfix` | 關 | 送 IPFIX 而不是 NetFlow v9 |

`--bind` 與 `--source` 不會改動 JSON 紀錄內的位址。分開發送 Template 與 Data 時，請維持相同的 `source_id`、來源 IP:port 與 Template 定義。

### 持續發送與日誌

```sh
# 發送 100 包 Data
nfv9gen send examples/basic.json --target 192.0.2.20:7777 --rate 10 --count 100

# 持續到 Ctrl-C，每 5 秒把統計寫進日誌
nfv9gen send examples/basic.json --target 192.0.2.20:7777 --duration 0 >send.log 2>&1
```

在終端機執行時進度會原地更新；輸出被重新導向時，統計每 5 秒寫到 stderr（所以要加 `2>&1`）。完成或按 Ctrl-C 時會印出摘要。

退出碼：`0` 成功、`1` 設定或執行錯誤、`2` 參數錯誤、`130` 被中斷。

## 範例

| 檔案 | 內容 |
|---|---|
| [basic.json](examples/basic.json) | 一筆 IPv4 TCP 紀錄 |
| [shared-template.json](examples/shared-template.json) | 兩組值共用一個 Template |
| [ranges.json](examples/ranges.json) | IP range、陣列與組合 |
| [random-ports.json](examples/random-ports.json)、[random-counters.json](examples/random-counters.json)、[random-ips.json](examples/random-ips.json) | 每次發送重抽的隨機值 |
| [interfaces.json](examples/interfaces.json)、[nexthop.json](examples/nexthop.json) | Interface index 與 next-hop |
| [options.json](examples/options.json) | 多種 scope 的取樣 Options |
| [asn.json](examples/asn.json)、[mpls.json](examples/mpls.json)、[convenience.json](examples/convenience.json) | ASN 寫法、MPLS、ICMP |
| [collector-all-fields.json](examples/collector-all-fields.json) | 全部 53 個欄位與 5 種 scope，見 [COLLECTOR-TEST.zh-TW.md](COLLECTOR-TEST.zh-TW.md) |
| [ipfix-inline-monitoring.json](examples/ipfix-inline-monitoring.json) | IPFIX inline monitoring（需加 `--ipfix`） |

## 更多文件

- [field.zh-TW.md](field.zh-TW.md)：所有欄位、長度與可用的 value
- [FIELD-SUPPORT.md](FIELD-SUPPORT.md)：各欄位的 value 格式（英文）
- [COLLECTOR-TEST.zh-TW.md](COLLECTOR-TEST.zh-TW.md)：用 nfcapd／nfdump 做全欄位測試

## 授權

[PolyForm Noncommercial 1.0.0](LICENSE)。可免費用於學習、研究、個人與其他非商業用途，包含修改後的版本。不得商業使用。
