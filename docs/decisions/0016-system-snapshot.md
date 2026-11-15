# 0016 動いている System はスナップショットにしてから書き出し、manifest にしかないものを足す

- 状態: 採用(案)
- 日付: 2026-11-15
- 補うもの: [0015](0015-conf-round-trip.md) の 3(Composite のノードの `args` の順)と 10(J10)を置き換える

## 背景

roadmap の Step 8 として、動いている System の profile_full からモデルを書き出し、conf には無いもの(つながっていない引数、引数の順、readonly、データ型、juiz:id)を埋め、J10 を検査する([0015](0015-conf-round-trip.md) の 2、spec/conf.md §8)。

juiz を手元でビルドし、juiz_examples_rust と juiz_examples_python のサンプルを動かして HTTP の API を調べた。分かったこと:

- `GET /api/system/profile_full` には Process・Container・ContainerProcess・EC・Topic・ファクトリ(manifest)があるが、接続・Composite の一覧・Transform・PoseListener は無い。それぞれ `/<種類>/list` と `/<種類>/profile_full?identifier=` で取れる。
- 接続の一覧には、`publishes`・`subscribes` の配線(端が Topic)も出る。
- System の名前、params、`p_apply`、接続の名前は profile に出ない。
- juiz は Composite のノードを、conf の `args` の順ではなく子の manifest の引数の順で呼び、conf に書いてない引数は引数名のまま公開する(juiz_core の composite_process_factory.rs の `compile`)。

## 決めたこと

詳細は [spec/system.md](../../spec/system.md)。

1. **書き出しは動いている System から直接ではなく、スナップショット(profile を 1 つにまとめた JSON)から行う。** `juiz_opm fetch <url>` が取り、`from-system` はファイルか URL を読む。テストはチェックインしたスナップショットで行い、juiz を動かさない。
2. **スナップショット → conf → モデルの順に作る。** conf の読み方(spec/conf.md §4)を 2 つ持たないため。モデルには manifest から引数の順・つながっていない引数・既定値・データ型・readonly・juiz:id を足す(system.md §4.1)。
3. **conf とスナップショットを併せて読める**(`from-conf --system`)。params や `p_apply` と manifest の両方を持つモデルになる。足したものは conf に写らないので、conf との往復はそのまま成り立つ。
4. **conf の正規形で、Composite のノードの `args` を名前の順にし、名前のとおりに公開した既定値の無い引数(`arg: {expose: arg}`)を書かない。** juiz にとってどちらも意味を持たない(背景の 4 つめ)。[0015](0015-conf-round-trip.md) の 3 で「意味を持つので保つ」としていたのを置き換える。conf だけから作るモデルの、ノードの引数の juiz:arg_index も名前の順になる。
5. **J10 は、スナップショットの Composite の型名で検査する**(`check --system`、`check_with_system`)。[0015](0015-conf-round-trip.md) の 10 を置き換える。
6. **スナップショットから分からないものは比べない。** スナップショットから書いた conf と元の conf は、params・`p_apply`・description・`use_memo`・接続の名前と、juiz が埋める既定値を除いて比べる(system.md §3.2)。
7. **HTTP には ureq を使う**(TLS なし、ローカルのブローカー向け)。

## 理由

- スナップショットにすると、取り直さない限り同じ結果になり、golden の OPL で比べられる。juiz を CI で動かすと、4 言語のプラグインのビルドが要る。
- conf を経ると、Topic の名前の解決、名前のぶつかり、Composite の木の作り方を conf と同じ 1 か所で済ませられる。
- ノードの `args` の順を保つと、conf に書いた順と juiz が呼ぶ順が違うのに、違う順の conf が別の正規形になる。juiz の動きに合わせて名前の順にした方が、スナップショットから書いた conf と比べられる。

## 影響

- crates/juiz_opm に `system`(スナップショット、conf への読み替え、`SystemInfo`)、`to_model_with`、`check_with_system`、コマンド `fetch`・`from-system`・`from-conf --system`・`check --system` を足した。依存に ureq を足した。
- tests/system に juiz_examples_rust の 12 本、juiz_examples_python の 4 本のスナップショットと、そこから書いた juiz-OPL を足した。cpp と haskell はプラグインのビルドが要るので後に回す。
- conf の正規形が変わったので、tests/conf の control_loop.opl と opencv_capture_oneshot_composite.opl の juiz:arg_index が変わった。
- CI はスナップショットから書いた XML も XSD に通す。
