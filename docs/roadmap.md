# 進め方

1 Step を 1 PR とし、Step ごとに新しい作業スレッドで進める。

| Step | 内容 | 状態 |
| --- | --- | --- |
| 0 | リポジトリの土台、対応の仕様、決めたことの記録、調査の移植 | この PR |
| 1 | メタモデルの表(要素、状態、リンク、図の階層、juiz の拡張)を spec/ に書く。XML の書き方とは切り離す | 未着手 |
| 2 | 交換形式の XML と XSD、juiz 名前空間の仕様 | 未着手(Dori 先生の返事を見てから) |
| 3 | juiz-OPL の文法(EBNF)、整形式の規則、正規形 | 未着手 |
| 4 | opm_model: メタモデルと XML の読み書き | 未着手 |
| 5 | opm_opl: OPL の解析・検査・正規形での出力(check, fmt) | 未着手 |
| 6 | opm_graph: DOT / SVG の描画、実装状況の色(opl_graph) | 未着手 |
| 7 | juiz_opm: juiz の conf との往復と往復テスト、サンプル 4 リポジトリの題材 | 未着手 |
| 8 | 動いている System からの書き出し(profile_full) | 未着手 |
| 9 | OPX の書き出しと OPCloud での確認 | 未着手 |
| 10 | Claude 用の skill、juiz 本体の `juiz opl`、公式サイト | 未着手 |

題材づくり(サンプル 4 リポジトリの conf を手で juiz-OPL に書き起こす)は Step 1〜3 と並行してよい。
