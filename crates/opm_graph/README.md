# opm_graph

OPM のモデル(交換形式の XML か juiz-OPL)を図にする。OPD ごとに 1 枚の Graphviz の DOT を書き、`dot` コマンドで SVG や PNG にする。

- 図の描き方: [spec/diagram.md](../../spec/diagram.md)
- 決定: [docs/decisions/0013](../../docs/decisions/0013-opm-graph-crate.md)

今(Step 6a)は描く範囲(§2)、OPM の形(§3.11)、実装状況の色(§3.10)を描く。juiz の略式の記号(§3.1〜§3.9)は Step 6b で足す。

```rust
let model = opm_graph::read_model("examples/xml/coffee_making.opm.xml".as_ref())?;
for picture in opm_graph::draw(&model) {
    let svg = opm_graph::svg::to_svg(&picture.dot)?; // Graphviz の dot を呼ぶ
    std::fs::write(format!("{}.svg", picture.name), svg)?;
}
```

コマンド(SVG と PNG には [Graphviz](https://graphviz.org/) が要る):

```sh
cargo run -p opm_graph -- examples/xml/talker_listener.opm.xml              # DOT を標準出力に
cargo run -p opm_graph -- -T svg -o out/ examples/xml/coffee_making.opm.xml  # SD, SD1, SD2 の 3 枚
cargo run -p opm_graph -- -T png --diagram SD1 -o sd1.png examples/xml/tutorial.opm.xml
```

## ライセンス

MIT OR Apache-2.0(リポジトリの [LICENSE-MIT](../../LICENSE-MIT)、[LICENSE-APACHE](../../LICENSE-APACHE))。
