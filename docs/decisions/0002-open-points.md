# 0002 対応の未決事項 7 点の決定

- 状態: 採用
- 日付: 2026-11-06

## 決めたこと

ysuga が「基本的には推奨通りで良い、ツールは作りたい」と判断した(6 のみ推奨と違う)。

1. **pull 接続**: invocation の線は引かず、Object を共有した上で instrument 線に「pull」と書く。OPM の invocation は「終わったら次を起こす」意味で、pull の「先に計算させて値を受け取る」とずれるため。push は invocation P1→P2 で描く。
2. **引数**: instrument にする。inlet の値は実行後も残るため。Topic のメッセージを consumption にするかは、交換形式を決めるときに改めて見る。
3. **ブローカーと配置**: どの System がどのブローカーでつながるかは C4 風の別の図にし、OPM の図には入れない。
4. **型と実体**: 普段は実体(inc0 など)を描き、型は例示(instance of)で必要な時だけ出す。conf と照らし合わせやすいため。
5. **OPL の言語**: 英語の OPL を正本にし、日本語の定型文を対応づけておく。
6. **ツール**: Graphviz で描くだけでなく、専用の道具を作る([0003](0003-separate-repository.md))。OPCloud の併用は、OPX 書き出しで開けるか試してから決める。
7. **置き場所**: 当初の案は juiz の docs/spec/opm-mapping.md だったが、[0003](0003-separate-repository.md) で juiz_opm リポジトリの spec/ に変えた。
