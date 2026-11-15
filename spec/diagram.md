# 略式図

- 状態: 案(2026-11-15)。決めたことは [docs/decisions/0011](../docs/decisions/0011-abbreviated-diagram.md) に記録した
- 関連: [metamodel.md](metamodel.md)(図が写すモデル、§7.2 の OPD と Appearance)、[opm-mapping.md](opm-mapping.md) §5(実装状況の色)
- 見本: [diagram/](diagram/)(Step 6 の前に手で描いたもの)

## 1. 目的と範囲

juiz のモデルを人が読む図の描き方を決める。図は 1 種類で、これを略式図と呼ぶ。

- モデル(XML、juiz-OPL)は完全な OPM のまま持つ。略式図はモデルから決まった規則で描く。図からモデルは作らない。
- 決まった形の要素の組(§3)は 1 つの記号にまとめて描く。組にならない Thing とリンクは OPM の形で描く(§3.11)。juiz の注釈のないモデルは組を持たないので、OPM の図になる。
- 何を描くかは図の作成者が選ぶ(§2)。
- 配置は描く道具が決める。OPD の Appearance に layout があれば、それに従ってよい。

## 2. 描く範囲

1. 図は OPD 1 つにつき 1 枚。OPD の木(metamodel §7.2)はそのまま図の木になる。
2. モデルに OPD がなければ、全部を 1 枚に描く。
3. OPD があれば、その Appearance に載せた Thing だけを描く。§3 の記号は、表の「描く条件」が満たされるときに描く。
4. リンクは、両端が描かれていて、OPD の `suppressed_links` に挙げられていないときに描く。§3 の記号にまとめられたリンクは、その記号の条件に従う。
5. Appearance の `suppressed_states` に挙げた State は描かない。
6. 省くのは作成者の自由である。ただし、描いたものはモデルどおりでなければならない。§3 の記号は、その組がモデルに揃っているときだけ使う。揃っていない組は、記号にまとめずに OPM の形で描く。
7. 省いたことを図に示す記号は持たない。
8. リンクの端の Thing が描かれていなくても、それを中に持つ InZoom の refinee が描かれていれば、端をその refinee に移して描く(上の階層の図の「まとめた線」、metamodel §7.1)。描かれている最も内側の refinee に移し、同じになった線は 1 本にし、両端が同じ refinee になった線は描かない。状態を指定した effect は、端を移したら状態を外す([0013](../docs/decisions/0013-opm-graph-crate.md))。
9. `suppressed_states` の State を端に取るリンクは、その Object に付けて描く。

例: [examples/xml/tutorial.opm.xml](../examples/xml/tutorial.opm.xml) の SD は Client から inc0 への call を `suppressed_links` で省き、SD1 は counter0 とその ContainerProcess だけを描く(System の枠と引数・戻り値のポートも省く)。

## 3. 記号

`P` は Process、`V` は role value の Object。「まとめるもの」はその記号が表すモデルの要素で、記号を描くときは個別には描かない。

### 3.1 Process とポート

| 記号 | まとめるもの | 描く条件 |
| --- | --- | --- |
| 楕円。中に名前、下に `juiz:type` | role process・container_process・node・composite・ec_cycle 以外の Process | P が出ている |
| 入力ポート: 楕円の左の輪郭の小さな丸。中に引数名 | V(input)か、接続の源の V(output)・topic・pose からの instrument(`juiz:arg` のあるもの) | P が出ていて、V(input)なら V も出ている |
| 出力ポート: 楕円の右端の小さな黒い三角。横に型 | P から V(output)への result | P と V が出ている |

- 入力ポートは `juiz:arg_index` の順に上から並べる。引数が増えると楕円は縦に伸び、横幅は名前で決める。
- 入力ポートの丸は、接続・Topic・Pose 通知の線が来ていれば黒、自分だけの V(input)なら白にする。
- ポートの注記は、丸から細い線でぶら下げる。既定値 `juiz:default` は灰色の `= 値`、`juiz:applied`(p_apply)は茶色の `← 値 (p_apply)`。
- 出力ポートの型は V の `juiz:datatype` の最後の部分(`juiz.Image` なら `Image`)。

### 3.2 接続

| 記号 | まとめるもの | 描く条件 |
| --- | --- | --- |
| 出力ポートから入力ポートへの線。push は太線、pull は細線。ラベルは `push (connection_name)` | P1 の V(output)から P2 への instrument(`juiz:connection` のあるもの)と、push なら P1 から P2 への invocation | P1・P2・V が出ていて、instrument が `suppressed_links` にない |

### 3.3 Container

| 記号 | まとめるもの | 描く条件 |
| --- | --- | --- |
| 太枠の箱。上の帯に名前、`juiz:type`、`juiz:params` | role container・composite_container の Object | Object が出ている |
| 箱の中の楕円。名前の前に ✎(書き込み)か 👁(readonly) | 持ち主への effect か instrument を持つ container_process(J5)と、そのリンク | container_process が出ている |
| 名前の前の ⚓ と周期 | `juiz:type` が `periodic_anchor` / `static_anchor` の Container と `juiz:rate` | Container が出ている |

### 3.4 EC

| 記号 | まとめるもの | 描く条件 |
| --- | --- | --- |
| 角の丸い箱。名前、`juiz:type`、中に State を並べ、initial を太枠にする | role ec の Object と State | ec が出ている |
| started の State から bind 先への破線。ラベルは `periodically invokes · 周期`、bind 先が 2 つ以上なら `#order` を足す | ec_cycle、started からの condition の instrument、ec_cycle の自分への invocation、bind 先への invocation | ec と bind 先が出ていて、ec_cycle が自分を invocation で起こす(Timer 系、main_loop_ec など) |
| started の State から bind 先への破線。ラベルは `invokes once`、注記 `→ stopped` | 上と同じで、自分への invocation の代わりに ec を stopped に戻す effect | ec と bind 先が出ていて、ec_cycle が ec を started から stopped へ変える(one_shot_ec) |

「periodically invokes」は OPM にない関連(State は Process を起こせない)なので、図の記号だけに使い、モデルには持たない。

### 3.5 System

| 記号 | まとめるもの | 描く条件 |
| --- | --- | --- |
| 角の丸い枠。左上に名前と `system` | role system の Object と、そこからの aggregation・exhibition | system が出ている |

副系(system から system への aggregation)は枠の入れ子にする。

### 3.6 Transformations

| 記号 | まとめるもの | 描く条件 |
| --- | --- | --- |
| 始点の Container から終点の Container への太い青の線。中ほどに向きを示す白抜きの三角を置き、矢じりは付けない。ラベルは translation(回転があれば rotation も) | role transform の Object、`juiz:end` の付いた tagged 2 本 | transform と両端の Container が出ている |
| Container から Process の入力ポートへの破線と矢じり。ラベルは `pose` と `ref: reference`、`initial`、`on_update` | role pose の Object、Container からの exhibition、pose から P への instrument(event) | pose、Container、P が出ていて、instrument が `suppressed_links` にない |

Transform の木を同じ図に入れるか、別の OPD に分けるかは作成者が選ぶ(§2)。

### 3.7 Topic

role topic の Object は OPM の形(箱)で描く。publish(result)は出力ポートから Topic への線、subscribe(event の instrument)は Topic から入力ポートへの破線にする。

### 3.8 Composite

role composite の Process は §3.1 の楕円で描く。InZoom の OPD(refinement `in_zoom`)では、中の node と value を §3.1・§3.2 で描く。

### 3.9 外からの call

role client(環境の Process)は破線の楕円で描き、invocation は `call` と書いた破線にする。

### 3.10 実装状況

`juiz:status` は [opm-mapping.md](opm-mapping.md) §5 の塗りの色で表す。まとめた記号は、まとめた要素のうち最も進んでいない状況の色にする。

### 3.11 それ以外

§3.1〜§3.9 の組にならない Thing とリンクは OPM の形で描く。Process は楕円、Object は箱(physical は太枠、environmental は破線)、State は Object の中の角の丸い箱。リンクの端は OPM の記号(instrument は白丸、agent は黒丸、effect は両矢印、result と consumption は矢印、invocation は稲妻か `invokes` のラベル)。構造リンクは線の端に三角を置く。

## 4. 見本

Step 6 の前に手で描いたもの。配置は Graphviz、形は SVG で描いた。

| 図 | 中身 |
| --- | --- |
| [diagram/tutorial.svg](diagram/tutorial.svg) | チュートリアル 3 章(tutorial.conf)。ポート、注記、Container と ContainerProcess、System の枠、外からの call |
| [diagram/talker_listener.svg](diagram/talker_listener.svg) | チュートリアル 4 章。push 接続、EC の periodically invokes |
| [diagram/many_args.svg](diagram/many_args.svg) | 引数が多い Process、p_apply の注記、pull 接続 |
| [diagram/transformations.svg](diagram/transformations.svg) | juiz の transformations.md §11 の conf。Anchor、Transform、Pose 通知 |

## 5. この文書で決めないこと

- 注記とラベルを線と重ならないように置く規則(Step 6)。
- System をまたぐ Transform(juiz の transformations.md §19)。メタモデルにまだ持たない。
- `suppressed_links` と Appearance を OPL に書く方法。今は書かない(図の層は OPL に書かない、opl.md §1)。
