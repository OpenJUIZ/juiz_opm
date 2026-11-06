# OPCAT の保存形式(OPX)

- 調査日: 2026-11-06
- 調べたもの: 公開されている OPCAT のソース [alex-shekhter/OPCAT](https://github.com/alex-shekhter/OPCAT) と、同梱の例 Examples/ATM.opz
- ライセンスの注意: OPCAT のソースは CC BY-NC 4.0。形式を読んで互換の書き出しを作るのはよいが、コードをこのリポジトリ(MIT / Apache-2.0)に持ち込んではいけない

## 形

.opz は .opx を gzip で圧縮しただけのもの。OPX は「モデル」と「図」を分けた XML で、スキーマ(XSD や DTD)はない。

| 部分 | 中身 |
| --- | --- |
| OPX/OPMSystem | 名前、作者、日時、id の採番の最大値 |
| LogicalStructure/ObjectSection | LogicalObject(名前、説明、physical、environmental、LogicalState の並び) |
| LogicalStructure/ProcessSection | LogicalProcess(名前、実行時間の下限と上限) |
| LogicalStructure/LinkSection | LogicalLink(linkType、sourceId、destinationId、condition) |
| LogicalStructure/RelationSection | LogicalRelation(relationType、両端、多重度) |
| VisualPart/OPD | 図ごとの配置と色。子の図は InZoomed と Unfolded に入れ子 |
| Scenarios | シミュレーションの初期状態 |

要素の共通部分は EntityAttr(id、environmental、name、description、url)、Thing は ThingAttr(physical、numberOfInstances など)を持つ。

## 種類の番号

`exportedAPI/OpcatConstants.java` で決まっている。

| 番号 | 種類 |
| --- | --- |
| 301 | consumption |
| 302 | effect |
| 303 | instrument |
| 304 | condition |
| 305 | agent |
| 306 | result |
| 307 | invocation |
| 308 | instrument event |
| 309 | exception |
| 310 | consumption event |
| 201 | specialization |
| 202 | exhibition |
| 203 | instantiation |
| 204 | aggregation |
| 205 | 片方向の名前付き構造リンク |
| 206 | 双方向の名前付き構造リンク |

## juiz の交換形式への示唆

- 取り入れる: モデルと図の分離、全要素に一意の id、リンクを両端の id で表すこと、in-zooming を図の入れ子で表すこと。
- 変える: 種類は数字ではなく ISO 19450 の名前にする。XSD を付ける。juiz 固有の情報は別の XML 名前空間に入れる。色は数字ではなく実装状況の名前で持つ。
- 確かめたい: OPCloud は OPCAT のファイルを読めるとされる。juiz から OPX を書き出せれば OPCloud で開ける見込みがある。
