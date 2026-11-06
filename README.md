# juiz_opm

[juiz](https://github.com/OpenJUIZ/juiz) のシステムの設計と実装状況を、OPM(Object-Process Methodology, ISO 19450)で記述・交換・図示するための仕様と道具を置くリポジトリです。

Specifications and tools for describing, exchanging and drawing juiz system models with OPM (Object-Process Methodology, ISO 19450).

## 状態

議論と仕様づくりの段階です。交換形式の XML スキーマと検証スクリプト、juiz-OPL の文法はありますが、道具のコードはまだありません。

## 構成

| 場所 | 中身 |
| --- | --- |
| [spec/opm-mapping.md](spec/opm-mapping.md) | juiz の概念と OPM の対応、描画の決めごと |
| [spec/metamodel.md](spec/metamodel.md) | モデルが持つもの(要素、状態、リンク、図の階層、juiz の拡張)の表 |
| [spec/xml.md](spec/xml.md)、[spec/schema/](spec/schema/) | 交換形式の XML とその XSD |
| [spec/opl.md](spec/opl.md) | juiz-OPL の文法(EBNF)、整形式の規則、正規形、XML との対応 |
| [examples/xml/](examples/xml/)、[examples/opl/](examples/opl/) | 交換形式の例と、その juiz-OPL の書き起こし |
| [scripts/validate_xml.py](scripts/validate_xml.py) | 例と tests/xml/ を XSD で検査する |
| [docs/decisions/](docs/decisions/) | 決めたことの記録(1 件 1 ファイル) |
| [docs/research/](docs/research/) | 先行例の調査(OPCAT の保存形式など) |
| [docs/roadmap.md](docs/roadmap.md) | 今後の作業の順番 |

予定している道具(Rust の workspace、`crates/` に置く予定):

| クレート | 役割 |
| --- | --- |
| opm_model | OPM のメタモデル、交換用 XML の読み書き、OPCAT の OPX の書き出し |
| opm_opl | OPL の解析と正規形での出力 |
| opm_graph | DOT / SVG の描画(コマンド `opl_graph`) |
| juiz_opm | juiz 拡張と juiz の conf との相互変換 |

## ライセンス

次のどちらかを選んで使えます。

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) または <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) または <https://opensource.org/licenses/MIT>)

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

### コントリビューション

明示的に別の意思表示をしない限り、このリポジトリに提出されたコントリビューションは、Apache-2.0 ライセンスの定義に従い、追加の条件なしに上記と同じデュアルライセンスで提供されるものとします。

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
