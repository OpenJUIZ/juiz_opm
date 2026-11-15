# 0013 opm_graph は DOT を自前で書き、配置と SVG は Graphviz に任せる。Step 6 は 6a と 6b に分ける

- 状態: 採用(案)
- 日付: 2026-11-15

## 背景

roadmap の Step 6 として、モデルを図にする道具を作る。[0011](0011-abbreviated-diagram.md) で図は略式図 1 種類にしたが、略式図は「組にならない Thing とリンクは OPM の形で描く」([spec/diagram.md](../../spec/diagram.md) §3.11)ので、OPM の形で描く部分が土台になる。diagram.md §5 で Step 6 に残した「上の階層の図のまとめた線」もここで決める。

## 決めたこと

1. **Step 6 を 2 つの PR に分ける。**
   - 6a(この決定): クレート opm_graph、描く範囲(diagram.md §2)、OPM の形(§3.11)、実装状況の色(§3.10)、InZoom の中身、まとめた線。juiz の注釈は名前の下に小さく `role : type` と書くだけで、記号にはまとめない。
   - 6b: juiz の記号(§3.1〜§3.9、ポート、Container の箱、EC の periodically invokes、System の枠、Transform など)。
2. **DOT は opm_graph が書き、配置と SVG・PNG は Graphviz の `dot` コマンドに任せる。** Rust のクレートは opm_model と opm_opl だけに依存する。`dot` は描くときだけ要る(`OPM_GRAPH_DOT` で別のコマンドを指せる)。CI は Graphviz を入れ、例がすべて SVG になることを確かめる。
3. **コマンドは `opm_graph`(クレート名と同じ)。** `opm_graph [-T dot|svg|png] [--diagram <名前>] [-o <出力>] <file.opl | file.xml>`。OPD ごとに 1 枚、OPD のないモデルは全体を 1 枚。roadmap と README にあった `opl_graph` はこれに置き換える。
4. **まとめた線(metamodel §7.1)の導き方。** 描かれていない Thing の端は、それを中に持つ InZoom の refinee をたどり、描かれている最も内側のものに移す。
   - 移した線は、種類・両端・修飾の有無が同じなら 1 本にする。
   - 両端が同じ refinee に移った線(中だけの線)は描かない。
   - 状態を指定した effect は、端を移したら状態を外して effect にする(中の状態変化を上の図では「変える」とだけ言う)。移さなかった側の State の端は残す。
5. **描き方の細部。**
   - Appearance の `suppressed_states` に挙げた State を端に取る線は、その Object に付ける。
   - 構造リンクは、始点・種類(aggregation なら ordered も)が同じものを 1 つの三角にまとめ、三角から各終点へ線を引く。三角の頂点は全体の側を向く。
   - Fan は線のラベルに `XOR` / `OR` と書く(弧は描かない)。
   - InZoom を持つ Thing が中の Thing と一緒に描かれるときは、中を囲む枠にする(Process は角の丸い枠)。枠のある図は上から下へ(OPM の InZoom の時間の向き)、ない図は左から右へ並べる。step の順は見えない線で保つ。
   - in_zoom の OPD は、refinee の Appearance がなくても refinee を描く。
   - リンクの実装状況は線の色とラベル(refs)で表す。

## 理由

- 配置を自前で書くのは大きな仕事で、Graphviz の dot は層状の配置が十分に良い。見本(spec/diagram/)と試作(プロジェクトの abbrev_preview・transform_preview)も Graphviz で配置している。
- 6b のポートなど Graphviz の形で描けない記号は、試作と同じく `dot -Tjson` で位置だけ受け取り SVG を自前で描く。その仕組みは 6a の DOT と分けて 6b で入れる。
- まとめた線を導く規則がないと、上の階層の図(SD)で InZoom した Process の外との線が消える。OPM の書き方(OPCAT、OPCloud)も、中の線を外の Thing に引き上げて描く。

## 影響

- workspace に crates/opm_graph を足した。CI(rust.yml)は Graphviz を入れる。
- diagram.md §5 の「上の階層の図のまとめた線」は、この決定の 4 で決めた。
- examples に talker_listener(チュートリアル 4 章)を足した。
- 6b で、juiz の注釈のあるモデルは記号にまとめて描くようになる。6a の出力はそのとき、juiz の注釈のないモデル(coffee_making など)の図として残る。
