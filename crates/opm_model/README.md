# opm_model

OPM(ISO 19450)のメタモデルの型と、juiz_opm の交換形式 XML の読み書き、コアの整形式の規則(C1〜C12)の検査。

- メタモデル: [spec/metamodel.md](../../spec/metamodel.md)
- XML: [spec/xml.md](../../spec/xml.md)(この実装の決めごとは §14)
- 決定: [docs/decisions/0009](../../docs/decisions/0009-opm-model-crate.md)

```rust
let model = opm_model::read::from_path("examples/xml/coffee_making.opm.xml")?;
let report = opm_model::check(&model);
for d in &report.diagnostics {
    println!("{}", d); // error[C5] l_heat: ...
}
let text = opm_model::write::to_string(&model);
```

juiz の注釈など他の名前空間の属性と要素は、解釈せずに `Extensions` として持ち、そのまま書き戻す。juiz の規則(J1〜J13)は juiz_opm クレート(Step 7)が見る。

手元で試す:

```sh
cargo run -p opm_model --example opm_xml -- check examples/xml/capture_filter.opm.xml
cargo run -p opm_model --example opm_xml -- fmt examples/xml/capture_filter.opm.xml
```

## ライセンス

MIT OR Apache-2.0(リポジトリの [LICENSE-MIT](../../LICENSE-MIT)、[LICENSE-APACHE](../../LICENSE-APACHE))。
