# 0008 juiz-OPL の言い回しを Dori の入門書に合わせる

- 状態: 採用(案。ISO 19450 の本文か Dov Dori 先生の返事で見直す)
- 日付: 2026-11-07
- 置き換えるもの: [0007](0007-opl-grammar.md) の 4(modifier の文)のうち condition の consumption の後半と、[spec/opl.md](../../spec/opl.md) の Step 3 時点の InZoom・Fan・例外リンク・initial と final・多重度の形

## 背景

0007 の文の言い回しは、ISO 19450 の本文を読めないまま決めた仮置きだった。ysuga が OPM の入門書 D. Dori, *Model-Based Systems Engineering with OPM and SysML*(Springer, 2016)の本文を共有したので、OPL の文型を照らし合わせた。入門書は ISO 19450 の記法を使い、その上位集合とされる。

## 決めたこと

詳細は spec/opl.md §13。

1. **入門書に例のある文型はそれに合わせる。**
   - InZoom: `P zooms into A and B, in that sequence, as well as X.`、並行の step は `parallel A and B`(Step 3 の `then` の形はやめる)
   - Fan: XOR は 2 本なら `either A or B`、3 本以上なら `exactly one of A, B, or C`。OR は `at least one of A, B, or C`(Step 3 は `and`)
   - 例外リンク: `P2 occurs if duration of P1 exceeds 60 min.`(Step 3 は `exceeds its maximum duration`)。数は P1 の duration と一致させる
   - initial と final: `Vehicle is initially intact and finally crashed.`(Step 3 は `State s of A is initial.`)。default は `State s of A is default.` のまま
   - condition の consumption: `..., in which case P consumes A, otherwise P is skipped.`(Step 3 は `A is consumed`、入力では残す)
   - reciprocal の tagged: `A and B are tag.`
   - 多重度: `2 to many`、`at least one`、`optional`、`an optional`
   - 3 つ以上の並びは最後の `and` / `or` の前にも `,` を置く
2. **解析が一意でなくなるところは合わせない。** 文頭の State を大文字にしない、kind を宣言に書く、backward_tag のある tagged を 1 文にする、多重度の後の名前を複数形にしない。
3. **入門書で確かめられなかった形は仮置きのまま残す。** default の State、condition の agent、undertime の動詞、duration、経路ラベル。

## 理由

- 人と Claude が書く OPL が、OPM の利用者が読み慣れた形になる。OPCAT などの道具の OPL とも近くなる見込み。
- juiz-OPL は 1 文 1 事実と一意の解析を前提にしている(0007)ので、それを崩す形は採らない。

## 影響

- examples/opl/ の 3 つを新しい形に直した。
- Step 5 の opm_opl は、0007 の形のうち spec/opl.md に「入力だけ」と残したものも読む。
