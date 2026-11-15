# 0010 Dori 先生の返事を待たず、交換形式の名前は今の形で決める

- 状態: 採用
- 日付: 2026-11-15
- 置き換えるもの: [0004](0004-xml-interchange.md)、[0006](0006-xml-format.md)、[0007](0007-opl-grammar.md)、[0008](0008-opl-wording-from-book.md)、[0009](0009-opm-model-crate.md) の「Dov Dori 先生の返事次第で見直す」という条件

## 背景

2026-11-06 に Dov Dori 先生へ、OPM の XML 交換形式の発表や計画があるかを問い合わせた([0004](0004-xml-interchange.md))。返事を待つ間、XML の名前空間と要素名などを仮置きにして Step 2〜4 を進めた([spec/xml.md](../../spec/xml.md) §12)。2026-11-15 の時点で返事はない。ysuga が「こちらで判断して動こう」と決めた。

## 決めたこと

1. **返事を待たない。** 0004・0006・0009 は、返事次第で見直すという条件を外して採用とする。決定の中身は変えない。
2. **xml.md §12 で仮置きにしていた点は今の形で決める。** 名前空間 URI(`https://openjuiz.github.io/ns/opm/v0`、`https://openjuiz.github.io/ns/juiz/v0`)、コアを juiz_opm 独自の名前空間に置くこと、ISO 19450 の用語の綴りの要素名・属性名、リンクを種類ごとの要素に分けること、図(OPD)と配置の持ち方。
3. **後で既存の形式が分かったら、変換の道具で対応する。** そのときは今の形式を置き換えるのではなく、写す変換(OPX の書き出しの Step 9 と同じ扱い)を足すかを新しい決定で決める。
4. **OPL の言い回しの見直しは ISO 19450 の本文だけを待つ。** [0007](0007-opl-grammar.md)・[0008](0008-opl-wording-from-book.md) と spec/opl.md §13 の仮置きは、Dori 先生の返事ではなく ISO 19450 の本文で確かめられたときに見直す。

## 理由

- 返事を待つと、Step 5 以降(opm_opl、opm_graph、juiz_opm)が仮置きの名前の上に積み上がり続ける。
- 名前空間 URI の `v0` は 0.x の間の互換性のない変更を許す([spec/xml.md](../../spec/xml.md) §11)ので、決めても後で直す道は残る。

## 影響

- spec/xml.md §12 を「仮置き」から「決めたこと」に改めた。
- docs/roadmap.md、docs/decisions/README.md、docs/research/prior-art.md から「Dori 先生の返事次第」を外した。
