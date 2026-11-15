# サンプル 4 リポジトリの conf

Step 7 の往復テスト([spec/conf.md](../../spec/conf.md) §7)の題材。各リポジトリの conf を写したもの。

| ディレクトリ | 元 | コミット |
| --- | --- | --- |
| juiz_examples_rust | [OpenJUIZ/juiz_examples_rust](https://github.com/OpenJUIZ/juiz_examples_rust) | 2ebb57b(2026-11-15) |
| juiz_examples_python | [OpenJUIZ/juiz_examples_python](https://github.com/OpenJUIZ/juiz_examples_python) | 00b12d7(2026-11-15) |
| juiz_examples_cpp | [OpenJUIZ/juiz_examples_cpp](https://github.com/OpenJUIZ/juiz_examples_cpp) | dfd67dd(2026-11-15) |
| juiz_examples_haskell | [OpenJUIZ/juiz_examples_haskell](https://github.com/OpenJUIZ/juiz_examples_haskell) | 724f5de(2026-11-15) |

- `packages/<名前>/app.conf` は `packages__<名前>__app.conf` にした。
- `<名前>.opl` は `juiz_opm from-conf` で書いた juiz-OPL。テストはこれと一字一句同じものが出ることを確かめる。conf を写し直したら `JUIZ_OPM_BLESS=1 cargo test -p juiz_opm` で書き直し、差分を見てから入れる。
- conf のライセンスは元のリポジトリと同じ(MIT OR Apache-2.0、ysuga)。
