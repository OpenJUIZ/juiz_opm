# 使い方

juiz_opm のコマンドで、juiz のシステムを OPM のモデル(juiz-OPL と交換形式の XML)にし、検査し、図や OPX にする手順。コマンドは `juiz_opm` 1 本にまとめてある([0018](decisions/0018-one-command-and-release.md))。

## 1. 入れる

どちらか。

- **配布物を使う**: [Releases](https://github.com/OpenJUIZ/juiz_opm/releases) から自分の OS の圧縮ファイル(Linux x86_64、macOS arm64 / x86_64、Windows x86_64)を落として展開し、`juiz_opm`(Windows は `juiz_opm.exe`)を PATH の通った場所に置く。
- **ソースから入れる**(Rust が要る):

  ```sh
  cargo install --git https://github.com/OpenJUIZ/juiz_opm juiz_opm
  ```

図の SVG と DOT には [Graphviz](https://graphviz.org/) の `dot`、PNG にはさらに librsvg の `rsvg-convert` が要る。conf の変換、検査、OPX の書き出しには何も要らない。

```sh
juiz_opm --version
juiz_opm --help
```

## 2. conf からモデルを作る

```sh
juiz_opm from-conf control_loop.conf -o control_loop.opl        # juiz-OPL
juiz_opm from-conf --xml control_loop.conf -o control_loop.xml  # 交換形式の XML
```

- conf の `option` や `plugins` のようにモデルの外のものは、警告を出して写さない。
- OPL は英語の文で、1 文が 1 つの事実。読み方は [spec/opl.md](../spec/opl.md)。

動いている System から作ることもできる。conf に書いてない引数の順やデータ型が入る([spec/system.md](../spec/system.md))。

```sh
juiz -f control_loop.conf -d &                                     # juiz で System を動かす
juiz_opm fetch http://127.0.0.1:8000 -o control_loop.json          # スナップショットを取る
juiz_opm from-system control_loop.json -o control_loop.opl
juiz_opm from-conf --system control_loop.json control_loop.conf -o control_loop.opl  # 両方を併せる
```

## 3. 検査する

```sh
juiz_opm check control_loop.conf control_loop.opl     # OPM の規則 C1〜C12 と juiz の規則 J1〜J13
juiz_opm check --system control_loop.json control_loop.opl   # J10(Composite の型名)も
juiz_opm opl check model.opl                          # OPM の規則だけ(juiz でないモデルにも)
juiz_opm opl fmt --write model.opl                    # 正規形に書き直す
```

終了の状態は、0 が問題なし、1 が誤りあり、2 が使い方の誤りか読み書きの失敗。警告だけなら 0。

## 4. 図にする

```sh
juiz_opm draw control_loop.opl > control_loop.svg            # 略式図(モデル全体を 1 枚)
juiz_opm draw -T png -o out/ tutorial.opm.xml                # OPD ごとに out/tutorial.SD.png など
juiz_opm draw -T dot --diagram SD1 coffee_making.opm.xml     # OPM の形の DOT
```

略式図の描き方は [spec/diagram.md](../spec/diagram.md)。図を OPD に分けて何を出すかを選ぶときは、XML の `<opm:diagram>` に書く。OPL と conf は OPD を持たないので、そこから作ったモデルは全体を 1 枚に描く。

## 5. OPCAT・OPCloud 向けに書き出す

```sh
juiz_opm opx control_loop.opl -o control_loop.opz   # .opz は gzip、.opx なら XML のまま
```

OPD の無いモデルには SD と in-zoom の OPD を作ってから書く。OPX に書けないものは標準エラーに出る([spec/opx.md](../spec/opx.md))。OPCAT で読めることは確かめてある。

## 6. conf に戻す

```sh
juiz_opm to-conf control_loop.opl -o control_loop.conf   # モデルから conf(正規形)
juiz_opm fmt-conf control_loop.conf                      # conf を正規形で出す
juiz_opm round-trip control_loop.conf                    # conf → OPL → conf が同じになるか
```

## コマンドの一覧

| コマンド | すること |
| --- | --- |
| `from-conf`、`from-system`、`fetch` | conf や動いている System からモデルを作る |
| `to-conf`、`fmt-conf`、`round-trip` | モデルから conf を作る、conf を正規形にする、往復を確かめる |
| `check` | OPM と juiz の規則で検査する |
| `opl check / fmt / to-xml / from-xml` | juiz-OPL の検査・整形・XML との変換(`opm_opl` と同じ) |
| `draw` | 略式図(SVG・PNG)、OPM の形の DOT、OPX(`opm_graph` と同じ) |
| `opx` | OPCAT・OPCloud 向けの OPX / OPZ(`draw -T opx` の短縮) |

`opm_opl` と `opm_graph` のコマンドも同じ配布物に入っていて、前と同じように使える。
