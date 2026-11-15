# 0014 略式図は記号にまとめてから Graphviz で配置し、SVG は opm_graph が描く

- 状態: 採用(案)
- 日付: 2026-11-15
- 補うもの: [0011](0011-abbreviated-diagram.md)、[0013](0013-opm-graph-crate.md)

## 背景

roadmap の Step 6b として、略式図の juiz の記号([spec/diagram.md](../../spec/diagram.md) §3.1〜§3.9)を opm_graph に入れる。ポートを楕円の輪郭に置くなど、Graphviz の形では描けない記号がある。Step 6 の前の試作(プロジェクトの abbrev_preview・transform_preview)は、Graphviz に位置だけを決めさせ、形は自前で描いていた。

## 決めたこと

1. **3 段で描く。** `abbrev` が View を記号にまとめ(どの要素がどの記号に入るか)、`layout` が記号の大きさの空の箱を Graphviz に渡して位置を受け取り(`dot -Tjson`)、`draw` が SVG を書く。線の経路は Graphviz の経路を使わず、端の向きに沿った 3 次ベジエにする(ポートの位置は Graphviz が知らないため)。線のラベルは配置用の DOT にも付け、層の間にラベルの幅を取らせる。
2. **juiz の要素が 1 つもない View は、6a の OPM の形の DOT を Graphviz に描かせる。** まとめる組がないので略式図は OPM の図になる(0011 の 2)。OPM の形は Graphviz の方がよく描ける(構造リンクの三角の束ね、InZoom の上下の向き)。
3. **CLI の出力。** `-T svg` と `-T png` は略式図。PNG は SVG を `rsvg-convert`(librsvg)で変換する(`OPM_GRAPH_RSVG` で別のコマンドを指せる)。`-T dot` は 6a の OPM の形の DOT のまま(Graphviz の道具で開けるもの)。
4. **diagram.md §3 の読み方を補った。**
   - §3.1 の楕円は ec_cycle と client 以外のすべての Process に使う。client は破線の楕円(§3.9)。ec_cycle は EC の記号に入る。
   - 入力ポートは `juiz:arg` の付いた instrument ごとに 1 つ。源の value が他の Process の result なら、その出力ポートから線を引く(接続。`juiz:connection` がなければ Composite の中の値の受け渡しとして細線でラベルなし)。源の value がこのリンクの他に System の aggregation しか持たなければ、白い丸にして value を描かない(`juiz:default` か const の `juiz:value` を `= 値` の注記にする)。それ以外の源は OPM の形で描き、そこから黒い丸へ線を引く。
   - push の invocation は、モデルに同じ組の push 接続があれば描かない(接続の線が表す)。
   - EC の線は、ec_cycle がモデルにあれば(OPD に Appearance がなくても)描く。ec が出ていれば ec_cycle は描かない。
   - Transform の線は transform と両端の Container が出ているときに描き、transform の Object は描かない。ラベルは `(x, y, z)`、回転があれば `rot (x, y, z, w)` を足す。
   - Topic への result は出力ポートの位置から `publish` の線、Topic からの instrument は入力ポートへの `subscribe` の破線にする。
   - InZoom の枠は System の枠と同じ仕組みで描く(角の丸い黒い枠)。step の順は左から右。
   - OPM の形で残る構造リンクは、始点の側に三角を置いた 1 本の線にする(6a の束ねる三角は Graphviz で描く OPM の形だけ)。
5. **ラベルと注記の重なりはまだ避けない。** diagram.md §5 のとおり残す。

## 理由

- 記号へのまとめ方を描き方から分けると、テストで「どの要素がどの記号になったか」を画像なしで確かめられる。
- 線の経路まで自前で決めると大きな仕事になる。ポートから出て端の向きに伸びる曲線で、試作と同じ見た目は得られる。
- PNG はスレッドや文書に貼るのに要る。Graphviz は自前の SVG を PNG にできないので、広く入っている librsvg を使う。

## 影響

- opm_graph に `abbrev`・`layout`・`draw` を足し、`draw_svg` と `abbreviated_svg` を公開した。
- roadmap の Step 6b を済みにした。残る点: 線とラベルの重なり、上の階層の図で InZoom した Composite の中の線を引き上げたときのポートの扱いの見直し、System をまたぐ Transform(diagram.md §5)。
