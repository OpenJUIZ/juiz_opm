# juiz_opm

juiz の System の conf と、juiz の注釈を持つ OPM のモデル(juiz-OPL・交換形式の XML)を相互に変換する。動いている System からも書き出す。juiz の規則 J1〜J13 も検査する。

- 写し方と正規形: [spec/conf.md](../../spec/conf.md)
- 動いている System から: [spec/system.md](../../spec/system.md)
- 決定: [docs/decisions/0015](../../docs/decisions/0015-conf-round-trip.md)、[0016](../../docs/decisions/0016-system-snapshot.md)

```rust
let text = std::fs::read_to_string("tests/conf/juiz_examples_python/talker_listener.conf")?;
let (model, notes) = juiz_opm::model_of_conf(&text).map_err(|n| n.errors.join("\n"))?;
// notes.warnings: plugins や option など、モデルに持たないキー
let opl = opm_opl::write(&model, opm_opl::Options::default()).text;
let conf = juiz_opm::to_conf(&model).map_err(|e| e.join("\n"))?;   // 正規形の conf
assert!(juiz_opm::check(&model).is_empty());                        // J1〜J13
juiz_opm::round_trip(&text)?;                                       // conf → OPL / XML → conf
```

コマンド(`juiz_opm opl ...`・`draw`・`opx` で opm_opl・opm_graph も使える。全体の使い方は [docs/usage.md](../../docs/usage.md)):

```sh
cargo run -p juiz_opm -- from-conf tests/conf/juiz_examples_rust/control_loop.conf > control_loop.opl
cargo run -p juiz_opm -- from-conf --xml tests/conf/juiz_examples_rust/control_loop.conf -o control_loop.opm.xml
cargo run -p juiz_opm -- to-conf examples/opl/tutorial.opl          # OPL か XML から conf
cargo run -p juiz_opm -- fmt-conf my_system.conf                    # conf の正規形
cargo run -p juiz_opm -- check examples/opl/capture_filter.opl      # コアの規則と J1〜J13
cargo run -p juiz_opm -- round-trip tests/conf/*/*.conf

# 動いている System から(spec/system.md)
cargo run -p juiz_opm -- fetch http://127.0.0.1:8000 -o snapshot.json       # スナップショットを取る
cargo run -p juiz_opm -- from-system --name talker_listener snapshot.json   # juiz-OPL(--xml で XML、--conf で conf)
cargo run -p juiz_opm -- from-conf --system snapshot.json talker_listener.conf
cargo run -p juiz_opm -- check --system snapshot.json talker_listener.opl   # J10 も検査する
```

conf だけから作るモデルには、conf に無いもの(つながっていない引数、引数の順、readonly、データ型、juiz:id)は出さない。動いている System のスナップショットを併せると埋まる(spec/system.md §4)。

## ライセンス

MIT OR Apache-2.0(リポジトリの [LICENSE-MIT](../../LICENSE-MIT)、[LICENSE-APACHE](../../LICENSE-APACHE))。
