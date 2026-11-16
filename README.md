# juiz_opm

[juiz](https://github.com/OpenJUIZ/juiz) のシステムの設計と実装状況を、OPM(Object-Process Methodology, ISO 19450)で記述・交換・図示するための仕様と道具を置くリポジトリです。

Specifications and tools for describing, exchanging and drawing juiz system models with OPM (Object-Process Methodology, ISO 19450).

## 使い方

コマンドは `juiz_opm` 1 本です。[Releases](https://github.com/OpenJUIZ/juiz_opm/releases) の配布物を使うか、`cargo install --git https://github.com/OpenJUIZ/juiz_opm juiz_opm` で入れます。

```sh
juiz_opm from-conf my_system.conf -o my_system.opl   # juiz の conf から juiz-OPL
juiz_opm check my_system.opl                         # OPM と juiz の規則で検査
juiz_opm draw my_system.opl > my_system.svg          # 略式図(Graphviz が要る)
juiz_opm opx my_system.opl -o my_system.opz          # OPCAT・OPCloud 向け
```

通しの手順とコマンドの一覧は [docs/usage.md](docs/usage.md) にあります。

### In English

`juiz_opm` turns juiz confs (or a running juiz System) into OPM models written in juiz-OPL or the interchange XML, checks them against the OPM and juiz rules, draws them as diagrams (SVG / PNG through Graphviz), and exports them as OPCAT files (OPX) for OPCAT and OPCloud. Install a release archive or run `cargo install --git https://github.com/OpenJUIZ/juiz_opm juiz_opm`; `juiz_opm --help` lists the commands. The documents in this repository are written in Japanese; the OPL itself is English.

## 状態

仕様づくりと道具づくりの段階です。交換形式の XML スキーマと検証スクリプト、juiz-OPL の文法、交換形式を読み書きして検査する opm_model クレート、juiz-OPL を読み書きして検査する opm_opl クレート、モデルを図や OPX にする opm_graph クレート、juiz の conf と相互に変換する juiz_opm クレートがあります。

## 構成

| 場所 | 中身 |
| --- | --- |
| [spec/opm-mapping.md](spec/opm-mapping.md) | juiz の概念と OPM の対応、描画の決めごと |
| [spec/metamodel.md](spec/metamodel.md) | モデルが持つもの(要素、状態、リンク、図の階層、juiz の拡張)の表 |
| [spec/xml.md](spec/xml.md)、[spec/schema/](spec/schema/) | 交換形式の XML とその XSD |
| [spec/opl.md](spec/opl.md) | juiz-OPL の文法(EBNF)、整形式の規則、正規形、XML との対応 |
| [spec/diagram.md](spec/diagram.md)、[spec/diagram/](spec/diagram/) | 略式図の描き方と、手で描いた見本 |
| [examples/xml/](examples/xml/)、[examples/opl/](examples/opl/) | 交換形式の例と、その juiz-OPL の書き起こし |
| [scripts/validate_xml.py](scripts/validate_xml.py) | 例と tests/xml/ を XSD で検査する |
| [crates/opm_model/](crates/opm_model/) | メタモデルの型、交換形式の読み書き、コアの規則の検査 |
| [crates/opm_opl/](crates/opm_opl/) | juiz-OPL の読み書きと検査、XML との変換(コマンド `opm_opl`) |
| [crates/opm_graph/](crates/opm_graph/) | モデルを略式図(SVG / PNG)や OPM の形の DOT、OPX にする(コマンド `opm_graph`) |
| [spec/conf.md](spec/conf.md) | juiz の conf との往復(読み方、正規形、写し方、往復テスト) |
| [spec/system.md](spec/system.md) | 動いている System からの書き出し(スナップショット、manifest から足すもの、J10) |
| [spec/opx.md](spec/opx.md) | OPCAT の保存形式(OPX)への書き出し(OPCAT・OPCloud で開くため) |
| [crates/juiz_opm/](crates/juiz_opm/) | juiz の conf との相互変換と juiz の規則の検査(コマンド `juiz_opm`) |
| [tests/conf/](tests/conf/) | サンプル 4 リポジトリの conf と、そこから書いた juiz-OPL |
| [tests/opl/](tests/opl/) | OPL の文の形をすべて使う例と、誤りの例 |
| [docs/decisions/](docs/decisions/) | 決めたことの記録(1 件 1 ファイル) |
| [docs/research/](docs/research/) | 先行例の調査(OPCAT の保存形式など) |
| [docs/usage.md](docs/usage.md) | 使い方(入れ方、conf から図・OPX までの手順、コマンドの一覧) |
| [docs/roadmap.md](docs/roadmap.md) | 今後の作業の順番 |

道具(Rust の workspace、`crates/` に置く):

| クレート | 役割 | 状態 |
| --- | --- | --- |
| opm_model | OPM のメタモデル、交換用 XML の読み書き、コアの規則の検査 | あり(Step 4) |
| opm_opl | OPL の解析・検査・正規形での出力、XML との変換(コマンド `opm_opl`) | あり(Step 5) |
| opm_graph | 略式図の SVG / PNG と、OPM の形の DOT の描画、OPCAT・OPCloud 向けの OPX の書き出し(コマンド `opm_graph`) | あり(Step 6、9) |
| juiz_opm | juiz の conf と juiz-OPL・XML の相互変換、動いている System からの書き出し、juiz の規則 J1〜J13 の検査。他のクレートのコマンドもまとめた `juiz_opm` コマンド | あり(Step 7、8、0018) |

## ライセンス

次のどちらかを選んで使えます。

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) または <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) または <https://opensource.org/licenses/MIT>)

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

### コントリビューション

明示的に別の意思表示をしない限り、このリポジトリに提出されたコントリビューションは、Apache-2.0 ライセンスの定義に従い、追加の条件なしに上記と同じデュアルライセンスで提供されるものとします。

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
