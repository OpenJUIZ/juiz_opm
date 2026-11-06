# 0003 仕様と道具は juiz_opm リポジトリに置く

- 状態: 採用
- 日付: 2026-11-06

## 背景

対応の議論を記録しながら進め、OPL を受けて図を描く道具(opl_graph)も作りたい。Claude も同じ道具を使えるようにしたい。

## 決めたこと

OpenJUIZ/juiz_opm を作り、仕様(spec/)、決めたことの記録(docs/decisions/)、調査(docs/research/)、道具(crates/)を置く。ライセンスは他の OpenJUIZ リポジトリと同じ MIT OR Apache-2.0。

道具は Rust の workspace とし、次のクレートに分ける予定。

| クレート | 役割 |
| --- | --- |
| opm_model | メタモデル、交換用 XML の読み書き、OPX の書き出し |
| opm_opl | OPL の解析と正規形での出力 |
| opm_graph | DOT / SVG の描画(コマンド opl_graph) |
| juiz_opm | juiz 拡張と conf との往復 |

## 理由

- 交換形式と描画は OPM 一般の道具で、juiz の外でも使える。
- 版の上げ方を juiz 本体と分けられる。後で juiz 本体の `juiz opl` がこのクレートを使えばよい。
- juiz と同じ Rust にすれば、juiz の conf の読み込みや単一バイナリでの配布がそのまま使える。
- 名前 opmx は Open Provenance Model の XML 形式と重なるので避けた。
