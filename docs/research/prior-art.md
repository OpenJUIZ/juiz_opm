# OPM のモデル交換形式の先行例

- 調査日: 2026-11-06
- 結論: OPM の標準的な XML 交換形式は見つからなかった。あるのはツールごとの独自形式と、論文のメタモデルだけ。

| 対象 | 分かったこと | 出典 |
| --- | --- | --- |
| ISO 19450(2015 年版 PAS、現行は 2024 年版) | 図(OPD)と文(OPL)の記法を定める。交換形式の規定は検索では見つからなかった。本文はこの調査環境から開けず未確認 | [ISO 19450](https://www.iso.org/standard/62274.html) |
| OPCAT | 独自 XML(.opx、gzip 圧縮で .opz)で保存。XMI 出力の記述もある(古い資料) | [opcat-opx.md](opcat-opx.md)、[Gollner 2008](https://www.gollner.ca/2008/03/object-process.html)、[OPM vs UML](https://www.researchgate.net/publication/220920423_Object-Process_Methodology_OPM_vs_UML_-_a_Code_Generation_Perspective) |
| OPCloud | OPCAT のファイルを読めるとされる。外部連携は OSLC と RDF | [OPCloud](https://www.opcloud.tech/)、[経験報告](https://www.academia.edu/50114361/Designing_and_Developing_OPCloud_an_OPM_based_Collaborative_Software_Environment_in_a_Mixed_Academic_and_Industrial_Setting_An_Experience_Report) |
| 論文: 反射的メタモデル | Reinhartz-Berger と Dori による OPM 自身の OPM メタモデル。要素・状態・リンクの型の土台に使えそう | [A reflective meta-model of OPM](https://cris.haifa.ac.il/en/publications/a-reflective-meta-model-of-object-process-methodology-the-system-/) |
| 論文: OPM から SysML | OPM モデルから SysML のモデルと図を生成する Technion の研究 | [OPM to SysML](https://dovdori.technion.ac.il/wp-content/uploads/2022/02/OPM-to-SysML-DesignAndEvaluation.pdf) |

注意: 「Open Provenance Model」も OPM と略され、XML スキーマ(OPMX)を持つが別物。

## 未確認のこと

- ISO 19450 に OPL の形式文法(EBNF)の附属書があるか。2015 年版にあったと記憶しているが、本文で確かめていない。
- OPCloud が OPX を実際に読めるか。小さな .opz を作って試す。
- Dov Dori 先生への問い合わせ(2026-11-06 送付)の返事。
