# juiz_opm

juiz の System の conf と、juiz の注釈を持つ OPM のモデル(juiz-OPL・交換形式の XML)を相互に変換する。juiz の規則 J1〜J13 も検査する。

- 写し方と正規形: [spec/conf.md](../../spec/conf.md)
- 決定: [docs/decisions/0015](../../docs/decisions/0015-conf-round-trip.md)

```rust
let text = std::fs::read_to_string("tests/conf/juiz_examples_python/talker_listener.conf")?;
let (model, notes) = juiz_opm::model_of_conf(&text).map_err(|n| n.errors.join("\n"))?;
// notes.warnings: plugins や option など、モデルに持たないキー
let opl = opm_opl::write(&model, opm_opl::Options::default()).text;
let conf = juiz_opm::to_conf(&model).map_err(|e| e.join("\n"))?;   // 正規形の conf
assert!(juiz_opm::check(&model).is_empty());                        // J1〜J13
juiz_opm::round_trip(&text)?;                                       // conf → OPL / XML → conf
```

コマンド:

```sh
cargo run -p juiz_opm -- from-conf tests/conf/juiz_examples_rust/control_loop.conf > control_loop.opl
cargo run -p juiz_opm -- from-conf --xml tests/conf/juiz_examples_rust/control_loop.conf -o control_loop.opm.xml
cargo run -p juiz_opm -- to-conf examples/opl/tutorial.opl          # OPL か XML から conf
cargo run -p juiz_opm -- fmt-conf my_system.conf                    # conf の正規形
cargo run -p juiz_opm -- check examples/opl/capture_filter.opl      # コアの規則と J1〜J13
cargo run -p juiz_opm -- round-trip tests/conf/*/*.conf
```

conf から作るモデルには、conf に無いもの(つながっていない引数、引数の順、readonly、データ型)は出さない。これらは Step 8 で動いている System の profile_full から埋める(spec/conf.md §8)。

## ライセンス

MIT OR Apache-2.0(リポジトリの [LICENSE-MIT](../../LICENSE-MIT)、[LICENSE-APACHE](../../LICENSE-APACHE))。
