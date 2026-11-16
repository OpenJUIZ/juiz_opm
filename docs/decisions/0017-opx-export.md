# 0017 OPX は opm_graph が書き出し、OPD の無いモデルには OPD を作ってから書く

- 状態: 採用(案。OPCloud で開けるか確かめてから見直す)
- 日付: 2026-11-16
- 補うもの: [0003](0003-separate-repository.md) の表(OPX の書き出しを opm_model に置く予定)を置き換える。metamodel §6.2・§6.3 の「Step 9 で決める」を決める

## 背景

roadmap の Step 9 として、モデルを OPCAT の保存形式(OPX)に書き出し、OPCloud で開けるか確かめる([0002](0002-open-points.md) の 6)。

OPCAT に同梱の例(Examples/*.opz、6 つ)と、その読み書きのソースを読んで、OPX の形を確かめた。分かったこと:

- OPX はモデル(LogicalStructure)と図(VisualPart)を分けるが、図の線や三角は必ずどれかの論理の要素を指す。OPD の無いファイルは読めない。
- 要素の番号は数で、OPD の中の番号(entityInOpdId)も別に要る。根の OPD の番号は 1 に決まっている。
- in-zoom した Thing の中は、ChildrenContainer の入れ子と、親からの相対座標で表す。上の階層の OPD の、in-zoom した Process の外の線は、中の線とは別の論理の要素である。
- effect は Object → Process の向きで持つ。状態の変化は 301(元の State から)と 306(先の State へ)の 2 本で持つ。aggregation・generalization・classification の source は全体・一般・型の側。
- overtime と undertime の区別、ordered、effect と agent の modifier を表す番号は無い。

## 決めたこと

詳細は [spec/opx.md](../../spec/opx.md)。

1. **OPX の書き出しは opm_graph に置き、コマンドは `opm_graph -T opx` と `-T opz` にする。** OPD ごとに何を出すか(まとめた線を含む)は略式図と同じ `opm_graph::view` で決める。OPX は図を含むので、描く道具の出力の 1 つとして扱う。
2. **OPD の無いモデルには、SD と InZoom ごとの in_zoom の OPD を作ってから書く**(spec/opx.md §3)。conf やスナップショットから作ったモデルは OPD を持たない。
3. **まとめた線は、それぞれ論理の要素を足して書く**(§4.3)。モデルに「まとめた線」を持たない決まり(metamodel §7.1)は変えない。OPX に書くときだけ足す。
4. **書けないものは書かず、標準エラーで知らせる**(§4.4)。overtime と undertime はどちらも 309 にする(metamodel §6.2 の注)。
5. **配置は決まった規則で置き、線の引き方は読む道具に任せる**(§5)。Graphviz は使わない。Appearance の layout があればそれを使う。
6. **検査は、読み手が要るものをテストで確かめる**(§6)。OPCAT のコードは持ち込まず、動かしもしない。
7. OPX を読むこと(OPCAT・OPCloud のモデルを取り込むこと)は、この Step ではしない。

## 理由

- OPX の大半は図(OPD・座標・線)で、何を出すか、まとめた線をどう導くかは opm_graph が既に持っている。opm_model に置くと、その規則を 2 か所に持つことになる。
- conf から作ったモデルは juiz の道具の主な入力なので、OPD が無くても開けるファイルにしたい。
- 配置に Graphviz を使うと、OPX だけ欲しい人にも Graphviz が要る。開いた後に OPCloud で置き直せるので、決まった規則で十分。
- 読み手の要るもの(番号の参照、線の端と要素の端が合うこと)は、OPCAT の読み手が誤りにする点そのものなので、テストで押さえれば開けない原因の大半を除ける。

## 影響

- opm_graph に `opx`(`to_opx`、`gzip`、`Options`)と `-T opx`・`-T opz` を足した。依存に flate2 を足した。
- opm_model の `Modifier` に `Hash` を足した。
- metamodel §6.2(overtime と undertime)と §6.3(OPCAT の関係の向き)の注を、この決定を指すように直した。
- OPCloud で開けるかは ysuga が確かめる(spec/opx.md §7)。開けなければ、この決定を見直す。
