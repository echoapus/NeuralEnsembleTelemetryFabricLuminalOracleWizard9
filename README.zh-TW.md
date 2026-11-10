# NeuralEnsembleTelemetryFabricLuminalOracleWizard9

[English (en-US)](README.md) | [繁體中文 (zh-TW)](README.zh-TW.md)

**NeuralEnsembleTelemetryFabricLuminalOracleWizard9**（執行檔：`nfv9gen`）是以 Rust 實作的 NetFlow v9 流量產生器。讀取 JSON，先驗證設定、離線預覽，再透過 UDP 發送。支援 53 個一般欄位、5 種 Options scope、IPv4／IPv6、Interface、ASN、ICMP、MPLS、範圍展開，以及 L4 port／IN_PKTS／IN_BYTES 隨機值。

| 要做的事 | 文件或工具 |
|---|---|
| 第一次使用 | [編譯與快速開始](#編譯與快速開始) → [啟動 collector](#先讓-collector-接收) → [首次發送](#第一次發送與核對) |
| 在瀏覽器填表產生 JSON | [離線 HTML 產生器](config-builder.html) |
| 測試全部欄位 | [全欄位設定](examples/collector-all-fields.json)、[測試手冊](COLLECTOR-TEST.zh-TW.md) |
| 查欄位 ID、長度與輸入規則 | [設定格式](#設定格式)、[完整 field name／value 表](field.zh-TW.md) |
| 逐欄比較支援的 value 格式 | [Field/value support matrix（英文）](FIELD-SUPPORT.md) |
| 查發送參數、無限執行與日誌 | [CLI](#cli)、[持續發送與日誌](#持續發送與日誌) |
| 查 Collector 每分鐘預期平均速率 | [Collector 每分鐘預期平均速率](#collector-每分鐘預期平均速率) |
| 查組合數、RAM 與限制 | [紀錄數、封包數與設定上限](#紀錄數封包數與設定上限)、[記憶體用量](#記憶體用量) |
| 遇到接收或解碼問題 | [常見問題](#常見問題) |

目前針對 Linux 建置；Windows 可在 WSL2 內建置與執行。輸出固定為 **NetFlow v9**，不支援 IPFIX v10、任意欄位 ID 或企業自訂 PEN。部分額外欄位採用 IANA IPFIX registry 的定義，但封包版本仍為 v9，接收端需支援對應欄位。

<a id="build-and-quick-start"></a>

CLI 與 HTML 介面使用美式英文；兩種語言文件均保留實際英文輸出與按鈕名稱。

## 編譯與快速開始

### 下載已編譯執行檔

開啟本專案的 **Releases** 頁面，下載 `nfv9gen-linux-x86_64.tar.gz` 與對應的 `.sha256` 檔案。執行檔適用於 Linux x86-64、glibc 2.35 以上（Ubuntu 22.04 以上，包含 WSL2）。其他系統請依下方步驟自行編譯。

```bash
sha256sum -c nfv9gen-linux-x86_64.tar.gz.sha256
tar -xzf nfv9gen-linux-x86_64.tar.gz
cd nfv9gen-linux-x86_64
./nfv9gen --version
./nfv9gen validate examples/basic.json
```

使用下載的執行檔時，請將下方指令的 `./target/release/nfv9gen` 替換成 `./nfv9gen`。

### 自動編譯與發佈

每次推送至 `main`，GitHub Actions 都會執行測試與編譯。也可在 **Actions → Build and release → Run workflow** 選擇 `main` 手動編譯，不會發佈版本。完成後可在該次執行的 artifacts 下載壓縮檔與校驗碼。

發佈版本前，先更新 `Cargo.toml`、`Cargo.lock`，提交並推送變更，再推送相符的版本標籤：

```bash
git tag v0.1.0
git push origin v0.1.0
```

每個版本使用新標籤。測試與編譯成功後，工作流程會建立 GitHub Release，附上執行檔壓縮檔與校驗碼。使用 GitHub 自動提供的 token，不需設定個人存取權杖或 repository secret。

### 自動編譯（建議）

在 GitHub 選擇 **Code → Download ZIP**，下載並解壓縮本專案，再於包含 `build.sh` 的資料夾開啟終端機，執行：

```sh
bash build.sh
```

腳本會辨識 Linux 與 CPU 架構，檢查 C 編譯器／連結器，並在 Ubuntu／Debian、Fedora／RHEL 相容系統、openSUSE、Arch 衍生系統安裝缺少的系統工具。現有 Rust 若符合 Cargo.toml 的最低需求，就直接沿用；否則透過 [官方 rustup](https://rust-lang.github.io/rustup/concepts/profiles.html) 安裝精簡的穩定版工具鏈。接著使用鎖定的依賴版本編譯，檢查版本、基本設定與離線封包預覽，不會發送封包。

請**不要加 sudo** 執行腳本。只有缺少系統套件時，腳本才會使用 sudo；安裝套件與首次下載 Rust／依賴需要網路。腳本會自動接受套件與 Rust 的安裝提示；sudo 仍可能要求輸入登入密碼。程式不會安裝到系統目錄；執行檔位於 `target/release/nfv9gen`，完成後會顯示其絕對路徑。

更新原始碼後可再次執行 `bash build.sh`，沿用既有工具與編譯快取。不支援自動安裝的發行版，請先手動安裝 C 編譯器／連結器、curl 與 CA 憑證，腳本就能繼續處理 Rust 與編譯。Arch 使用者請先確認系統與套件資料庫已更新；腳本不會執行整套系統升級。任何步驟失敗都會停止並顯示錯誤，請修正後再執行。

### 手動編譯

若想自行安裝工具，請依下列步驟操作。

以下步驟以 **Ubuntu 或 Debian Linux** 為例，也適用於 WSL2 裡的 Ubuntu。安裝工具與第一次編譯需要網路；安裝系統套件時，帳號需要能使用 `sudo`。其他 Linux 發行版請先安裝該發行版的 C 編譯器、連結器與 curl，再執行 Rust 安裝及編譯步驟。

請在**終端機（Terminal）**中依序執行，每次複製一個指令區塊，不要複製預期輸出。`sudo` 要求輸入登入密碼時，畫面不會顯示字元；輸入完按 Enter 即可。只有下方安裝系統套件的指令需要 `sudo`，編譯與執行 NeuralEnsembleTelemetryFabricLuminalOracleWizard9 請使用一般使用者帳號。

### 1. 安裝 Linux 編譯工具

```sh
sudo apt update
sudo apt install -y build-essential curl ca-certificates
```

`build-essential` 提供產生執行檔所需的編譯器與連結器；curl 用來下載 Rust 安裝程式。發行版相關背景請參考 [Ubuntu 的 Rust 設定指南](https://ubuntu.com/developers/docs/howto/rust-setup/)。

### 2. 安裝 Rust 與 Cargo

如果 `rustc --version` 與 `cargo --version` 已能正常執行，而且 Rust 版本夠新，可直接進入步驟 3。專案宣告的最低需求是 Rust 1.85；建議使用最新穩定版，以符合鎖定的依賴套件需求。

依 [Rust 官方安裝說明](https://doc.rust-lang.org/book/ch01-01-installation.html)安裝：

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

安裝程式出現選項時，按 Enter 採用預設安裝。結束後，在目前終端機載入環境設定，再確認兩個工具：

```sh
. "$HOME/.cargo/env"
rustc --version
cargo --version
```

兩個版本指令都應顯示版本號。Rust 負責編譯程式；Cargo 負責下載依賴與管理編譯流程。若先前透過 rustup 安裝、現在需要更新版本，可執行 `rustup update stable`。

### 3. 下載專案並開啟資料夾

1. 在 GitHub 開啟**本專案的儲存庫首頁**，在檔案清單上方選擇 **Code → Download ZIP**。操作可參考 [GitHub 下載說明](https://docs.github.com/en/repositories/working-with-files/using-files/downloading-source-code-archives)。
2. 在檔案管理員將 ZIP 解壓縮，再開啟解壓縮後的專案資料夾；不要直接在 ZIP 內操作。
3. 找到包含 `Cargo.toml` 的資料夾，在空白處按右鍵，選擇 **Open in Terminal／在終端機開啟**（若檔案管理員提供此功能）。若已有原始碼，直接使用既有的專案資料夾即可。

也可以在已開啟的終端機執行 `cd "/full/path/to/project"`，將引號內路徑替換成實際解壓縮後的資料夾。路徑包含空白時，請保留引號。

確認目前位置是專案根目錄：

```sh
pwd
ls Cargo.toml Cargo.lock src/main.rs examples/basic.json
```

`pwd` 會印出目前資料夾；`ls` 應列出四個路徑，且沒有錯誤。下方與本指南其餘指令，都請在這個資料夾執行。

### 4. 編譯執行檔

```sh
cargo build --release --locked
```

第一次會下載並編譯依賴套件，可能需要數分鐘。請等到顯示成功的 `Finished` 訊息，且終端機提示符號重新出現。`--release` 產生最佳化的執行檔；`--locked` 使用 `Cargo.lock` 記錄的依賴版本。

完成的程式位於專案資料夾中的 **`target/release/nfv9gen`**，不必執行 `cargo install` 或安裝到系統目錄。

### 5. 不連接 collector，先確認編譯成功

```sh
./target/release/nfv9gen --version
./target/release/nfv9gen validate examples/basic.json
./target/release/nfv9gen preview examples/basic.json --kind data --hex
```

`--version` 應顯示 `nfv9gen 0.1.0`。路徑前方的 `./` 表示執行目前位置下的檔案。驗證應顯示：

```text
Validation passed: Source ID=42, 1 templates, 1 records, 1 data packets
```

`validate` 與 `preview` 不建立 socket，也不需要 collector。兩個指令都成功結束，就表示程式已可使用。

### 6. 發送到你的 collector

範例 `192.0.2.20:7777` 是文件用位址，不是可直接使用的 collector。請換成實際 collector 的 IP／UDP port。先完成[啟動 collector](#先讓-collector-接收)，讓 listener、target 與抓包條件一致，再發送：

```sh
./target/release/nfv9gen send examples/basic.json \
  --target 192.0.2.20:7777
```

未指定 `--rate` 時預設為 **20 包／秒**，是整份設定共用的目標速率。未指定 `--count` 或 `--duration` 時，只送一輪：先送全部 Template，再將全部 Data 封包各送一次。持續發送可指定秒數，或用 `--duration 0` 無限執行直到 Ctrl-C／SIGTERM：

```sh
./target/release/nfv9gen send examples/basic.json \
  --target 192.0.2.20:7777 --rate 20 --duration 10
```

更新程式後重新執行 `cargo build --release --locked`。依賴已快取時可加 `--offline`。以下命令均以專案根目錄為工作目錄；若已將執行檔複製為 `./nfv9gen`，可替換命令中的路徑。

### 編譯步驟失敗時

| 訊息或情況 | 處理方式 |
|---|---|
| `cargo: command not found` 或 `rustc: command not found` | 安裝 Rust 後，在目前終端機執行 `. "$HOME/.cargo/env"`，或重新開啟終端機 |
| 連結器錯誤提到找不到 `cc` | 依步驟 1 安裝 `build-essential`，再重新編譯 |
| `could not find Cargo.toml` | 目前資料夾不對；回到解壓縮後、包含 Cargo.toml 的資料夾，再執行步驟 3 的 `ls` 檢查 |
| Rust 版本太舊，或依賴套件要求較新的 rustc | 若透過 rustup 安裝，執行 `rustup update stable` 與 `rustup default stable`，確認 `rustc --version`，再重新編譯 |
| 依賴下載失敗或網域名稱解析錯誤 | 檢查網路／代理伺服器後重試；只有所有需要的依賴都已快取，才能使用 `--offline` |
| `./target/release/nfv9gen: No such file or directory` | 確認編譯已成功，且目前仍在專案根目錄；執行 `ls -l target/release/nfv9gen` 檢查 |

## 先讓 collector 接收

<a id="先讓-collector-接收"></a>

先在 collector 主機確認目的 port 有程式監聽：

```sh
sudo ss -lunp 'sport = :7777'
```

遠端 IPv4 發送需要監聽 `192.0.2.20:7777` 或 `0.0.0.0:7777`；只綁 `127.0.0.1` 不足。另確認防火牆允許來自 PC 的 UDP 流量。若服務在容器內，還要確認對外發布的是正確的 UDP port。

若已有正式 collector，請從該系統設定啟用 NetFlow v9 的接收 port。若你是使用官方 nfdump 工具組，可用 nfcapd 開一個測試接收器；以下在 collector 的終端機執行：

```sh
NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR=$(mktemp -d /tmp/NeuralEnsembleTelemetryFabricLuminalOracleWizard9-collector.XXXXXX)
printf '%s\n' "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR"
nfcapd -l "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -b 0.0.0.0 -p 7777 -t 60 -E
```

這個 nfcapd 保持前景執行，另開終端機檢查 `ss`。不要在同一個 port 再啟動第二個接收程式。`-E` 是解碼觀察模式，較新版亦可使用 `-v 3`；若是 1.6 系列，依本機 `nfcapd -h` 確認並加上 `-T all`，保存額外欄位。[nfcapd 官方手冊](https://github.com/phaag/nfdump/blob/master/man/nfcapd.1)

若目的 UDP port 沒有接收程式，OS 可能回覆 ICMP Port Unreachable；發送端出現 `Connection refused` 後，NeuralEnsembleTelemetryFabricLuminalOracleWizard9 會直接停止。單獨啟動旁路抓包工具不會代替 UDP 接收器。

## 第一次發送與核對

### 發送設定與讀取結果

HTML 下載的 `NeuralEnsembleTelemetryFabricLuminalOracleWizard9.json` 和手寫 JSON 使用同一個 CLI。先以基本範例測通，再替換設定檔：

```sh
./target/release/nfv9gen send examples/basic.json \
  --target 192.0.2.20:7777
```

不加 count/duration 時，基本範例只送 **1 包 Template＋1 包 Data**，Data 有一筆紀錄。發送端完成只表示本機 OS 接受封包；請再從 collector 的接收日誌或查詢結果確認。

使用官方 nfcapd 收集時，發送完回到 nfcapd 終端機按 Ctrl-C，等它結束並寫完檔案，再於同一個終端機執行：

```sh
ls -lh "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR"
nfdump -R "$NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR" -N -6 -o raw
```

若換終端機，先將 NEURAL_ENSEMBLE_TELEMETRY_FABRIC_LUMINAL_ORACLE_WIZARD9_FLOW_DIR 設為剛才印出的目錄。不要讀取仍在寫入的 `nfcapd.current.*`；先不加聚合或時間過濾，避免掩蓋收到的資料。[nfdump 官方手冊](https://github.com/phaag/nfdump/blob/master/man/nfdump.1)

### 分清楚同名的 nfdump 工具

現場若使用 `./nfdump -i eth0 -a udp`，這是即時抓包解碼的介面；它和上述「nfcapd 寫檔，再由官方 nfdump 讀檔」的操作不同。請依該工具的 help 使用參數，不要直接混用 `-R`、`-o raw` 等命令。

驗證結果要分開記錄：

| 證據 | 可以確認的事情 |
|---|---|
| `ss` 顯示 UDP socket | 有程式監聽此 IP／port |
| 網卡抓到封包 | 封包抵達抓包點 |
| 抓包工具列出正確欄位 | 該工具能解碼這些欄位 |
| collector 接收計數／解析日誌 | 正式服務有接收並處理 |
| 寫檔、資料庫或報表可查到 | 資料實際保存或入庫 |

抓到完整封包，不能直接宣稱正式 collector 已接收並入庫。必要時在 collector 另開終端機抓包，先啟動抓包，再發送：

```sh
sudo tcpdump -ni any -nn \
  'host 192.0.2.10 and (udp port 7777 or icmp)'
```

來源 IP 請改為實際 PC 位址；若經過 NAT，collector 看到的位址可能不同。若要留下完整內容供 Wireshark 核對，可使用：

```sh
sudo tcpdump -ni any -s 0 -w /tmp/NeuralEnsembleTelemetryFabricLuminalOracleWizard9-test.pcap 'udp dst port 7777'
```

## HTML JSON 產生器

直接以現代瀏覽器開啟 [config-builder.html](config-builder.html)。頁面可離線使用，不需伺服器、帳號或網路；採莫蘭迪配色，資料不會上傳。

1. 已有 JSON 時，在頁面上方「Start here」選檔直接載入；或展開「Paste JSON content」，按「Load pasted JSON」。
2. 新設定從預設 IPv4 TCP 開始；下方可新增 IPv6、ICMP、Options、MPLS 或自訂 flow。
3. 調整 Source ID、Template ID 與欄位值，依需要切換單值、多值、range、prefix 或隨機模式。
4. 檢查右側預覽與欄位錯誤，狀態為「Ready to download」後下載 `NeuralEnsembleTelemetryFabricLuminalOracleWizard9.json`。
5. 用 `nfv9gen validate NeuralEnsembleTelemetryFabricLuminalOracleWizard9.json`、`nfv9gen preview NeuralEnsembleTelemetryFabricLuminalOracleWizard9.json` 檢查，再交給 `nfv9gen send` 發送。

| 功能 | 行為 |
|---|---|
| 範本 | IPv4／IPv6 TCP、ICMP、Options、MPLS、自訂欄位 |
| 欄位搜尋與分類 | 可搜尋英文說明、英文欄位名稱或 `interface` 等分類 |
| 值的模式 | 單值、陣列、range、IP prefix、Random between；IN_BYTES／OUT_BYTES 另有「Calculate from PKTS」 |
| 錯誤定位 | 欄位旁列出原因，可跳到第一個錯誤；有錯時禁止下載與複製 |
| 匯入 JSON | 頁面上方直接選檔載入，或展開貼上 JSON；先完整驗證，成功後才替換表單 |
| 複製與排序 | 「Duplicate flow」產生新 ID；「Duplicate with shared template」保留相同 ID；欄位及 scope 可上下移動 |
| 本機草稿 | 預設關閉；自行勾選後保存完整表單，包含未完成輸入，可還原或刪除 |
| 離頁提醒 | 未下載的修改會請求瀏覽器提醒；實際對話框由瀏覽器決定 |

`IN_BYTES`、`OUT_BYTES` 的連動操作與 JSON 寫法見 [IN／OUT 計數器](#inout-計數器)。

匯入不會略過未知內容。未知屬性、`random_file` IP 位址池、含名稱或物件的 ICMP range，以及無法精確表示的大整數會被拒絕，原表單保留；這些有效的 CLI 設定仍可用命令列操作。HTML 的 range step 上限為 `9007199254740991`，CLI 可讀取更大的 u64 step。ASN、RD 與 IP prefix 匯出前會正規化為等值的格式。

草稿只存在目前瀏覽器。每次開啟頁面需重新勾選自動儲存；取消勾選保留既有草稿，刪除草稿會同時關閉自動儲存。瀏覽器禁止儲存或空間不足時會提示失敗。下載後會清除離頁提醒，仍須確認檔案已儲存；複製 JSON 不會清除提醒。若剪貼簿操作受限，頁面會選取 JSON，讓你手動複製。

右側 Data 包數按預設 `--packet-size 1400` 計算。更改 packet-size 後以 CLI 結果為準。單獨提供 HTML 即可使用；頁尾的說明連結另需隨附 README。

<a id="configuration-format"></a>

## 設定格式

JSON 的結構鍵名固定，例如 `source_id`、`flows`、`template_id`、`fields`、`name`、`value`。**不支援 `n`／`v` 短鍵別名**。`name` 的欄位名稱不分大小寫，例如 `IPV4_SRC_ADDR`；未知屬性或欄位會在建立 socket 前報錯。

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
        {"name":"TCP_FLAGS","value":"SA"},
        {"name":"IN_PKTS","value":10},
        {"name":"IN_BYTES","value":1000}
      ]
    }
  ]
}
```

| 項目 | 規則 |
|---|---|
| `source_id` | 可省略，預設 0，範圍 `0..=4294967295`；不是來源 IP |
| `flows` | 至少一項；每項的值獨立展開，可共用同一個 Template |
| `template_id` | `256..=65535`；相同 ID 的 scope、欄位名稱、順序與長度必須一致 |
| `fields` | 有順序的欄位陣列；至少一項 |
| `scope` | 可省略；非空時建立 Options Template／Data，先於一般 fields 編碼 |
| `name`、`value` | 每個欄位都必須提供；scope 與 fields 各自拒絕重複名稱 |
| 檔案上限 | 8 MiB |
| 展開上限 | 所有 flows 合計最多 500,000 筆，包含 Options |

### 多組資料共用 Template

多個 flow 可使用相同 `template_id`，以保存各組欄位值的搭配，不限於 IP Pair，也適用 port、protocol、flags、計數器與 Options。共用 ID 的 flow 必須具有相同的 scope、欄位名稱、順序與長度；值與值的模式可以不同，結構衝突會拒絕整份設定。

每個 flow 仍獨立做笛卡兒積，彼此不交叉配對。相同 ID 的 Template 只建立一份，初始發送及每次刷新也只送一次。Data 按各 flow 分開分包，不會把不同 flow 的紀錄合併進同一個 Data 包。

範例 [shared-template.json](examples/shared-template.json) 使用一個 Template 保存兩組搭配：

| Flow | 來源 IP | 目的 IP | 目的 port | IN_PKTS | IN_BYTES |
|---|---|---|---:|---:|---:|
| 1 | 192.0.2.1 | 198.51.100.1 | 443 | 10 | 1000 |
| 2 | 192.0.2.2 | 198.51.100.2 | 80 | 20 | 3000 |

只產生表中的兩筆，不會產生來源 .1 配目的 .2 或其他跨組搭配：

```sh
./target/release/nfv9gen validate examples/shared-template.json
./target/release/nfv9gen preview examples/shared-template.json --template-id 256 --kind data
```

預期為 **1 個 Template、2 筆紀錄、2 包 Data**。兩個 flow 的每個欄位仍需各自填寫，不會繼承另一個 flow 的值。HTML 可按「Duplicate with shared template」建立下一組，再修改其值；若要調整結構，需同步修改共用者或改用新 ID。不同 ID 即使結構相同，也不會自動合併。

### 多筆值、IP prefix 與 range

下列是可放進 `fields` 的個別項目，不是完整設定檔：

```json
{"name":"L4_DST_PORT","value":[80,443]}
```

```json
{"name":"IPV4_SRC_ADDR","value":{"start":"192.0.2.1","end":"192.0.2.3","step":1}}
```

```json
{"name":"L4_DST_PORT","value":{"start":10000,"end":10005,"step":2}}
```

陣列不可為空。不同欄位的多值採笛卡兒積：2 個來源 × 2 個目的 × 2 個 port＝8 筆；最後一個欄位變化最快。range 的 `step` 預設 1，必須是正整數；最後一例產生 10000、10002、10004，不強制包含 end。CLI step 最大為 `18446744073709551615`。RD 不支援 range，MPLS label range 的端點只接受 label 整數，`IP_PROTOCOL_VERSION` range 只能產生 4 或 6。

IP 欄位另支援 CIDR：

```json
{"name":"IPV4_SRC_ADDR","value":"1.1.1.0/32"}
```

```json
{"name":"IPV4_SRC_ADDR","value":{"prefix":"192.0.2.0/30","hosts_only":true}}
```

`.0` 可使用；第一例只產生 `1.1.1.0`。Prefix 會先歸零 host bits，預設包含網段全部位址；`192.0.2.3/30` 產生 `.0`～`.3`。`hosts_only:true` 排除 IPv4 網段首尾，但 `/31` 保留兩個位址、`/32` 保留一個。IPv6 `/126` 產生四個位址，`/128` 一個，不接受 `hosts_only:true`。Prefix 不會自動加入 mask 欄位。

Prefix、range、random 物件放在 `value` 最外層；陣列用於離散值，不在陣列中巢狀展開這些物件。

### 紀錄數、封包數與設定上限

| 名稱 | 意義 |
|---|---|
| 設定紀錄數 | 各 flow 的 scope／fields 值數相乘，再將所有 flow 相加；最多 500,000 筆 |
| 每輪 Data 包數 | 紀錄按 packet-size 分包，一包可有多筆；包含 Options Data |
| `--count N` | 發送 N 包後停止；all/data 計 Data，templates 計 Template |
| `--rate N` | 每秒目標封包數；不是每秒紀錄數，也不是模擬流量 bytes |
| 累積紀錄數 | 成功送出的 Data／Options 紀錄總數；重送也計，不受設定展開上限限制 |

例如來源 `/24` × 目的 `/24` 會產生 `256 × 256 = 65,536` 筆：每個來源都會配上全部 256 個目的位址。兩邊都設定 `hosts_only:true` 時為 `254 × 254 = 64,516` 筆；再加 10 個 port 則超過 500,000 筆上限。單一 PKTS／BYTES 不增加組合數。

預設 all 的初始／刷新 Template 額外發送，不計入 count/rate。目前沒有按累積紀錄數停止的參數。random_between 每欄只算一個值，不增加設定展開數，也不保證唯一連線。

### Collector 每分鐘預期平均速率

`preview` 可在不建立 socket、不發送封包的情況下，粗估 collector 每分鐘彙總後的 IN／OUT 平均 bps、pps；`send` 發送前的計畫也會顯示同樣估算：

```sh
./target/release/nfv9gen preview examples/basic.json --kind data --rate 20
```

基本範例每輪一筆紀錄、一包 Data，IN_PKTS＝10、IN_BYTES＝1000，預期顯示：

```text
Template         IN bps         IN pps        OUT bps        OUT pps
     256      160000.00         200.00              -              -
```

計算方式為：

```text
各 Template 預期平均 bps = 該 Template 一輪 BYTES 合計 × 8 × rate ÷ 一輪全部 Data 包數
各 Template 預期平均 pps = 該 Template 一輪 PKTS 合計 × rate ÷ 一輪全部 Data 包數
```

每個一般 Template 在表格中占一列，並排顯示 IN／OUT bps、pps（頻寬優先）；相同 template_id 的多個 flow 合併加總，不同 ID 分開顯示。多值先依笛卡兒積加總。rate 是完整設定共用的發送速率，不是每個 Template 各自取得相同速率。Options 不算模擬流量，但其 Data 包會占用發送時間，所以分母包含 Options Data。隨機欄位使用 `(min + max) / 2` 的期望值，不保證每次實際值相同。

同一 Template 內任一一般 flow 缺少某個方向的 PKTS 或 BYTES，該 Template 的該欄就顯示 `-`，不影響其他 Template，不把缺少的值當作 0；明確填入 0 則計為零。只送 Template 時不列流量估算，Options Template 只列 ID 與 `Options`，不填流量數值。

每分鐘預期只在 `preview` 與 `send` 發送前顯示一次。發送中的週期統計不會重新計算每分鐘預期，而是顯示實際匯出的封包、紀錄與 UDP payload bytes；其中的 bytes 不是 IN_BYTES／OUT_BYTES。

這是 **collector 每分鐘彙總後的預期平均 bps、pps**，假設持續按目標 rate 循環發送、全部收到、按接收時間彙總且不套用取樣倍率；不是實際 UDP 傳輸量。這是長期平均換算，不保證每個 60 秒區間相同；未滿一分鐘的發送仍顯示相同換算值。collector 若依 FIRST_SWITCHED／LAST_SWITCHED 分配時間區間，結果可能不同。`--limit`、`--template-id` 只篩選預覽，不縮小估算範圍。短程發送、burst 瞬間、Template 成本及實際系統／網路速度都可能讓觀察結果不同；未指定 rate 時，preview 與 send 均使用 20。

### 記憶體用量

目前在發送前預組全部封包並保存在 RAM，循環發送時重用，不會按 `--count` 或 `--duration` 複製清單。500,000 筆是程式的展開保護門檻，不是 NetFlow 協定限制，也不是 50 MB 的 RAM 硬限制；欄位越多、每筆越長，用量越大。

連續位址／數值優先用 prefix／range，離散值才用陣列。prefix／range 保存起點與數量，不先建立逐值清單。陣列已改為連續 bytes 儲存，並在封包預組前釋放原始 JSON、開始發送前釋放用完的欄位資料；解析大型 JSON 陣列仍有額外成本。

先前記錄的環境中，release 版以 `validate` 預組 500,000 筆、單一 `IN_BYTES` 欄位，各量測五次的程序峰值 RSS：

| 值的寫法 | 峰值 RAM |
|---|---:|
| range `0..=499999` | 約 5 MiB |
| 列出 500,000 個整數的 JSON 陣列 | 約 22 MiB |

陣列優化前同案例約 48 MiB，優化後約減少 55%。這是特定設定的量測，並非所有設定或長時間發送的保證值。可在 Linux 用 `/usr/bin/time -v ./target/release/nfv9gen validate NeuralEnsembleTelemetryFabricLuminalOracleWizard9.json` 查看自己的峰值；目前沒有自動預估 RAM 或記憶體預算選項。

### IN／OUT 計數器

四個欄位 `IN_PKTS`、`IN_BYTES`、`OUT_PKTS`、`OUT_BYTES` 都是 4-byte 非負整數，範圍 `0..=4294967295`。它們描述紀錄內的流量，不是 NeuralEnsembleTelemetryFabricLuminalOracleWizard9 實際送出的 UDP 包數或大小，也不會隨循環發送自動累加。

HTML 可在 `IN_BYTES`／`OUT_BYTES` 的「值的模式」選「Calculate from PKTS」，輸入正整數平均封包大小（bytes / packet）。同一 flow 必須有對應方向的 PKTS，且使用單一值：

| 方向 | PKTS | 平均大小 | 計算後 BYTES |
|---|---:|---:|---:|
| IN | 10 | 100 bytes / packet | 1000 |
| OUT | 20 | 150 bytes / packet | 3000 |

修改 PKTS 或平均大小會即時重算；缺少 PKTS、使用多值／range／隨機 PKTS，或乘積超過上限時禁止下載。JSON 只保存算好的整數，沒有公式或平均大小屬性。以下四項放入同一 flow 的 `fields`：

```json
[
  {"name":"IN_PKTS","value":10},
  {"name":"IN_BYTES","value":1000},
  {"name":"OUT_PKTS","value":20},
  {"name":"OUT_BYTES","value":3000}
]
```

連動模式與平均大小僅隨本機草稿保留；重新匯入 JSON 後需自行啟用。直接手改 JSON 的 PKTS 時，也要自行更新 BYTES。`random_between` 目前只支援 IN 計數器與 L4 port，OUT 計數器不支援隨機模式。

### Interface 與 next-hop

`INPUT_SNMP`、`OUTPUT_SNMP` 是紀錄內的輸入／輸出 ifIndex，均為 4 bytes、`0..=4294967295`，不接受 `eth0` 這類名稱。它們不會指定真正發送網卡。Options 的 `scope.INTERFACE` 則表示選項適用的介面。

IPv4 next-hop 使用 `IPV4_NEXT_HOP`、`BGP_IPV4_NEXT_HOP`；IPv6 使用 `IPV6_NEXT_HOP`、`BGP_IPV6_NEXT_HOP`。可用單值、陣列、prefix 或 range，位址版本需符合欄位。這些位址不會更改 OS 發送路由。

例如把下列項目加入 `fields`，可測試 32-bit ifIndex、ASN 與 BGP next-hop：

```json
[
  {"name":"INPUT_SNMP","value":70001},
  {"name":"OUTPUT_SNMP","value":70002},
  {"name":"SRC_AS","value":"AS1.10"},
  {"name":"BGP_IPV4_NEXT_HOP","value":"192.0.2.254"}
]
```

`AS1.10` 等於 65546。ASN 的點與 RD 的冒號前後可有空白，例如 `"AS1 . 10"`、`"65000 : 100"` 都可接受。紀錄內的 IPv6 流量可透過 IPv4 UDP 傳送；JSON 位址與真正傳輸版本分開設定。

### 更新舊設定

舊設定語法已不再接受：

- 將 `{"random":true,"min":49152,"max":65535}` 改為 `{"random_between":[49152,65535]}`。上下限都必須明列；舊設定若省略上下限，請使用 `[1,65535]`。
- 將 flow 層級的 `mpls_stack` 改為 `fields` 中明確的 label 與 metadata 欄位。保留欄位順序、label 的 TC／BoS、stack depth、stack length 與 top-label EXP，範例見 [convenience.json](examples/convenience.json)。
- `random_file` 清單只接受純 IP。若要保留舊版的單一主機行為，將 `198.51.100.0/24` 改為 `198.51.100.0`；清單項目不會展開網段。

### 隨機 L4 port、封包數與 bytes

`L4_SRC_PORT`、`L4_DST_PORT`、`IN_PKTS`、`IN_BYTES` 可用：

```json
{"name":"L4_SRC_PORT","value":{"random_between":[49152,65535]}}
{"name":"IN_PKTS","value":{"random_between":[10,100]}}
{"name":"IN_BYTES","value":{"random_between":[1000,100000]}}
```

上下限都包含且 min≤max；L4 port 須為 `0..=65535`，IN_PKTS／IN_BYTES 須為 `0..=4294967295` 的整數。min=max 為固定值。每次發送逐筆重抽，重送同一包也重抽，允許重複。各欄位獨立抽樣，不會自動維持 packets 與 bytes 的比例；只改紀錄欄位，不改實際 UDP 的來源／目的 port。

`validate`、`preview` 不抽樣，preview hex 使用 min 占位。send 使用緩衝的 `/dev/urandom`，沒有固定 seed；逐筆抽樣有額外成本，不能套用固定封包的效能估計。

### 從檔案隨機取 IP

所有 IP 欄位都接受 `{"random_file":"PATH"}`，每次送出時每筆紀錄都會從檔案中隨機取一行：

```json
{"name":"IPV4_SRC_ADDR","value":{"random_file":"random-ips.txt"}}
```

每行一個位址，空行與 `#` 開頭的行會略過。每行必須是純 IP 位址；不接受 `198.51.100.0/24` 這類 CIDR prefix。位址版本必須與欄位相符。相對路徑以 JSON 檔所在資料夾為準。無效項目會列出檔案路徑與行號。檔案上限 8 MiB，只算一個設定值。preview hex 使用第一個位址。範例見 `examples/random-ips.json`。

### TCP flags、ASN 與 ICMP

| 類型 | 可用寫法與限制 |
|---|---|
| TCP flags | `"SA"`、`"SYN\|ACK"`、整數 `18`、`"0x12"` 等值；0～255 |
| 短 flags 字母 | F、S、R、P、A、U、E、C，依序為 FIN、SYN、RST、PSH、ACK、URG、ECE、CWR |
| 完整 flags 名稱 | 以 `\|` 分隔；`"SYNACK"`、空字串與十進位字串 `"18"` 不接受 |
| ASN | `65000`、`"AS65000"`、`"1.10"`、`"AS1.10"`；完整範圍 0～4294967295 |
| asdot | `high × 65536 + low`，兩部分各 0～65535；`"1.10"`＝65546，需用字串 |
| ASN 編碼 | SRC_AS、DST_AS 固定 4 bytes，大 ASN 不截斷為 16-bit |
| ICMP | IPv4 用 ICMP_TYPE，IPv6 用 ICMP_TYPE_IPV6，均為 2 bytes |
| ICMP 值 | `{"type":8,"code":0}` 或已合併整數 `2048`；code 可省略，預設 0 |
| ICMP 編碼 | `type × 256 + code`；type/code 各 0～255，不自動設定 PROTOCOL |

TCP flags 不分大小寫，重複旗標以 OR 合併；`"ECE"` 是 0x40，`"EC"` 是 ECE＋CWR＝0xc0。支援的 ICMP 名稱如下，可直接作為 value 或放在 type：

| ICMP 名稱 | IPv4 type | IPv6 type |
|---|---:|---:|
| echo-request | 8 | 128 |
| echo-reply | 0 | 129 |
| destination-unreachable | 3 | 1 |
| time-exceeded | 11 | 3 |
| parameter-problem | 12 | 4 |
| packet-too-big | 不適用 | 2 |

IPv4 ICMP 請設 `PROTOCOL` 為 1 並使用 `ICMP_TYPE`，IPv6 ICMP 請設 `PROTOCOL` 為 58 並使用 `ICMP_TYPE_IPV6`；移除不適用的 TCP flags 與 L4 port。兩者的 echo-request 分別編碼為 2048（8/0）與 32768（128/0）。

```json
{"name":"TCP_FLAGS","value":"SYN|ACK"}
```

`|` 是 flags 名稱的分隔符，大小寫與分隔符旁的空白皆可使用，例如 `"syn | ack"`；不要將完整名稱直接接成 `"SYNACK"`。

### Options

非空 scope 會建立 Options Template／Data。每個 scope 保有自己的值與順序，scope 與 fields 會一起參與組合展開。

| Scope | Type ID | Bytes |
|---|---:|---:|
| SYSTEM | 1 | 4 |
| INTERFACE | 2 | 4 |
| LINE_CARD | 3 | 4 |
| NETFLOW_CACHE | 4 | 4 |
| TEMPLATE | 5 | 2 |

以下是 sampling interval＝1 的完整設定：

```json
{
  "source_id": 42,
  "flows": [{
    "template_id": 300,
    "scope": [{"name":"SYSTEM","value":1}],
    "fields": [
      {"name":"SAMPLING_INTERVAL","value":1},
      {"name":"SAMPLING_ALGORITHM","value":1}
    ]
  }]
}
```

多 scope 範例見 [options.json](examples/options.json)，其中 interval＝100，collector 可能依倍率換算 packets／bytes；全欄位測試採 interval＝1，避免倍率影響。Options 通常不會列成一般流量資料列，各 scope 的實際支援取決於 collector。

### MPLS 設定

手動欄位使用實際 label 值，不要預先位移：

```json
{"name":"MPLS_LABEL_1","value":{"label":16000,"tc":3,"bos":false}}
```

Label 範圍 0～1048575，TC 為 0～7，BoS 表示 stack 底層。直接填整數時 TC＝0、BoS＝false。每個 label section 匯出 3 bytes，編碼為 `(label << 4) | (tc << 1) | bos`，不含 TTL；TTL 使用 `MPLS_TOP_LABEL_TTL`。手動欄位不會自動推算 BoS、depth、length 或 EXP。

在 `fields` 明確設定各層 `MPLS_LABEL_1`～`MPLS_LABEL_10` 與需要的 stack metadata，範例見 [convenience.json](examples/convenience.json)。只有最後一層設 BoS=true；stack 長度按原始 MPLS 每層 4 bytes 計算，不是匯出的 3-byte section 長度。

RD 固定 8 bytes，依 [RFC 4364 §4.2](https://www.rfc-editor.org/rfc/rfc4364.html#section-4.2) 編碼：

| RD 寫法 | 類型 | 範圍 |
|---|---|---|
| `"65000:100"` | Type 0 | 16-bit ASN＋32-bit 編號 |
| `"192.0.2.1:101"` | Type 1 | IPv4＋16-bit 編號 |
| `"4200000000:102"` | Type 2 | 32-bit ASN＋16-bit 編號 |
| `"2:65000:100"` | 明確指定類型 | type 可為 0、1、2，限制同上 |

ASN 部分支援 AS 前綴與 asdot；未指定 type 時，ASN≤65535 選 Type 0，較大者選 Type 2。RD 可用值陣列，不支援 range。額外 metadata 的 ID／定義對照 [IANA IPFIX registry](https://www.iana.org/assignments/ipfix)，collector 需能處理它們在 v9 Template 中的固定長度。

## CLI

```text
nfv9gen validate CONFIG [--packet-size BYTES]
nfv9gen preview CONFIG [--packet-size BYTES] [--kind all|templates|data] [--hex] [--limit N] [--template-id ID] [--rate N]
nfv9gen send CONFIG [OPTIONS]
# 所有指令都可加 -s/--silent：只輸出 done 或 fail
```

### 驗證與預覽

| 選項 | 預設 | 說明 |
|---|---|---|
| `--packet-size BYTES` | 1400 | 三個指令共用；64～65507，含 20-byte NetFlow header，不含 IP／UDP header |
| `--kind` | all | preview 專用：all、templates、data；all 先列 Template 再列 Data |
| `--hex` | 關閉 | preview 顯示完整 UDP payload hex |
| `--limit N` | 8 | 篩選後最多顯示 N 包，至少 1；不是紀錄數 |
| `--template-id ID` | 全部 | 只預覽此 ID，仍驗證完整設定；摘要與 Collector 每分鐘預期平均速率仍是完整設定總計 |
| `--rate N` | 20 | preview 的 Collector 每分鐘預期平均速率使用的目標 Data 包/s；不發送封包 |

預覽列出欄位長度、值數、第一筆及多值最後一筆值、展開算式。預覽使用目前 UNIX 時間、uptime＝0、sequence 從 0 開始，不沿用發送 session，也不讀取亂數或建立 socket。

### 發送

| 選項 | 預設 | 說明 |
|---|---|---|
| `--target host:port` | `127.0.0.1:2055` | collector 位址；IPv6 使用 `[::1]:2055`，port 不可為 0 |
| `--bind IP:port` | 系統選擇 | 本機來源位址與 port；IP 須已配置，port＝0 可由 OS 分配 |
| `--source IP:port` | 無 | 指定可非本機的來源，需 Linux raw socket 權限；與 bind 互斥 |
| `--kind all` | all | 初始送 Template，之後送 Data，並自動刷新 Template |
| `--kind templates` | — | 只送 Template；count/rate 以 Template 包計 |
| `--kind data` | — | 只送 Data；接收端須已有對應來源與定義的 Template |
| `--count N` | 所選種類一輪 | 至少 1；all/data 計 Data 包，自動 Template 額外計算 |
| `--duration SECONDS` | 無 | 只有明確寫成 `0` 才表示無限執行，直到 Ctrl-C／SIGTERM；正數可含小數；過小、負數及其他零寫法拒絕；與 count 互斥，含初始 Template 發送時間 |
| `--rate N` | 20 | 每秒目標封包數，1～1000000000 |
| `--mode stream` | stream | 逐包排程 |
| `--mode burst` | — | 分組發送，以組間隔控制平均目標速率 |
| `--burst-size N` | 64 | 至少 1，僅 burst 使用；實際組大小不超過 rate |
| `--template-interval SECONDS` | 30 | 僅 all；有限正數，可含小數 |
| `--template-every N` | 20 | 僅 all；至少 1，每 N 包 Data 後於下一包 Data 前刷新 |
| `--packet-size BYTES` | 1400 | UDP payload 上限；Template 描述及單筆紀錄都必須放得下 |

兩個刷新門檻任一達到，就在下一包 Data 前刷新全部 Template。每次執行 sequence 從 0 開始，成功送出每個 datagram 後遞增，Template／Data 共用並自然回繞。

發送前只列來源／目的、設定摘要、模式／速率／停止條件、UDP payload 範圍、Template 刷新規則與逐 Template 的流量表。欄位結構、首尾值及隨機範圍不在 send 重複列出，請用 `nfv9gen preview CONFIG --template-id ID` 查看。stdout 與 stderr 都連接終端機時，進度會原地更新；任一串流重新導向時，每 5 秒向 stderr 輸出一行統計（執行秒數、累積封包／紀錄／bytes，以及平均封包速率），不含終端機控制碼。可用 `--duration 0 >send.log 2>&1` 保存持續發送的紀錄。完成或 Ctrl-C／SIGTERM 中斷會輸出兩行摘要：累計時分秒、Data／Template 包數，以及紀錄數、UDP payload MiB、平均 Data 包/s；傳輸錯誤立即回報失敗，不輸出正常完成摘要。

Stream 高速率受系統排程精度限制；burst 可降低逐包等待成本，但不保證達到設定速率。設定與封包預先建立，不按 count 複製清單；random_between 與 random_file 欄位另在發送前改寫。FIRST_SWITCHED／LAST_SWITCHED 等紀錄欄位保持設定值，不會隨時間更新。

### 持續發送與日誌

```sh
# 發送 100 包 Data；Template 額外計算
./target/release/nfv9gen send examples/basic.json \
  --target 192.0.2.20:7777 --rate 10 --count 100

# 持續 10 秒；count 與 duration 不可同時使用
./target/release/nfv9gen send examples/basic.json \
  --target 192.0.2.20:7777 --rate 20 --duration 10

# 無限循環，直到 Ctrl-C／SIGTERM；每 5 秒將統計寫入日誌
./target/release/nfv9gen send examples/basic.json \
  --target 192.0.2.20:7777 --rate 20 --duration 0 \
  >send.log 2>&1

# 以 burst 分組發送，並調整 Template 刷新門檻
./target/release/nfv9gen send examples/ranges.json \
  --target 192.0.2.20:7777 --mode burst --burst-size 64 \
  --rate 10000 --count 10000 --template-every 100 --template-interval 5
```

從另一個終端機用 `tail -f send.log` 查看。統計中的紀錄數包含 Options 與重複發送的紀錄，bytes 為成功送出的 UDP payload 總量；封包速率是從啟動至今的平均，不是瞬間值。stdout 保存啟動資訊及最終摘要，週期統計寫到 stderr；只用 `>send.log` 不會保存 stderr，需加 `2>&1`。短於 5 秒的執行可能只有最終摘要。統計寫入或 flush 失敗（例如磁碟已滿、管線關閉）會停止發送並以退出碼 1 結束，不會當作正常完成或手動停止；若 stderr 本身無法寫入，錯誤訊息可能無法保存，但不會因輸出錯誤而 panic。

`--duration 0` 適用 all、data、templates；`--template-interval 0` 仍不接受。`0.0`、`-0`、`0e10`、`1e-400` 等寫法不會啟用無限模式，會作為無效參數拒絕（退出 2）；有限正數若小到無法表示為有效 Duration，也會拒絕。手動停止退出碼為 130，傳輸或統計輸出錯誤會停止並退出 1。要確認現場行為，先在實際 collector 環境驗證接收與停止訊號。

| 發送計畫項目 | 判讀方式 |
|---|---|
| 每輪 Data 包數 | 循環清單長度，不是每輪紀錄數 |
| 來源 → 目的 | 真正的 UDP IP:port，與 JSON 模擬位址分開 |
| 自動 Template | all 模式額外發送，不算入 count/rate |
| 每包紀錄數與大小 | 依 packet-size 分包；packet-size 計 UDP payload |
| Collector 每分鐘預期平均速率表 | 每個 Template 一列；IN／OUT bps、pps（頻寬優先） 並排，缺少計數器以 - 表示 |
| 結構與隨機範圍 | 使用 preview CONFIG --template-id ID 查看，send 不重複逐欄列出 |

### 指定實際 UDP 來源 IP

```sh
./target/release/nfv9gen send examples/basic.json \
  --target 192.0.2.20:7777 --bind 192.0.2.10:55000
```

```sh
sudo ./target/release/nfv9gen send examples/basic.json \
  --target 192.0.2.20:7777 --source 198.51.100.123:55000
```

bind 與 source 都須和 target 使用相同 IP 版本。source 必須是明確 unicast，來源／目的 port>0，不接受 IPv6 source scope ID；需要 root 或 `CAP_NET_RAW`。Raw 模式自行建立 IP／UDP header、checksum，TTL／hop limit＝64，不主動分片，仍受路由與路徑 MTU 限制。這些參數不會改寫 JSON 內的流量位址。

分開執行 templates/data 時，請維持相同 source_id、來源 IP:port、Template ID 與定義，並確認 collector 的快取行為。一般測試優先使用同一次 `--kind all`。

### 靜默模式

任何指令加上 `-s` / `--silent`，stdout 只會輸出 `done` 或 `fail`，不顯示進度、統計或錯誤細節。退出碼不變。參數錯誤仍由參數解析器輸出。

```bash
nfv9gen send examples/basic.json --target 127.0.0.1:2055 --count 100 --silent
```

### 退出碼

| 退出碼 | 意義 |
|---:|---|
| 0 | 完成；stdout 輸出管線關閉（BrokenPipe）亦視為成功 |
| 1 | 設定、解析、socket、發送或統計輸出錯誤；stderr 的 BrokenPipe 屬於失敗 |
| 2 | CLI 參數錯誤 |
| 130 | Ctrl-C／SIGTERM 中斷 |

## 支援欄位

完整 value 範圍、每個欄位的範例與支援格式見 [field.md](field.zh-TW.md)。

以下 53 個一般欄位的 ID、長度及型別由 [src/fields.rs](src/fields.rs) 定義，固定寬度數值以 network byte order 編碼。一般整數為非負值，按欄位寬度限制；mask、IP version、MPLS TC 等另有語意範圍。

| 名稱 | Type ID | Bytes | 值 |
|---|---|---|---|
| IN_BYTES | 1 | 4 | 整數、值陣列、range 或 random_between |
| IN_PKTS | 2 | 4 | 整數、值陣列、range 或 random_between |
| FLOWS | 3 | 4 | 整數 |
| PROTOCOL | 4 | 1 | 整數 |
| SRC_TOS | 5 | 1 | 整數 |
| TCP_FLAGS | 6 | 1 | 字母組合、完整名稱、整數或 0x 字串 |
| L4_SRC_PORT | 7 | 2 | 整數、值陣列、range 或 random_between |
| IPV4_SRC_ADDR | 8 | 4 | IPv4 |
| SRC_MASK | 9 | 1 | IPv4 mask |
| INPUT_SNMP | 10 | 4 | 輸入介面 ifIndex，0..=4294967295 |
| L4_DST_PORT | 11 | 2 | 整數、值陣列、range 或 random_between |
| IPV4_DST_ADDR | 12 | 4 | IPv4 |
| DST_MASK | 13 | 1 | IPv4 mask |
| OUTPUT_SNMP | 14 | 4 | 輸出介面 ifIndex，0..=4294967295 |
| IPV4_NEXT_HOP | 15 | 4 | IPv4 |
| SRC_AS | 16 | 4 | ASN 整數／ASplain／AS 前綴／asdot |
| DST_AS | 17 | 4 | ASN 整數／ASplain／AS 前綴／asdot |
| BGP_IPV4_NEXT_HOP | 18 | 4 | IPv4 |
| LAST_SWITCHED | 21 | 4 | 整數，毫秒 |
| FIRST_SWITCHED | 22 | 4 | 整數，毫秒 |
| OUT_BYTES | 23 | 4 | 整數 |
| OUT_PKTS | 24 | 4 | 整數 |
| IPV6_SRC_ADDR | 27 | 16 | IPv6 |
| IPV6_DST_ADDR | 28 | 16 | IPv6 |
| IPV6_SRC_MASK | 29 | 1 | IPv6 mask |
| IPV6_DST_MASK | 30 | 1 | IPv6 mask |
| ICMP_TYPE | 32 | 2 | IPv4 type/code 物件、名稱或已合併整數 |
| SAMPLING_INTERVAL | 34 | 4 | 整數 |
| SAMPLING_ALGORITHM | 35 | 1 | 整數 |
| MPLS_TOP_LABEL_TYPE | 46 | 1 | 整數 |
| MPLS_TOP_LABEL_IP_ADDR | 47 | 4 | IPv4 |
| DST_TOS | 55 | 1 | 整數 |
| IP_PROTOCOL_VERSION | 60 | 1 | 4 或 6 |
| IPV6_NEXT_HOP | 62 | 16 | IPv6 |
| BGP_IPV6_NEXT_HOP | 63 | 16 | IPv6 |
| MPLS_LABEL_1 | 70 | 3 | 20-bit label 整數或物件 |
| MPLS_LABEL_2 | 71 | 3 | 20-bit label 整數或物件 |
| MPLS_LABEL_3 | 72 | 3 | 20-bit label 整數或物件 |
| MPLS_LABEL_4 | 73 | 3 | 20-bit label 整數或物件 |
| MPLS_LABEL_5 | 74 | 3 | 20-bit label 整數或物件 |
| MPLS_LABEL_6 | 75 | 3 | 20-bit label 整數或物件 |
| MPLS_LABEL_7 | 76 | 3 | 20-bit label 整數或物件 |
| MPLS_LABEL_8 | 77 | 3 | 20-bit label 整數或物件 |
| MPLS_LABEL_9 | 78 | 3 | 20-bit label 整數或物件 |
| MPLS_LABEL_10 | 79 | 3 | 20-bit label 整數或物件 |
| MPLS_VPN_RD | 90 | 8 | RD 字串，支援 Type 0 / 1 / 2 |
| MPLS_TOP_LABEL_PREFIX_LEN | 91 | 1 | FEC prefix length，0～128 |
| ICMP_TYPE_IPV6 | 139 | 2 | IPv6 type/code 物件、名稱或已合併整數 |
| MPLS_TOP_LABEL_IPV6_ADDR | 140 | 16 | IPv6 FEC 位址 |
| MPLS_TOP_LABEL_TTL | 200 | 1 | Top label TTL，0～255 |
| MPLS_LABEL_STACK_LENGTH | 201 | 4 | 原始 MPLS stack 長度，單位 bytes |
| MPLS_LABEL_STACK_DEPTH | 202 | 4 | 原始 MPLS stack 的 label 數 |
| MPLS_TOP_LABEL_EXP | 203 | 1 | Top label EXP/TC，0～7 |

## 範例與完整欄位測試

| 設定 | 內容 |
|---|---|
| [basic.json](examples/basic.json) | 單筆 IPv4 TCP |
| [shared-template.json](examples/shared-template.json) | 多組欄位搭配共用一個 Template，避免跨組配對 |
| [interfaces.json](examples/interfaces.json) | INPUT_SNMP／OUTPUT_SNMP |
| [nexthop.json](examples/nexthop.json) | 四種 IPv4／IPv6 next-hop |
| [random-ports.json](examples/random-ports.json) | 每次發送重抽來源 L4 port |
| [random-ips.json](examples/random-ips.json) | 每次發送從 [random-ips.txt](examples/random-ips.txt) 抽 IPV4_SRC_ADDR |
| [random-counters.json](examples/random-counters.json) | 每次發送重抽 IN_PKTS／IN_BYTES |
| [ranges.json](examples/ranges.json) | IP range、陣列與多欄位組合 |
| [options.json](examples/options.json) | 多 scope 取樣 Options |
| [asn.json](examples/asn.json) | ASN 表示法與 RD |
| [mpls.json](examples/mpls.json) | 手動 label、RD、IPv4／IPv6 FEC 與 metadata |
| [convenience.json](examples/convenience.json) | ICMP、prefix、range、明確設定的 MPLS stack 欄位 |
| [collector-all-fields.json](examples/collector-all-fields.json) | 全部 53 個一般欄位、5 種 scope，共 14 個 Template |

全欄位設定每輪 14 包 Data、22 筆紀錄：17 筆一般流量＋5 筆 Options。先啟動目的 port 的 collector，再執行：

```sh
./target/release/nfv9gen send examples/collector-all-fields.json \
  --target 192.0.2.20:7777 \
  --kind all --rate 10 --count 140 --template-every 14
```

這是十輪，共 170 筆一般流量＋50 筆 Options。沒有丟包、過濾或 Template 不相容時，未聚合的一般流量輸出預期為 170 筆；Options 不應算成一般流量列。collector 的取樣設定也可能影響計數，這份設定的 sampling interval 為 1。

驗證全欄位設定預期為 Source ID=424242、14 個 Template、22 筆紀錄、14 包 Data。可針對單組預覽：

```sh
./target/release/nfv9gen preview examples/collector-all-fields.json \
  --template-id 1005 --kind data --hex --limit 1
```

完整欄位對照、各組預期與 nfcapd／nfdump 命令見 [COLLECTOR-TEST.md](COLLECTOR-TEST.zh-TW.md)。實測即時抓包曾確認一輪 28 包（sequence 0～27）、17＋5 筆紀錄；`.0/32`、32-bit ifIndex 邊界及 10 層 MPLS label 符合設定。該解碼工具把 RD Type 1／2 顯示成兩個 32-bit 整數，且部分欄位只顯示數字 ID；此結果只證明封包抵達與該工具的解碼行為，不代表正式 collector 已入庫或保存全部欄位。

## JSON 大小與發行

`collector-all-fields.json` 為 15,629 bytes；自行移除縮排後為 7,259 bytes，約少 53.6%，設定及封包內容等值。只有空白被省略，鍵名仍是 name/value。減少設定重複可使用 prefix 或 range；這些寫法仍須保留想測試的欄位與值。修改欄位順序會改變 Template，不能當成任意設定的無損替換。

可用 Python 移除 JSON 縮排，轉檔後再驗證；NeuralEnsembleTelemetryFabricLuminalOracleWizard9 執行本身不需要 Python：

```sh
python3 -c 'import json,sys; json.dump(json.load(sys.stdin),sys.stdout,separators=(",",":"))' \
  < NeuralEnsembleTelemetryFabricLuminalOracleWizard9.json > NeuralEnsembleTelemetryFabricLuminalOracleWizard9.min.json
./target/release/nfv9gen validate NeuralEnsembleTelemetryFabricLuminalOracleWizard9.min.json
```

省略空白不會縮小 NetFlow 封包；原檔若含浮點寫法的大整數，轉檔工具可能影響精度，應使用規定的整數格式。

Release 已啟用 `lto="thin"`、`codegen-units=1`、`strip=true`，沒有導入加殼、程式混淆或防逆向機制。UPX 可作為額外的執行檔壓縮步驟，但可解殼，不能承諾防逆向；需另行測試相容性與效能。[UPX 官方文件](https://github.com/upx/upx/blob/devel/doc/upx.pod)

發行時提供 release executable、必要 JSON、HTML 及文件；原始碼與 target/debug 不必一起交付。Linux executable 仍需相容的 OS／動態函式庫，不代表跨平台單檔。執行檔、JSON、NetFlow 線路封包的大小是不同項目。

## 常見問題

| 現象 | 檢查與處理 |
|---|---|
| `Connection refused (os error 111)` | 先確認目標 IP／port 的 UDP listener 與防火牆；NeuralEnsembleTelemetryFabricLuminalOracleWizard9 收到錯誤即停止，先不要判定為欄位格式問題 |
| 抓包工具有看到，正式系統沒有資料 | 分別檢查 UDP 接收、來源允許清單、Template 解析、寫檔與入庫日誌 |
| `invalid netflow version` | 若抓包條件是所有 UDP，可能把其他流量送入解碼器；縮小為實際來源 IP 與目的 port，再保留 pcap 比對 |
| 只送出一小段就結束 | 未指定 count/duration 時只送一輪；持續執行請用 --duration 0 |
| 日誌沒有定期統計 | 統計約每 5 秒寫到 stderr；使用 >send.log 2>&1，短於 5 秒可能只有最終摘要；寫入失敗會停止並退出 1 |
| duration 被拒絕 | 無限模式只接受明確的 0；其他值須為可表示的有限正秒數，不接受其他零寫法或浮點下溢值 |
| 想看每個 Template 的模擬速率 | 使用 preview CONFIG --rate N；每個 Template 一組 IN／OUT，共用 ID 合併；發送中不重算 |
| RAM 用量偏高 | 規律值使用 prefix／range；大型 JSON 陣列解析有額外成本；50 萬筆不代表 RAM 必定低於 50 MB |
| BYTES 沒跟著 PKTS 改變 | 連動只在 HTML 的「Calculate from PKTS」模式；JSON 僅保存結果，重新匯入需自行啟用 |
| 共用 Template 結構不一致 | 相同 ID 的 scope、欄位名稱、順序與長度須一致；只改值，或為不同結構指定新 ID |
| collector 找不到 Template | 首次使用 all；若分程序送 templates/data，固定來源與 source_id，並核對 Template 定義 |
| JSON 格式錯誤 | 使用雙引號，移除註解、尾端逗號；name/value 不可縮成 n/v |
| random_file 讀不到 | 相對路徑以 JSON 檔所在資料夾為準，不是目前目錄；檔案須為 UTF-8、上限 8 MiB，每行純 IP、不含 CIDR prefix；依錯誤中的路徑與行號檢查 |
| IP 版本不符 | IPv4 欄位填 IPv4，IPv6 欄位填 IPv6；真實傳輸版本與紀錄版本分開設定 |
| 設定超過 500,000 筆 | 按各欄位值數相乘再加總所有 flow；random_between 與 random_file 各算一個設定值 |
| Template 或單筆紀錄放不下 | 合理提高 packet-size 或減少欄位；仍需符合實際路徑 MTU |
| `Message too long` | 降低 packet-size，尤其 raw 模式不主動分片；不要只看工具允許的最大值 |
| bind 失敗 | 使用本機已配置的 IP，確認 IP 版本及 port 是否被占用 |
| raw socket 權限不足 | source 模式需要 Linux root 或 CAP_NET_RAW；一般 UDP 不需要 sudo |
| Interface 顯示成不同值 | 核對 Template 長度為 4 bytes，測試 65536 以上值是否被截斷 |
| ICMP 顯示 2048 或 32768 | 這是合併的 type/code；分別代表 8/0、128/0 |
| MPLS RD Type 1／2 不符 | 核對 type 對應的解碼格式與原始 bytes，不能一律當成兩個 32-bit 整數 |
| packet/byte 計數變大 | 檢查 collector 的取樣換算與是否聚合；全欄位基準 interval 為 1 |
| 草稿未儲存或無法還原 | 草稿需自行開啟，僅存目前瀏覽器；檢查儲存限制，並下載備份 |

回報問題時保留執行命令、NeuralEnsembleTelemetryFabricLuminalOracleWizard9 與 collector 版本、發送摘要、接收日誌，以及必要時的 pcap。先核對封包與設定，再判斷是發送、解碼、保存還是顯示層的問題。

## 開發驗證

```sh
cargo test --locked
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
node --test tests/*.test.cjs
```

Rust 預設測試涵蓋設定驗證、欄位編碼、分包、Template 共用與刷新、分組 Collector 每分鐘預期平均速率、排程、取消、CLI 與全欄位設定覆蓋；另驗證 duration 浮點下溢拒絕、統計寫入／flush／BrokenPipe 失敗時停止，stderr 無法寫入時不 panic、慢速 burst 內的進度回報與取消，以及 count／duration 結束後不再呼叫進度回報。設定測試會拒絕已移除的語法與 IP 清單中的 CIDR；CLI 測試涵蓋全部 13 個內建 JSON 範例與隨機 IP 檔案預覽。Node.js 僅供開發測試；測試涵蓋生成與匯入／匯出往返，另含 IN／OUT 連動計算與 500,000 筆邊界檢查，以及使用 DOM 替身的事件測試，不能替代瀏覽器畫面、原生對話框、剪貼簿或下載測試。

編譯腳本測試以模擬發行版、套件管理員與 Rust 安裝流程執行，不會變更主機系統；涵蓋工具沿用、sudo 權限界線、下載／編譯失敗，以及含空白路徑的執行方式。

下列三項不隨預設測試執行：

```sh
cargo test --locked --test protocol udp_loopback -- --ignored
sudo cargo test --locked --lib raw_loopback -- --ignored
cargo test --locked --release --test protocol offline_throughput -- --ignored --nocapture
```

前兩項需要系統允許 socket，raw loopback 另需 Linux 與 CAP_NET_RAW。離線效能測試不含 UDP syscall、網路或 collector，不能據此宣稱線路吞吐。實機驗證仍需保留發送摘要、接收程式日誌與必要時的 pcap。

## 授權

[PolyForm Noncommercial 1.0.0](LICENSE)。可自由用於學習、研究、個人及其他非商業用途（包含修改後重新編譯）。禁止商業用途（不得販售本軟體或其修改版本）。
