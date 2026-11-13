# 支援欄位與 value 寫法

[English (en-US)](field.md) | [繁體中文 (zh-TW)](field.zh-TW.md)

[Field/value support matrix (English)](FIELD-SUPPORT.md)

本表依目前 [欄位定義](src/fields.rs) 與 [JSON 驗證器](src/config.rs) 整理，包含 **53 個一般欄位、5 種 Options scope**。操作流程與 CLI 參數見 [README](README.zh-TW.md)。

多組 flow 可共用同一個 `template_id`，但 scope 與欄位結構必須一致；每組的值獨立展開，不做跨組配對，見 [共用 Template](README.zh-TW.md#多組資料共用-template)。

一般欄位放在 `flows[].fields`；每一項使用 `{"name":"欄位名稱","value":值}`。名稱不分大小寫，下表統一使用大寫。所有數值範圍都包含兩端，數字必須是 JSON 整數，IP／flags 名稱／RD 等文字必須加雙引號。

```json
{
  "source_id": 42,
  "flows": [{
    "template_id": 256,
    "fields": [
      {"name":"IPV4_SRC_ADDR","value":"192.0.2.1"},
      {"name":"IPV4_DST_ADDR","value":"198.51.100.2"},
      {"name":"PROTOCOL","value":6},
      {"name":"TCP_FLAGS","value":"SYN|ACK"},
      {"name":"IN_PKTS","value":10},
      {"name":"IN_BYTES","value":1000}
    ]
  }]
}
```

## 一般欄位

「陣列」是單一值的非空清單，例如 `[80,443]`；「range」是起訖物件；「prefix」只適用 IP；「random_between」與「random_file」表示每次發送逐筆重抽。詳細格式見下方各節。Type ID 與 Bytes 是 NetFlow v9 編碼，不是 JSON 字串長度。

| Field name | Type ID | Bytes | value 單一值格式／範圍 | 其他支援格式 | value 範例 |
|---|---:|---:|---|---|---|
| `IN_BYTES` | 1 | 4 | 整數 `0..=4294967295` | 陣列、range、random_between | `1000` |
| `IN_PKTS` | 2 | 4 | 整數 `0..=4294967295` | 陣列、range、random_between | `10` |
| `FLOWS` | 3 | 4 | 整數 `0..=4294967295` | 陣列、range | `1` |
| `PROTOCOL` | 4 | 1 | 整數 `0..=255` | 陣列、range | `6` |
| `SRC_TOS` | 5 | 1 | 整數 `0..=255` | 陣列、range | `0` |
| `TCP_FLAGS` | 6 | 1 | 整數 `0..=255`、flags 字串或十六進位字串 | 陣列、range | `"SYN\|ACK"` |
| `L4_SRC_PORT` | 7 | 2 | 整數 `0..=65535` | 陣列、range、random_between | `12345` |
| `IPV4_SRC_ADDR` | 8 | 4 | IPv4 位址字串 | 陣列、range、prefix、random_file | `"192.0.2.1"` |
| `SRC_MASK` | 9 | 1 | 整數 `0..=32` | 陣列、range | `24` |
| `INPUT_SNMP` | 10 | 4 | 整數 `0..=4294967295` | 陣列、range | `70001` |
| `L4_DST_PORT` | 11 | 2 | 整數 `0..=65535` | 陣列、range、random_between | `443` |
| `IPV4_DST_ADDR` | 12 | 4 | IPv4 位址字串 | 陣列、range、prefix、random_file | `"198.51.100.2"` |
| `DST_MASK` | 13 | 1 | 整數 `0..=32` | 陣列、range | `24` |
| `OUTPUT_SNMP` | 14 | 4 | 整數 `0..=4294967295` | 陣列、range | `70002` |
| `IPV4_NEXT_HOP` | 15 | 4 | IPv4 位址字串 | 陣列、range、prefix、random_file | `"192.0.2.254"` |
| `SRC_AS` | 16 | 4 | 整數 `0..=4294967295` 或 ASN 字串 | 陣列、range | `"AS1.10"` |
| `DST_AS` | 17 | 4 | 整數 `0..=4294967295` 或 ASN 字串 | 陣列、range | `4200000000` |
| `BGP_IPV4_NEXT_HOP` | 18 | 4 | IPv4 位址字串 | 陣列、range、prefix、random_file | `"198.51.100.254"` |
| `LAST_SWITCHED` | 21 | 4 | 整數 `0..=4294967295`；exporter uptime 毫秒 | 陣列、range | `1000` |
| `FIRST_SWITCHED` | 22 | 4 | 整數 `0..=4294967295`；exporter uptime 毫秒 | 陣列、range | `0` |
| `OUT_BYTES` | 23 | 4 | 整數 `0..=4294967295` | 陣列、range | `3000` |
| `OUT_PKTS` | 24 | 4 | 整數 `0..=4294967295` | 陣列、range | `20` |
| `IPV6_SRC_ADDR` | 27 | 16 | IPv6 位址字串 | 陣列、range、prefix、random_file | `"2001:db8::1"` |
| `IPV6_DST_ADDR` | 28 | 16 | IPv6 位址字串 | 陣列、range、prefix、random_file | `"2001:db8:1::2"` |
| `IPV6_SRC_MASK` | 29 | 1 | 整數 `0..=128` | 陣列、range | `64` |
| `IPV6_DST_MASK` | 30 | 1 | 整數 `0..=128` | 陣列、range | `64` |
| `ICMP_TYPE` | 32 | 2 | 整數 `0..=65535`、type 名稱或 `{type,code}` | 陣列、range | `{"type":"echo-request","code":0}` |
| `SAMPLING_INTERVAL` | 34 | 4 | 整數 `0..=4294967295` | 陣列、range | `1` |
| `SAMPLING_ALGORITHM` | 35 | 1 | 整數 `0..=255` | 陣列、range | `1` |
| `MPLS_TOP_LABEL_TYPE` | 46 | 1 | 整數 `0..=255` | 陣列、range | `0` |
| `MPLS_TOP_LABEL_IP_ADDR` | 47 | 4 | IPv4 位址字串 | 陣列、range、prefix、random_file | `"192.0.2.254"` |
| `DST_TOS` | 55 | 1 | 整數 `0..=255` | 陣列、range | `0` |
| `IP_PROTOCOL_VERSION` | 60 | 1 | 整數 `4` 或 `6` | 陣列、range | `4` |
| `IPV6_NEXT_HOP` | 62 | 16 | IPv6 位址字串 | 陣列、range、prefix、random_file | `"2001:db8::ffff"` |
| `BGP_IPV6_NEXT_HOP` | 63 | 16 | IPv6 位址字串 | 陣列、range、prefix、random_file | `"2001:db8:1::ffff"` |
| `MPLS_LABEL_1` | 70 | 3 | 整數 label `0..=1048575` 或 `{label,tc,bos}` | 陣列、range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_2` | 71 | 3 | 整數 label `0..=1048575` 或 `{label,tc,bos}` | 陣列、range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_3` | 72 | 3 | 整數 label `0..=1048575` 或 `{label,tc,bos}` | 陣列、range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_4` | 73 | 3 | 整數 label `0..=1048575` 或 `{label,tc,bos}` | 陣列、range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_5` | 74 | 3 | 整數 label `0..=1048575` 或 `{label,tc,bos}` | 陣列、range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_6` | 75 | 3 | 整數 label `0..=1048575` 或 `{label,tc,bos}` | 陣列、range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_7` | 76 | 3 | 整數 label `0..=1048575` 或 `{label,tc,bos}` | 陣列、range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_8` | 77 | 3 | 整數 label `0..=1048575` 或 `{label,tc,bos}` | 陣列、range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_9` | 78 | 3 | 整數 label `0..=1048575` 或 `{label,tc,bos}` | 陣列、range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_LABEL_10` | 79 | 3 | 整數 label `0..=1048575` 或 `{label,tc,bos}` | 陣列、range | `{"label":16000,"tc":3,"bos":true}` |
| `MPLS_VPN_RD` | 90 | 8 | RD 字串，Type 0／1／2 | 陣列 | `"65000:100"` |
| `MPLS_TOP_LABEL_PREFIX_LEN` | 91 | 1 | 整數 `0..=128` | 陣列、range | `24` |
| `ICMP_TYPE_IPV6` | 139 | 2 | 整數 `0..=65535`、type 名稱或 `{type,code}` | 陣列、range | `{"type":128,"code":0}` |
| `MPLS_TOP_LABEL_IPV6_ADDR` | 140 | 16 | IPv6 位址字串 | 陣列、range、prefix、random_file | `"2001:db8:2::"` |
| `MPLS_TOP_LABEL_TTL` | 200 | 1 | 整數 `0..=255` | 陣列、range | `64` |
| `MPLS_LABEL_STACK_LENGTH` | 201 | 4 | 整數 `0..=4294967295`；原始 stack 的 bytes 數 | 陣列、range | `8` |
| `MPLS_LABEL_STACK_DEPTH` | 202 | 4 | 整數 `0..=4294967295`；stack 層數 | 陣列、range | `2` |
| `MPLS_TOP_LABEL_EXP` | 203 | 1 | 整數 `0..=7` | 陣列、range | `3` |

## Options scope

以下名稱只放在 `flows[].scope`，不能當成一般 `fields`。scope 與一般欄位使用不同的 Type ID 命名空間；非空 scope 會建立 Options Template。各 scope 都接受整數、非空整數陣列及 range。

| Scope name | Type ID | Bytes | value 範圍 | value 範例 |
|---|---:|---:|---|---|
| `SYSTEM` | 1 | 4 | 整數 `0..=4294967295` | `1` |
| `INTERFACE` | 2 | 4 | 整數 `0..=4294967295`；ifIndex | `70001` |
| `LINE_CARD` | 3 | 4 | 整數 `0..=4294967295` | `2` |
| `NETFLOW_CACHE` | 4 | 4 | 整數 `0..=4294967295` | `1` |
| `TEMPLATE` | 5 | 2 | 整數 `0..=65535` | `256` |

scope 的 `TEMPLATE` 是欄位值，其範圍與 flow 的 `template_id`（256～65535）不同。完整範例見 [options.json](examples/options.json)。

## 陣列、range 與 IP prefix

以下都是可直接加入 `fields` 的單一項目：

```json
{"name":"L4_DST_PORT","value":[80,443]}
```

```json
{"name":"L4_DST_PORT","value":{"start":10000,"end":10005,"step":2}}
```

range 的 step 預設 1，必須是正整數，CLI 最大為 `18446744073709551615`；上述例子產生 10000、10002、10004。起訖值使用該欄位可接受的單一值；MPLS label range 端點只能是 label 整數，RD 不支援 range，IP_PROTOCOL_VERSION 的每個展開值都必須為 4 或 6。

```json
{"name":"IPV4_SRC_ADDR","value":"192.0.2.0/24"}
```

```json
{"name":"IPV4_DST_ADDR","value":{"prefix":"198.51.100.0/24","hosts_only":true}}
```

```json
{"name":"IPV6_SRC_ADDR","value":"2001:db8::/126"}
```

IPv4 prefix 預設包含網段首尾，`/24` 為 256 個位址；hosts_only 排除首尾，但 `/31`、`/32` 保留全部位址。IPv6 `/126` 為 4 個位址，不接受 hosts_only=true。Prefix 會歸零 host bits，不會新增 mask 欄位。

各欄位多值採笛卡兒積，不逐項配對。來源 `/24` × 目的 `/24` 為 65,536 筆；每個來源都配上全部目的位址。scope 也參與相乘，所有 flows 合計不得超過 **500,000 筆**。Prefix／range／random 物件放在 value 最外層，不在陣列裡巢狀展開。

## 隨機值

只有 `L4_SRC_PORT`、`L4_DST_PORT`、`IN_PKTS`、`IN_BYTES` 支援 random_between，範圍遵守各欄位整數上限：

```json
{"name":"IN_PKTS","value":{"random_between":[10,100]}}
```

上下限都包含、min≤max，每次發送逐筆重抽，允許重複；每欄只算一個設定值。各欄位獨立抽樣，不保證 PKTS／BYTES 比例。OUT_PKTS／OUT_BYTES 不支援隨機值。隨機數值僅接受 `random_between`，舊的 `random`／`min`／`max` 物件會被拒絕。

所有 IP 欄位都支援 `random_file`，每次發送逐筆從文字檔抽一個位址：

```json
{"name":"IPV4_SRC_ADDR","value":{"random_file":"random-ips.txt"}}
```

每行一個位址，空行與 `#` 開頭的行會略過。每行必須是純 IP 位址；CIDR prefix 會被拒絕，並列出檔案路徑與行號。位址版本須與欄位相符。相對路徑以 JSON 檔所在資料夾為準。檔案上限 8 MiB，只算一個設定值。

## TCP flags

以下 value 等值，都表示 SYN＋ACK：

| value | 格式 |
|---|---|
| `18` | JSON 整數 |
| `"0x12"` | 十六進位字串 |
| `"SA"` | 短字母組合 |
| `"SYN\|ACK"` | 以 `\|` 分隔完整名稱 |
| `"syn \| ack"` | 大小寫與分隔符旁的空白皆可 |

| 短字母 | 完整名稱 | Bit 值 |
|---|---|---:|
| F | FIN | 1 |
| S | SYN | 2 |
| R | RST | 4 |
| P | PSH | 8 |
| A | ACK | 16 |
| U | URG | 32 |
| E | ECE | 64 |
| C | CWR | 128 |

重複旗標以 OR 合併。完整名稱不要接成 `"SYNACK"`；空字串、空的分隔項（如 `"SYN|"`）及十進位字串 `"18"` 不接受。

## ASN、ICMP 與 MPLS

ASN 的點與 RD 的冒號前後可有空白，例如 `"AS1 . 10"`、`"65000 : 100"`。

| 類型 | 可用 value 範例 | 限制 |
|---|---|---|
| ASN | `65000`、`"AS65000"`、`"1.10"`、`"AS1.10"` | 0～4294967295；asdot 的兩部分各 0～65535，1.10＝65546 |
| ICMPv4 | `2048`、`"echo-request"`、`{"type":8,"code":0}` | 裸整數是 type×256＋code；物件 type／code 各 0～255，code 預設 0 |
| ICMPv6 | `32768`、`"echo-request"`、`{"type":128,"code":0}` | 使用 ICMP_TYPE_IPV6；不自動設定 PROTOCOL |
| MPLS label | `16000`、`{"label":16000,"tc":3,"bos":true}` | label 0～1048575、tc 0～7、bos 為布林值；tc 預設 0、bos 預設 false |
| RD Type 0 | `"65000:100"`、`"0:65000:100"` | 16-bit ASN＋32-bit 編號 |
| RD Type 1 | `"192.0.2.1:100"`、`"1:192.0.2.1:100"` | IPv4＋16-bit 編號 |
| RD Type 2 | `"4200000000:100"`、`"2:65000:100"` | 32-bit ASN＋16-bit 編號 |

| ICMP type 名稱 | IPv4 | IPv6 |
|---|---:|---:|
| echo-request | 8 | 128 |
| echo-reply | 0 | 129 |
| destination-unreachable | 3 | 1 |
| time-exceeded | 11 | 3 |
| parameter-problem | 12 | 4 |
| packet-too-big | 不接受 | 2 |

ICMP 的 type 可用整數或以上名稱。IPv4 ICMP 設 PROTOCOL＝1，IPv6 ICMP 設 PROTOCOL＝58；CLI 的 ICMP range 也會經過單一值編碼，HTML 只接受合併整數端點。

MPLS label 與 stack metadata 必須明列於 `fields`，不再接受 flow 層級的 `mpls_stack`。MPLS label 以 3 bytes 匯出，不含 TTL；手動欄位不會自動補 BoS 或 stack metadata。原始 stack 每層為 4 bytes，所以兩層的 MPLS_LABEL_STACK_LENGTH 為 8。RD 固定 8 bytes，其 ASN 部分可用 AS 前綴／asdot；未指定 type 時，IPv4 管理者選 Type 1，ASN≤65535 選 Type 0，其餘選 Type 2。詳見 [MPLS 設定](field.zh-TW.md#asnicmp-與-mpls)。

## 計數器與 Collector 每分鐘預期平均速率

`IN_PKTS`／`OUT_PKTS` 是紀錄內的輸入／輸出封包數，`IN_BYTES`／`OUT_BYTES` 是位元組數；它們不會在每次發送時自動累加。bps 依 BYTES × 8 計算，pps 依 PKTS 計算；以每分鐘彙總量除以 60 秒表示平均速率：

```sh
./target/release/nfv9gen preview examples/shared-template.json --kind data --rate 20
```

預設 rate 為 20，範例中兩個 flow 共用 Template 256，合併後預期為：

```text
Template         IN bps         IN pps        OUT bps        OUT pps
     256      320000.00         300.00              -              -
```

每個一般 Template 在表格中占一列，IN／OUT bps、pps（頻寬優先） 並排顯示；不同 ID 分開、相同 ID 合併。rate 是整份設定共用的 Data 包/s；各組依「該 Template 每輪計數合計 × rate ÷ 完整設定每輪 Data 包數」換算，BYTES 再乘 8 得到 bps。Options 不估算流量，但其 Data 包會占用分母。隨機計數器使用上下限平均值。

同一 Template 缺少任何一般 flow 的 PKTS／BYTES 時，只將該方向的該欄顯示為 `-`，不當成 0，也不影響其他 Template。`preview` 的 limit／template-id 只限制封包預覽，每分鐘預期仍涵蓋完整設定；`send` 在發送前顯示同樣分組結果，發送中的定期統計不重算每分鐘預期。

以上假設持續循環發送、全數接收、按接收時間彙總且不套用取樣倍率；短程發送仍顯示長期平均換算值。這不是實際 UDP 傳輸量，也不是用 LAST_SWITCHED−FIRST_SWITCHED 期間換算的原始流量速率。FIRST_SWITCHED／LAST_SWITCHED 是設定的 uptime 毫秒值，重送時不自動更新；兩者相等時的紀錄期間為 0，與 CLI 的 `--duration 0` 無限發送不同。參數、日誌與退出碼以 [README](README.zh-TW.md#持續發送與日誌) 為準。

## JSON 與 HTML 的差異

JSON 的四個計數器都保存實際值，不支援公式。HTML 可讓 IN_BYTES／OUT_BYTES 以「同方向 PKTS × 平均封包大小」計算，但只支援單一 PKTS，且匯出後是一般 BYTES 整數；連動設定僅隨草稿保留，見 [IN／OUT 計數器](field.zh-TW.md#計數器與-collector-每分鐘預期平均速率)。

本文件列的是 CLI JSON 接受的格式；HTML 不支援匯入 `random_file`、含名稱／物件的 ICMP range 或超過 JavaScript 安全整數的值。一般欄位值不會改變實際 UDP 的來源／目的 IP、port 或發送網卡；這些由 CLI 參數決定。

## IPFIX 專用欄位

加上 `--ipfix` 時，CLI 會送出 IPFIX 而非 NetFlow v9，`scope` 改用一般欄位名稱（不再是上述五個 Options scope），並多接受四個欄位。未加旗標時這些欄位會驗證失敗。HTML 產生器不提供這些欄位。

| 欄位 | ID | 長度 | Value |
|---|---:|---:|---|
| `DIRECTION` | 61 | 1 | 整數；0 = ingress、1 = egress。可用陣列與 range |
| `FLOW_EXPORTER` | 144 | 4 | 整數 0..=4294967295。可用陣列與 range |
| `DATALINK_FRAME_SIZE` | 312 | 2 | 整數 0..=65535。可用陣列與 range |
| `DATALINK_FRAME_SECTION` | 315 | 變動 | 十六進位字串、等長十六進位字串的陣列，或 `frame` 物件 |

`DATALINK_FRAME_SECTION` 不能當 scope 欄位。同一個 flow 內每筆片段長度必須相同；需要不同長度時另開一個 flow（可共用 `template_id`）。`frame` 物件與各 key 的說明見 [IPFIX 輸出](README.zh-TW.md#ipfix-輸出)。
