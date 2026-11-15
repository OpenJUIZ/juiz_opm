# 0012 opm_opl は opm_model の型に読み、正規形の文の順で id を作る

- 状態: 採用(案。OPL の言い回しは ISO 19450 の本文で見直す、[0010](0010-settle-without-dori-reply.md) の 4)
- 日付: 2026-11-15

## 背景

roadmap の Step 5 として、[spec/opl.md](../../spec/opl.md) の juiz-OPL を読み、検査し、正規形で書き出す道具を作る。opl.md §8 と §14 で「規則に反したときの報告の形、誤りと警告の区別、コメントを `fmt` で残すか」を Step 5 に残していた。

## 決めたこと

詳細は [spec/opl.md](../../spec/opl.md) §15。

1. **クレート opm_opl を足し、モデルは opm_model の型をそのまま使う。** OPL 用の別のモデルは持たない。juiz の注釈は opm_model の `Extensions`(juiz の名前空間の属性と子要素)に入れ、XML に書くときの形(xml.md §7.2)と同じにする。依存は opm_model だけで、JSON の値も自前で確かめる(文字列と数を書いたまま保つため)。
2. **読むのは 3 段にする。** 字句、文法(文ごとに誤りから立ち直る)、モデルへの組み立て(名前の照合と O の規則)。`check` は O の誤りがなければ opm_model の C1〜C12 も見る。
3. **報告は opm_model と同じ形にし、文の位置を足す。** 番号は `syntax`、O1〜O12、C1〜C12・F1〜F4、`unwritable`、`dropped`。誤りにしないのは C8 と `dropped` だけ。
4. **id のない要素の id は、正規形の文の順で作る。** リンクと Fan の id がぶつかるときの `-2`、`-3` は、書き出したときの文の順で決まる。こうすると、同じモデルの OPL はどう書いても同じ id になる。ぶつかったときの後ろの付け方は Thing と State が `_2`、それ以外が `-2`(opl.md §10.4 を補った)。
5. **`fmt` はコメントを残さない。** コメントはモデルに入らない(opl.md §2)ので、正規形はモデルだけから決まるものにする。id は読んだ文書に `{id: ...}` があれば書く。
6. **CLI は `opm_opl check / fmt / to-xml / from-xml` にする。** `check` は XML も受ける。
7. **往復は文字列で比べる。** examples/xml から書いた OPL が examples/opl と一字一句同じであることを CI で確かめる。手で書いた capture_filter.opl の `[...]` の並び(value_kind と datatype)が正規形と違っていたので直した。

## 理由

- 型を分けると、XML と OPL の対応をもう一度書くことになり、往復の確かめが 2 つの型の対応に依存してしまう。
- 文ごとに立ち直ると、Claude や人が書いた長い OPL の誤りを 1 回で直せる。
- id を文の順で作れば、文の並びを変えても id が変わらない(正規形の並びは文の中身だけで決まる)。

## 影響

- Step 6(opm_graph)と Step 7(juiz_opm)は、OPL を opm_opl で読んで opm_model の型を受け取る。
- `tests/opl/` に、文の形を 1 つずつ持つ all_forms.opl と、O の規則ごとの誤りの例を置いた。CI は OPL から書いた XML も XSD に通す。
