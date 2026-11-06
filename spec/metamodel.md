# OPM メタモデル

- 状態: 案(2026-11-06、Step 1)。決めたことは [docs/decisions/0005](../docs/decisions/0005-metamodel-layers.md) に記録した
- 関連: [opm-mapping.md](opm-mapping.md)(juiz と OPM の対応)、[xml.md](xml.md)(この表を写した XML、Step 2)、[opl.md](opl.md)(この表を写した juiz-OPL、Step 3)、[0004](../docs/decisions/0004-xml-interchange.md)(交換形式)、[OPCAT の OPX](../docs/research/opcat-opx.md)

## 1. 目的と範囲

モデル情報として「何を持つか」を、書き方(XML、OPL、図)から切り離して表で決める。Step 2 の XML と XSD、Step 3 の juiz-OPL の文法、Step 4 の opm_model の型は、どれもこの表を写したものにする。

メタモデルは 3 つの層に分ける。

| 層 | 中身 | 使う道具 |
| --- | --- | --- |
| コア | ISO 19450 の部分集合。Thing、State、リンク、in-zooming | opm_model、opm_opl。juiz の外でも使う |
| 図 | OPD の木と、各 OPD に何を出すか。配置は任意 | opm_graph、OPX の書き出し |
| juiz 拡張 | コアの要素に付ける注釈(`juiz:` で始まる名前)と、juiz の整形式の規則 | juiz_opm |

拡張の注釈をすべて捨てると、コアの整形式を満たす素の OPM モデルになる(§9.1)。

この文書の名前(`object`、`consumption`、`juiz:role` など)はメタモデルの名前で、XML の要素名や OPL の語はそれぞれの Step で決める。

## 2. 全体像

```text
Model
├─ Thing ─┬─ Object ── State(並び)
│         └─ Process
├─ Link ──┬─ 手続きリンク(consumption, result, effect, agent, instrument, invocation, exception)
│         └─ 構造リンク(aggregation, exhibition, generalization, classification, tagged)
├─ Fan(XOR / OR の束ね)
├─ InZoom(Thing の中身と順序)
└─ OPD(図の木。中に Appearance)
```

モデルの事実は Thing・State・Link・Fan・InZoom が持つ。OPD は「どの図に何を出すか」だけを持ち、モデルの事実を足さない。

## 3. 共通の決めごと

### 3.1 表の読み方

| 書き方 | 意味 |
| --- | --- |
| 1 | ちょうど 1 つ(必須) |
| 0..1 | 省略してよい |
| * | 0 個以上 |
| [ ] | 順序に意味がある並び |
| ref(X) | 種類 X の要素の id |

### 3.2 id と名前

| 項目 | 決めごと |
| --- | --- |
| id | すべての要素(Thing、State、Link、Fan、InZoom、OPD、Appearance)が持つ。モデルの中で一意。読み書きで変えない。形は文字列で、XML では `[A-Za-z_][A-Za-z0-9_.-]*`([xml.md](xml.md) §4) |
| name | Thing と State が持つ。Thing の名前はモデルの中で一意、State の名前は持ち主の Object の中で一意 |
| description | Thing、State、Link が 0..1 で持つ自由な説明 |

id は名前から作らなくてよい。名前を変えても id は変わらない。RDF に写すときは id を URI の末尾に使う([0004](../docs/decisions/0004-xml-interchange.md))。

## 4. Thing

### 4.1 共通の属性

| 属性 | 多重度 | 値 | 既定 | ISO 19450 の用語 |
| --- | --- | --- | --- | --- |
| id | 1 | 文字列 | | |
| kind | 1 | `object` / `process` | | thing |
| name | 1 | 文字列 | | name |
| essence | 1 | `informatical` / `physical` | `informatical` | essence |
| affiliation | 1 | `systemic` / `environmental` | `systemic` | affiliation |
| description | 0..1 | 文字列 | | |

ISO の perseverance(static / dynamic)は kind で決まる(object は static、process は dynamic)ので持たない。

### 4.2 Object だけが持つもの

| 属性 | 多重度 | 値 | 意味 |
| --- | --- | --- | --- |
| states | [*] | State | 取りうる状態。並びは表示と OPL の順 |

属性の値(「Rate is 30.」)は、OPM の流儀どおり属性 Object の State として持つ。値そのものを Thing の属性にはしない。

### 4.3 Process だけが持つもの

| 属性 | 多重度 | 値 | 意味 |
| --- | --- | --- | --- |
| duration | 0..1 | min・expected・max と単位 | ISO の実行時間。juiz では使わないが、OPCAT との往復のために持つ |

## 5. State

| 属性 | 多重度 | 値 | 既定 | 意味 |
| --- | --- | --- | --- | --- |
| id | 1 | 文字列 | | |
| owner | 1 | ref(Object) | | 持ち主。State は 1 つの Object にだけ属する |
| name | 1 | 文字列 | | |
| initial | 1 | 真偽 | 偽 | 初期状態 |
| final | 1 | 真偽 | 偽 | 終状態 |
| default | 1 | 真偽 | 偽 | 既定の状態。Object ごとに高々 1 つ |
| description | 0..1 | 文字列 | | |

シミュレーションの「今の状態」は持たない(範囲外)。

## 6. リンク

### 6.1 共通の属性

| 属性 | 多重度 | 値 | 意味 |
| --- | --- | --- | --- |
| id | 1 | 文字列 | |
| kind | 1 | §6.2・§6.3 の名前 | |
| source | 1 | ref(Thing) か ref(State) | 始点 |
| target | 1 | ref(Thing) か ref(State) | 終点 |
| modifier | 0..1 | `event` / `condition` | 制御の修飾(手続きリンクのうち表で ○ のものだけ) |
| path | 0..1 | 文字列 | 経路ラベル。同じ Process に入る線と出る線の組を結ぶ |
| source_multiplicity | 0..1 | `1`・`0..1`・`*`・`2..*` などの文字列 | 始点の側の多重度 |
| target_multiplicity | 0..1 | 同上 | 終点の側の多重度 |
| description | 0..1 | 文字列 | |

State を端に取るリンク(状態を指定した consumption など)は、source か target に State の id を書く。そのリンクの Object はその State の owner とみなす。

### 6.2 手続きリンク

向きはモデルの source → target。図の矢印の向きと違うものがあるので注意。

| kind | ISO 19450 の名前 | source → target | modifier | 追加の属性 | OPL の例 | OPCAT の番号 | juiz |
| --- | --- | --- | --- | --- | --- | --- | --- |
| consumption | consumption link | Object / State → Process | ○ | | P consumes A. | 301(event は 310) | 使わない([0002](../docs/decisions/0002-open-points.md) の 2 で保留) |
| result | result link | Process → Object / State | | | P yields B. | 306 | 戻り値、publish |
| effect | effect link / input-output link pair | Process → Object | ○ | from_state 0..1、to_state 0..1 | P affects A. P changes A from s1 to s2. | 302 | 書き込み ContainerProcess |
| instrument | instrument link | Object / State → Process | ○ | | P requires A. | 303(event は 308、condition は 304) | 引数、readonly、subscribe(event) |
| agent | agent link | Object / State → Process | ○ | | H handles P. | 305 | 使わない(人が呼ぶ図だけ) |
| invocation | invocation link | Process → Process | | | P1 invokes P2. | 307 | push、EC の bind と周期、外からの call |
| overtime_exception | overtime exception link | Process → Process | | | | 309 | 使わない |
| undertime_exception | undertime exception link | Process → Process | | | | 309 | 使わない |

- 状態変化は 1 本の effect に `from_state` と `to_state` を付けて持つ(ISO の input link と output link の組を 1 本にまとめる)。どちらかだけでもよい。1 つの文(P changes A from s1 to s2.)が 1 本のリンクに対応する。
- 状態を指定した consumption・result・instrument・agent は、source / target に State を書く。
- invocation は source と target が同じ Process でもよい。ISO 19450:2024 §9.5.2.5.2 の self-invocation link(図では戻りの矢印、OPL は「P invokes itself.」)で、繰り返し(ループ)はこれで表す。別の kind にはせず、source と target が同じことで見分ける。
- OPCAT は overtime と undertime を区別せず 309 にまとめている。書き出すときの扱いは Step 9 で決める。

### 6.3 構造リンク

| kind | ISO 19450 の名前 | source → target | 端の種類 | 追加の属性 | OPL の例 | OPCAT の番号 | juiz |
| --- | --- | --- | --- | --- | --- | --- | --- |
| aggregation | aggregation-participation | 全体 → 部分 | 両端とも Object か、両端とも Process | ordered 0..1(真偽) | W consists of A and B. | 204 | System の中身、CompositeContainer のスロット |
| exhibition | exhibition-characterization | 持ち主 → 特徴 | 何でも | | A exhibits X. | 202 | System の Process、Container の Pose |
| generalization | generalization-specialization | 一般 → 特殊 | 両端とも同じ kind | | B is an A. | 201 | 使わない(型は注釈、§9.3) |
| classification | classification-instantiation | 型 → 実体 | 両端とも同じ kind | | b is an instance of B. | 203 | 使わない(型は注釈、§9.3) |
| tagged | tagged structural link | 始点 → 終点 | 何でも | tag 1、backward_tag 0..1 | A relates to B. | 205(backward_tag ありは 206) | Transform |

- tagged で backward_tag があるものは双方向(ISO の bidirectional / reciprocal)。backward_tag が tag と同じなら reciprocal。
- OPCAT の relation の source / destination の向きが全体・部分のどちらかは、Step 9 で例のファイルを見て確かめる。

### 6.4 Fan(論理の束ね)

同じ Process の同じ側に付く複数の手続きリンクを、XOR か OR で束ねる。束ねないリンクは AND(全部)とみなす。

| 属性 | 多重度 | 値 |
| --- | --- | --- |
| id | 1 | 文字列 |
| operator | 1 | `xor` / `or` |
| links | 2 以上 | ref(Link) の集合。kind が同じで、Process 側の端が同じであること |

juiz では使わない。OPCAT と OPCloud のモデルを読めるように持つ。

## 7. 図の階層

### 7.1 InZoom(モデルの事実)

in-zooming は「Thing の中身と、中の Process の順序」というモデルの事実なので、図の層ではなくコアに置く。unfolding は既にある構造リンクを別の図に出すだけなので、InZoom は作らない(§7.2 の OPD だけ)。

| 属性 | 多重度 | 値 | 意味 |
| --- | --- | --- | --- |
| id | 1 | 文字列 | |
| refinee | 1 | ref(Thing) | 中身を開く Thing。1 つの Thing に InZoom は高々 1 つ |
| steps | [*] | ref(Process) の集合の並び | 中の Process の実行順。同じ集合の中は並行。refinee が Object のときは空 |
| parts | * | ref(Thing) | 中に置く Object と、refinee が Object のときの部分 |

- 中の Thing(steps と parts)は refinee の部分である。aggregation のリンクは別に持たない(InZoom から導く)。
- 中の Thing は 1 つの InZoom にだけ属する。どの InZoom にも属さない Thing は最上位にある。
- 外の Object と refinee の間の線は、中の Process に付けた本当の端で持つ。上の階層の図に出す「まとめた線」は持たず、描くときに導く(導く規則は Step 6)。同じ事実を 2 か所に持たないためである。

### 7.2 OPD(図)

| 属性 | 多重度 | 値 | 意味 |
| --- | --- | --- | --- |
| id | 1 | 文字列 | |
| name | 1 | 文字列 | SD、SD1、SD1.1 のような名前 |
| parent | 0..1 | ref(OPD) | なければ根(SD)。根は 1 つ |
| refinement | 1 | `root` / `in_zoom` / `unfold` / `view` | この図の作り方 |
| refinee | 0..1 | ref(Thing) | in_zoom・unfold のとき必須。in_zoom なら InZoom を持つ Thing |
| appearances | * | Appearance | この図に出すもの |

Appearance:

| 属性 | 多重度 | 値 | 意味 |
| --- | --- | --- | --- |
| id | 1 | 文字列 | |
| thing | 1 | ref(Thing) | 1 つの Thing は複数の OPD に現れてよい |
| suppressed_states | * | ref(State) | この図で隠す State |
| layout | 0..1 | x・y・幅・高さ | 配置。なければ描く道具が決める |

- リンクの Appearance は持たない。両端が出ている図には、そのリンクを出す。
- OPD とその配置は往復の範囲外(§10)。juiz の道具は OPD を自動で作るが、OPCAT や OPCloud から読んだ図は保つ。

## 8. コアの整形式の規則

| 番号 | 規則 |
| --- | --- |
| C1 | id はモデルの中で一意。ref はすべて実在する要素を指す |
| C2 | Thing の名前はモデルの中で一意。State の名前は owner の中で一意 |
| C3 | 手続きリンクは表 §6.2 の向きと端の種類に従う。Process と Object(か State)を結ぶものは、片方が Process で片方が Object か State |
| C4 | modifier は表 §6.2 で ○ の kind にだけ付く |
| C5 | effect の from_state・to_state は target の Object の State |
| C6 | 構造リンクは表 §6.3 の端の種類に従う |
| C7 | default が真の State は Object ごとに高々 1 つ |
| C8 | (警告)Process は、InZoom を持つなら中の Process を含めて、少なくとも 1 本の consumption・result・effect を持つ。ISO は何も変えない Process を認めないが、main_loop_ec の周期の Process(ec_cycle)や client のように起こすだけの Process があるので、誤りにはせず警告にする |
| C9 | InZoom の steps と parts は重ならず、中の Thing は 1 つの InZoom にだけ属する。InZoom の入れ子は循環しない |
| C10 | OPD の木は根が 1 つで循環しない。in_zoom の OPD の refinee は InZoom を持つ |
| C11 | Fan の links は 2 本以上で、kind と Process 側の端が同じ |
| C12 | generalization と classification は循環しない |

## 9. juiz 拡張

### 9.1 方針

1. **標準で表せることは標準で表す。** readonly は instrument、書き込みは effect、Composite は InZoom で表し、同じことを注釈に重ねて書かない。
2. **注釈は標準で表せないことだけ。** pull・push の別、引数名、型名、Topic であること、実装状況など。
3. **注釈を捨てても素の OPM として正しい。** 注釈を全部落としたモデルはコアの整形式(§8)を満たす。
4. **標準と注釈が両方持つ事実は、一致を規則で縛る。** push は「注釈の push」と「invocation」の両方に出る(§9.5)。どちらかだけのモデルは juiz として整形式でない。

### 9.2 役割 `juiz:role`

juiz の要素になる Thing には役割を付ける。役割のない Thing は juiz の外のもの(説明のための Object など)で、conf には写らない。

| juiz:role | kind | essence | affiliation | juiz の概念 | conf のどこか |
| --- | --- | --- | --- | --- | --- |
| system | object | informatical | systemic | System | ファイル全体、`subsystems:` |
| container | object | 部品なら physical | systemic | Container(Anchor を含む) | `containers:` |
| composite_container | object | informatical | systemic | CompositeContainer | `composite_containers:` |
| topic | object | informatical | systemic | Topic | `publishes` / `subscribes` の名前 |
| value | object | informatical | systemic | 引数・戻り値・Composite の中の値 | 書かない(manifest から) |
| transform | object | informatical | systemic | Transform | `transforms:` |
| pose | object | informatical | systemic | ContainerPose | 書かない(pose_listener があるときだけ現れる) |
| slot | object | informatical | systemic | Composite の型のスロット | `composites[].containers` |
| process | process | informatical | systemic | Process | `processes:` |
| container_process | process | informatical | systemic | ContainerProcess、CompositeContainerProcess | `containers[].processes`、`composite_containers[].processes` |
| composite | process | informatical | systemic | Composite の型 | `composites:` |
| node | process | informatical | systemic | Composite の式の木のノード | `composites[].body` |
| ec | object | informatical | systemic | EC。State は `stopped` と `started` | `ecs:` |
| ec_cycle | process | informatical | systemic | EC が bind した Process を起こす 1 回分の実行(タイマー) | 書かない(ec ごとに 1 つ) |
| client | process | informatical | environmental | 外から call する相手 | 書かない(図のためだけ) |

### 9.3 Thing に付ける注釈

| 注釈 | 付く役割 | 多重度 | 値 | 意味 |
| --- | --- | --- | --- | --- |
| juiz:id | system 以外の実体(container, composite_container, process, container_process, ec, transform) | 0..1 | juiz の ID 文字列 | `core://core/process/<ns>/<name>::<type>` など。conf から作るときは省略してよく、profile_full から作るときは必ず付ける |
| juiz:name | 実体、topic、slot | 1 | 文字列 | juiz の名前(ネームスペースを含まない)。OPM の name は表示用で、`robot1/arm` のようにネームスペース付きでもよい |
| juiz:namespace | 実体 | 0..1 | 文字列 | ネームスペース。省略は既定 |
| juiz:type | container, process, container_process, composite, node, ec, slot | 1 | type_name | 型名。composite では自分の型名(J10 はこれで照らし合わせる)。型は分類リンクではなくこの注釈で持つ([0002](../docs/decisions/0002-open-points.md) の 4)。Anchor は `periodic_anchor` / `static_anchor` |
| juiz:datatype | value | 0..1 | 型の完全名か、`<完全名>@sha256:<64 桁>`(juiz の datatype.md §5) | juiz.datatype の型。型なしなら省略 |
| juiz:value_kind | value | 1 | `output` / `input` / `intermediate` / `const` | 戻り値、つながっていない引数、Composite の中の値、Composite の定数 |
| juiz:value | value(value_kind が const のとき) | 1 | JSON の値 | 定数 |
| juiz:rate | ec, container(Anchor) | 0..1 | 数(Hz) | 周期 |
| juiz:use_memo | process, container_process | 0..1 | 真偽 | |
| juiz:path | node | 1 | 文字列 | 式の木の中の位置(`body.img` など、composition.md §6) |
| juiz:translation, juiz:rotation | transform | 0..1 | juiz_std.geometry.Transform の形 | 省略は 0 と回転なし |
| juiz:writers | transform | * | Container の名前か ID | `set_transform` で書いてよい Container |
| juiz:status | 何にでも(リンクにも) | 0..1 | `released` / `merged` / `in_progress` / `planned` / `defect` | 実装状況(opm-mapping.md §5)。省略は released |
| juiz:refs | 何にでも | * | `juiz#123` のような issue / PR の参照 | 実装状況の根拠 |

### 9.4 リンクに付ける注釈

| 注釈 | 付くリンク | 多重度 | 値 | 意味 |
| --- | --- | --- | --- | --- |
| juiz:arg | value・topic・pose から process への instrument | 1 | 引数名 | どの引数に入るか |
| juiz:arg_index | 同上 | 1 | 0 からの整数 | manifest の引数の並び。Composite の評価順を導くのに使う |
| juiz:connection | 別の Process の output の value からの instrument | 1 | `push` / `pull` | 接続の種類 |
| juiz:connection_name | 同上 | 0..1 | 文字列 | conf の接続の `name` |
| juiz:applied | instrument | 0..1 | JSON の値 | conf の `p_apply` で入れる値 |
| juiz:order | ec_cycle から bind 先への invocation | 1 | 0 からの整数 | `bind` の並び |
| juiz:slot_name | composite_container からの aggregation | 1 | 文字列 | スロット名 |
| juiz:reference | pose からの instrument(event) | 0..1 | Container の名前 | pose_listener の基準 |
| juiz:initial | 同上 | 0..1 | 真偽 | pose_listener の `initial` |
| juiz:end | transform からの tagged | 1 | `source` / `destination` | Transform の始点か終点か |

### 9.5 juiz の概念ごとの形

| juiz | コアでの形 | 注釈 |
| --- | --- | --- |
| Process | process に、戻り値の value(output)への result がちょうど 1 本 | role、name、type |
| 引数(つながっていない) | 自分だけの value(input)から instrument | juiz:arg、juiz:arg_index |
| 接続 | 送り手の value(output)から受け手への instrument。送り手と受け手の Process は直接つながない | juiz:connection、juiz:arg |
| push 接続 | 上に加え、送り手から受け手への invocation | 送り手・受け手の組に push の接続が 1 本以上あるときに限り invocation が 1 本ある |
| ContainerProcess | 持ち主の Container への effect(書き込み)か instrument(readonly)がちょうど 1 本 | readonly の注釈は持たない(線の種類で分かる) |
| Topic | publish は topic への result、subscribe は topic からの instrument(modifier が event) | subscribe に juiz:arg |
| EC | 状態機械とタイマーの組。ec(Object)は State `stopped` と `started` を持ち、`auto_start` が真なら `started`、偽なら `stopped` を initial にする。ec_cycle は `started` の ec を condition の instrument で使い、bind 先を invocation で起こす。main_loop_ec は ec_cycle が自分を invocation で起こす(周期は ec の juiz:rate)。one_shot_ec は ec_cycle が ec を `started` から `stopped` へ変える(effect、1 回で止まる) | ec に juiz:type・juiz:rate、invocation に juiz:order |
| System | system から Container・CompositeContainer・Topic・Transform への aggregation、EC への aggregation、Process・ContainerProcess への exhibition。副系は system から system への aggregation | |
| Composite の型 | process(role composite)の InZoom。steps は node の評価順で、1 段に 1 つ。parts は中の value と slot | 外に見える引数は value(input)、戻り値は value(output) |
| Composite のノード | node の戻り値は value(intermediate)への result、それを親の node が instrument で受ける。根の node は Composite の value(output)への result | `const` は value(const)からの instrument、`expose` は Composite の value(input)からの instrument |
| Composite のインスタンス | 普通の process / container_process で、juiz:type が Composite の型名 | 中身は型の InZoom を見る |
| CompositeContainer | composite_container からスロットの参照先の Container への aggregation | juiz:slot_name |
| Composite のスロットを使う子 | node から slot への effect か instrument | |
| Transform | transform から始点・終点の Container への tagged(tag は `source` / `destination`) | juiz:end、値 |
| pose_listener | Container の pose(exhibition で持つ)から Process への instrument(modifier が event) | juiz:arg、juiz:reference、juiz:initial |
| 外からの call | client から Process への invocation | 往復の範囲外 |

例(opm-mapping.md §4 の Capture と Filter)をメタモデルの要素で書くと次のとおり。id は説明のために名前に揃えた。

| 要素 | kind | 注釈 / 属性 |
| --- | --- | --- |
| Camera | object, physical | role=container, type=video_capture |
| Capture | process | role=container_process, type=video_capture_readandget |
| Filter | process | role=process, type=cv_cvt_color |
| Main Loop EC | object | role=ec, type=main_loop_ec, rate=10。State は stopped と started(initial) |
| Main Loop EC Cycle | process | role=ec_cycle |
| Image | object | role=value, value_kind=output |
| Edges | object | role=value, value_kind=output |
| Capture → Camera | effect | |
| Capture → Image | result | |
| Image → Filter | instrument | arg=img, arg_index=0, connection=push(Filter の 2 つ目の引数 code は省いた) |
| Capture → Filter | invocation | (push があるため) |
| Filter → Edges | result | |
| Main Loop EC の started → Main Loop EC Cycle | instrument | modifier=condition |
| Main Loop EC Cycle → Main Loop EC Cycle | invocation | (self-invocation。周期で自分を起こす) |
| Main Loop EC Cycle → Capture | invocation | order=0 |

### 9.6 juiz の整形式の規則

| 番号 | 規則 |
| --- | --- |
| J1 | role が process・container_process の Thing は value(output)への result を、node は value(intermediate)か、根の node なら Composite の value(output)への result を、ちょうど 1 本持つ。value は 2 つ以上の Process から result を受けない |
| J2 | role が process・container_process・node の Thing の引数ごとに、instrument が 1 本以上ある(源は value、topic、pose)。value(input)からのものは、その引数に他の源がないときだけで、value(input)は 1 本の instrument の源にだけなる。引数名と juiz:arg_index は 1 対 1 で、juiz:arg_index は 0 から隙間なく並ぶ |
| J3 | juiz:connection は、別の Process の value(output)からの instrument にだけ付き、そのときは必ず付く |
| J4 | 送り手 P1 と受け手 P2 の間の invocation は、P1 の output から P2 への push の接続が 1 本以上あるときに限り、ちょうど 1 本ある |
| J5 | container_process は、role が container か composite_container の Object への effect か instrument を、ちょうど 1 本持つ(持ち主) |
| J6 | topic への result と topic からの instrument(event)は、role が process・container_process の Thing にだけ付く |
| J7 | ec は State `stopped` と `started` をちょうど持ち、initial はどちらか 1 つ。ec_cycle は ec ごとにちょうど 1 つで、その ec の `started` からの instrument(modifier が condition)をちょうど 1 本持つ。ec_cycle からの invocation の先は自分か process か container_process で、bind 先への juiz:order は 0 から隙間なく並ぶ。juiz:type が main_loop_ec なら自分への invocation があり、one_shot_ec なら自分への invocation はなく、ec を `started` から `stopped` へ変える effect がある。他の型の EC はこの 2 つを縛らない |
| J8 | juiz の実体(container、composite_container、topic、transform、process、container_process、ec)は、role が system の Object を source とする aggregation か exhibition の target に、ちょうど 1 回なる(composite_container からの aggregation は数えない) |
| J9 | composite の InZoom の steps は node を 1 段に 1 つずつ並べ、その順は式の木を juiz:arg_index の順に深さ優先でたどった順(composition.md §5.1)と一致する |
| J10 | process・container_process の juiz:type が composite の型名なら、同じ juiz:type の role composite の Thing がモデルにある |
| J11 | transform は juiz:end が source と destination の tagged をちょうど 1 本ずつ持ち、両端は container。Transform の木の規則(transformations.md §5.2)を満たす |
| J12 | pose は container からの exhibition をちょうど 1 本持ち、pose_listener の instrument がなければ現れない |
| J13 | role のある Thing は consumption、agent、例外リンク、Fan を持たない |

## 10. 往復の範囲

juiz-OPL と juiz の構成の全単射(opm-mapping.md §7)は、次の範囲で確かめる。

| 区分 | 含むもの |
| --- | --- |
| 往復する | 役割のある Thing と、その間のリンク、§9.3・§9.4 の注釈のうち juiz:status・juiz:refs 以外 |
| 往復しない(モデルには持てる) | OPD と配置、役割のない Thing、client、juiz:status・juiz:refs、description、duration、Fan |
| 持たない | ブローカーと配置([0002](../docs/decisions/0002-open-points.md) の 3)、プロセスの起動オプション、プラグインのパス、.juizapp の配布情報、本体のコード |

conf のキーとの対応:

| conf のキー | メタモデル |
| --- | --- |
| `name` | role system の Object |
| `processes:` | role process の Thing、`p_apply` は juiz:applied、`publishes` / `subscribes` は topic へのリンク、`use_memo` は juiz:use_memo |
| `containers:`、その `processes` | role container の Object と role container_process の Thing、持ち主への effect か instrument |
| `connections:` | instrument(juiz:connection、juiz:arg、juiz:connection_name)と、push なら invocation |
| `ecs:` | role ec の Object(`auto_start` は initial の State)、role ec_cycle の Process、invocation(juiz:order) |
| `composites:` | role composite の Thing と InZoom、node・value・slot |
| `composite_containers:` | role composite_container の Object と aggregation(juiz:slot_name) |
| `transforms:` | role transform の Object と tagged 2 本 |
| `pose_listeners:` | role pose の Object と instrument(event) |
| `subsystems:` | system から system への aggregation |
| `plugins:`、`option:`、`brokers:` | 持たない |

引数名・引数の並び・データ型は conf になく manifest にあるので、conf から作るときはプラグインの manifest か profile_full を併せて読む(Step 7)。

## 11. この Step で決めないこと

- OPL の語と文の並び、名前の引用の仕方(Step 3)。juiz の名前(`talker0`)をそのまま OPL の名前に使えるか。→ [opl.md](opl.md) で決めた(引用すれば使える)。
- 上の階層の図の「まとめた線」を導く規則と、描画の形(Step 6)。
- 同じ名前の Process と Container がある System で OPM の名前をどう分けるか(Step 7。juiz:id があれば juiz 側は区別できる)。
- Topic のメッセージを consumption にするか([0002](../docs/decisions/0002-open-points.md) の 2。今は instrument の event)。
- Composite のスロットを持つ Composite を子にする形(juiz 側も未対応、composition.md §13)。
