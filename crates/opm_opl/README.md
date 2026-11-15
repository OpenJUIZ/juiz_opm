# opm_opl

juiz-OPL(juiz_opm の英語の OPL)を読み、検査し、正規形で書き出し、交換形式の XML と相互に変換する。

- 言語: [spec/opl.md](../../spec/opl.md)(この実装の決めごとは §15)
- モデルの型と XML: [opm_model](../opm_model/)
- 決定: [docs/decisions/0012](../../docs/decisions/0012-opm-opl-crate.md)

```rust
let read = opm_opl::check(&std::fs::read_to_string("examples/opl/capture_filter.opl")?);
for d in &read.diagnostics {
    println!("{}", d); // 25:1: warning[C8] main_loop_ec_cycle: ...
}
let text = opm_opl::write(&read.model, opm_opl::Options::default()).text;
```

コマンド:

```sh
cargo run -p opm_opl -- check examples/opl/capture_filter.opl
cargo run -p opm_opl -- fmt --check examples/opl/*.opl
cargo run -p opm_opl -- to-xml examples/opl/tutorial.opl
cargo run -p opm_opl -- from-xml --ids examples/xml/coffee_making.opm.xml
```

juiz の規則(J1〜J13)は juiz_opm クレート(Step 7)が見る。

## ライセンス

MIT OR Apache-2.0(リポジトリの [LICENSE-MIT](../../LICENSE-MIT)、[LICENSE-APACHE](../../LICENSE-APACHE))。
