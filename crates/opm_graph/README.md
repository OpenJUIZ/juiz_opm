# opm_graph

OPM のモデル(交換形式の XML か juiz-OPL)を略式図にする。OPD ごとに 1 枚。OPCAT・OPCloud で開く OPX にも書き出す。

- 図の描き方: [spec/diagram.md](../../spec/diagram.md)
- OPX の書き出し: [spec/opx.md](../../spec/opx.md)
- 決定: [docs/decisions/0013](../../docs/decisions/0013-opm-graph-crate.md)(クレート、描く範囲、OPM の形)、[0014](../../docs/decisions/0014-abbreviated-renderer.md)(略式図の描き方)、[0017](../../docs/decisions/0017-opx-export.md)(OPX)

モデルの要素の組を記号(ポート、接続、Container、EC、System の枠、Transform など)にまとめ、Graphviz に配置させてから SVG を書く。juiz の要素がない図は、OPM の形の DOT を Graphviz が描く。

```rust
let model = opm_graph::read_model("examples/xml/talker_listener.opm.xml".as_ref())?;
for d in opm_graph::draw_svg(&model)? {  // Graphviz の dot を呼ぶ
    std::fs::write(format!("{}.svg", d.name), d.svg)?;
}
let dot = &opm_graph::draw(&model)[0].dot; // OPM の形の DOT
```

コマンド([Graphviz](https://graphviz.org/) が要る。PNG には librsvg の `rsvg-convert` も要る):

```sh
cargo run -p opm_graph -- examples/xml/talker_listener.opm.xml > tl.svg     # 略式図の SVG
cargo run -p opm_graph -- -T png -o out/ examples/xml/tutorial.opm.xml      # SD と SD1 の 2 枚
cargo run -p opm_graph -- -T dot --diagram SD1 examples/xml/coffee_making.opm.xml  # OPM の形の DOT
cargo run -p opm_graph -- -T opz -o coffee.opz examples/xml/coffee_making.opm.xml  # OPCloud・OPCAT 向け(Graphviz は要らない)
```

## ライセンス

MIT OR Apache-2.0(リポジトリの [LICENSE-MIT](../../LICENSE-MIT)、[LICENSE-APACHE](../../LICENSE-APACHE))。
