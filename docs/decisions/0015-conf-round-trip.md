# 0015 conf との往復は conf だけで閉じ、正規形どうしを文字列で比べる

- 状態: 採用(案)
- 日付: 2026-11-15
- 補うもの: [spec/metamodel.md](../../spec/metamodel.md) §9・§10・§11

## 背景

roadmap の Step 7 として、juiz の conf と juiz-OPL・XML の間を往復する道具と往復テストを作る。題材はサンプル 4 リポジトリ(juiz_examples_rust・python・cpp・haskell)の conf で、4 つとも公開されていて 2026-11-15 の時点で計 56 本の conf がある([0010](0010-settle-without-dori-reply.md) の前提どおり)。

metamodel.md §10 は「引数名・引数の並び・データ型・既定値は manifest にあるので、conf から作るときは manifest か profile_full を併せて読む(Step 7)」としていた。manifest はプラグインのソース(Rust のマクロ、Python のデコレータなど)にあり、言語ごとに読み方が違う。

## 決めたこと

詳細は [spec/conf.md](../../spec/conf.md)。

1. **クレート juiz_opm(コマンド `juiz_opm`)を足す。** conf を読み(`from-conf`)、モデルから conf を書き(`to-conf`)、conf を正規形にし(`fmt-conf`)、J1〜J13 を検査し(`check`)、往復を確かめる(`round-trip`)。依存は opm_model、opm_opl、serde_json、serde_yaml。
2. **conf だけで閉じる。manifest は読まない。** つながっていない引数、引数の宣言順、readonly、データ型は conf から作るモデルに出さない(conf.md §8)。これらは Step 8 で動いている System の profile_full から書き出すときに埋める。profile_full は manifest を言語によらず JSON で返すので、言語ごとの読み方を作らずに済む。
3. **往復は conf の正規形どうしを文字列で比べる。** 正規形(conf.md §3)はネームスペースを解決し、並びを名前の順にし、参照の書き方をそろえた形。juiz にとって順が意味を持つもの(`bind`、Composite のノードの `args`、`composites` の定義順)は保つ。conf → モデル → conf、conf → OPL → conf、conf → XML → conf の 3 つを確かめる。
4. **conf から作るモデルの id は opm_opl に作らせる。** 一度 juiz-OPL の正規形に書いて読み直す。同じ conf からは常に同じモデル(同じ id)になり、OPL → conf → OPL も同じ文字列になる。
5. **OPM の名前は juiz の名前から機械的に作り、ぶつかれば role を括弧で足す**(conf.md §4.1)。metamodel.md §11 で Step 7 に残していた「同じ名前の Process と Container の分け方」はこれで決める。
6. **注釈の付け方を 4 点補う**(metamodel.md §9.3 に反映):
   - Composite の公開引数(value(input))に juiz:name で公開名を持たせる。引数名(juiz:arg)は子の引数名で、公開名と違うことがある(`expose: x`)。
   - Topic に juiz:namespace を持たせる。同じ名前の Topic がネームスペースごとに別にある(namespace.conf)。
   - 副系(role system)に juiz:type を持たせる(`subsystems` の `type_name`)。
   - juiz:params を EC、Transform、Composite、副系にも付ける。conf の決まったキー以外を往復させるため(Transform の `rate`・`timeout`、Container の 3D の `model` など)。
7. **別の System の Container は environmental の Object にする。** System をまたぐ Transform の始点(transformations_robot.conf)は ID で書かれ、この System の Container ではない。OPM の affiliation(系の外)がそのまま意味に合う。juiz:id に ID を持ち、J8(System が持つ)から外す。
8. **conf の description を Thing の description に写す。** metamodel.md §10 は description を往復しないものにしていたが、Composite の `description` は conf に書かれ、juiz の profile にも出るので往復させる。
9. **古い接続の書き方も読む。** ID の文字列で書いた端と、`connection_type` の代わりの `type`(juiz_examples_rust の opencv.conf)。`type` は今の juiz が読まない(push になる)ので、読むときに警告を出す。
10. **J10 はモデルだけからは検査しない。** プラグインの型名の一覧が要る(Step 8)。

## 理由

- manifest を読むと、4 言語のプラグインのソースを解析することになり、Step 8 の profile_full とやることが重なる。conf にあるものだけで往復が成り立つことを先に確かめておけば、Step 8 は「足りない注釈を足す」だけになる。
- 正規形を文字列で比べると、差が出たときにどこが違うかがそのまま見える。並べ替えを比べる側に持たせるより、正規形を 1 か所で決める方が、`fmt-conf` としても使える。
- id を opm_opl に作らせると、conf から作ったモデルと、その OPL を読んだモデルが同じ id になる。往復で id の差を気にしなくてよい。

## 影響

- crates/juiz_opm、spec/conf.md、tests/conf(サンプル 4 リポジトリの conf と、そこから書いた juiz-OPL)、tests/juiz/invalid(J の規則ごとの誤りの例)を足した。
- examples/opl/talker_listener.opl は juiz_examples_python の talker_listener.conf から書いたものと一字一句同じ(テストで確かめる)。
- CI は conf から書いた XML も XSD に通す。
- roadmap の Step 7 を済みにし、manifest で埋める注釈を Step 8 に回した。
