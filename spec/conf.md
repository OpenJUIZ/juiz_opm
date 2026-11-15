# juiz の conf との往復

- 状態: 案(2026-11-15、Step 7)。決めたことは [docs/decisions/0015](../docs/decisions/0015-conf-round-trip.md) に記録した
- 関連: [metamodel.md](metamodel.md) §9(juiz 拡張)・§10(往復の範囲)、[opm-mapping.md](opm-mapping.md) §7(全単射の確認)
- 道具: [crates/juiz_opm/](../crates/juiz_opm/)(コマンド `juiz_opm`)

juiz の System の conf(YAML。JSON も YAML として読む)と、juiz の注釈を持つ OPM のモデル(juiz-OPL・XML)の間の写像を決める。

## 1. 範囲

往復するものは metamodel.md §10 の表のとおり。conf だけから作るときに分からないもの(下の §8)は、モデルにも出さない。

| conf | 扱い |
| --- | --- |
| `name`、`namespace`、`processes`、`containers`、`composite_containers`、`composites`、`connections`、`ecs`、`transforms`、`pose_listeners`、`subsystems` | 往復する |
| `plugins`、`option`、`brokers`、`broker_proxies`、`mastersystem` | 持たない。読むときに警告を出す |
| それ以外の最上位のキー | 持たない。読むときに警告を出す |
| 各項目の決まったキー以外(`initial_value`、`dt`、`model`、`trace`、Transform の `rate`・`timeout` など) | `juiz:params` に入れて往復する |
| `connections` の決まったキー以外 | 持たない(リンクは params を持たない)。読むときに警告を出す |

## 2. 読み方

juiz が conf を読むのと同じように読み、名前を解決した形で持つ。

- **ネームスペース**(juiz の system_builder/namespaces.rs): 最上位の `namespace` は System の既定。各項目の `namespace` は、なければ既定、`/` で始まれば絶対、それ以外は既定の下。ContainerProcess は Container のネームスペースに置く。`connections` の `source`・`destination` と `ecs` の `bind` は、接続や EC のネームスペースで解決する。
- **Topic 名**(juiz の topic_identifier.rs): 書いた Process のネームスペースの下。`/` で始まれば絶対。
- **CompositeContainer のスロット**は CompositeContainer のネームスペースで、`transforms`・`pose_listeners` の名前は System の既定のネームスペースで解決する。Transform の端が `://` を含む ID なら、別の System の Container として ID のまま持つ。
- **Process の参照**は `{type_name, name, namespace, container_name}` のほか、ID の文字列、`id`・`identifier` を持つ mapping(ID は解決しない絶対の名前)、`source_process_id`・`destination_process_id` を受ける。ID は `<broker_type>://<broker>/<class>/<ns>/<name>::<type>[:<container>]` と読む。
- **接続の種類**は `connection_type`(大文字小文字を問わない)。省略は push。古い形の `type` は `connection_type` として読み、警告を出す(今の juiz は `type` を読まない)。
- `auto_start` の省略は偽(juiz の execution_context_holder_factory.rs)。

## 3. 正規形

conf を比べるときの形。`juiz_opm fmt-conf` が書く。juiz が読んで同じ System になる形に限る。

1. 最上位のキーは `name`、`composites`、`processes`、`containers`、`composite_containers`、`connections`、`ecs`、`transforms`、`pose_listeners`、`subsystems` の順。空の並びは書かない。§1 で持たないキーは書かない。最上位の `namespace` は書かず、各項目に解決したネームスペース(先頭の `/` なし、ルートなら書かない)を書く。
2. 並びの順:
   - `processes`・`containers`・`composite_containers`・`ecs` はネームスペース、名前の順。Container の `processes` は名前の順。
   - `composites` は、使う Composite を先に(juiz は先に定義した Composite しか子に使えない)、その他は型名の順。
   - `connections` は受け手、引数名、送り手、接続名の順。`transforms` は始点、終点の順。`pose_listeners` は Container、Process、引数名の順。`subsystems` は型名、名前の順。
   - `publishes`・`subscribes`・`p_apply`・スロットは名前の順。
   - `bind` の順、Composite のノードの `args` の順、params のキーの順は保つ(前の 2 つは意味を持つ)。
3. 項目の中のキーの順は、`type_name`、`name`、`namespace`、`description`、その項目の決まったキー、params の順(Container と CompositeContainer は `processes` を最後)。
4. 書き方をそろえる:
   - Process の参照は `{type_name, name, namespace, container_name}`。ContainerProcess を指すときは `container_name` を必ず書く。
   - `connection_type` は `Push` / `Pull`。EC の `auto_start` は常に書く。
   - Topic 名とスロットの先は、書く項目のネームスペースからの相対(同じネームスペースなら名前だけ、違えば `/` 始まりの絶対)。
   - `rate` と `translation`・`rotation` の成分は浮動小数(`1.0`)で書く。`translation`・`rotation` は書いてあれば省いた成分を既定値(0、`w` は 1)で埋める。
   - `expose` は既定値がなければ名前だけ、あれば `{name, default}`。

## 4. conf → モデル

metamodel.md §9.5 の形にする。注釈の値は conf の値をそのまま写す。

### 4.1 OPM の名前

OPM の名前(表示用、OPL の Thing の名前)は次のように作る。`<full>` はネームスペース付きの名前(`robot1/talker0`)。

| 要素 | 名前 |
| --- | --- |
| System | conf の `name` |
| 副系(`subsystems`) | その `name` |
| Process、ContainerProcess、Container、CompositeContainer、EC | `<full>` |
| Process の戻り値 | `<Process の名前> out` |
| つながっていない引数(p_apply だけ) | `<Process の名前> <引数名>` |
| Topic | 解決した Topic 名 |
| EC の周期 | `<EC の名前> cycle` |
| Composite の型 | 型名 |
| Composite のノード | `<型名> <位置>`(`capture_save body.img`) |
| ノードの戻り値 | `<型名> <位置> out`。根は `<型名> out` |
| const の値 | `<型名> <位置>` |
| 公開引数(expose) | `<型名> <公開名>` |
| スロット | `<型名> <スロット名>` |
| ContainerPose | `<Container の名前> pose` |
| Transform | `<始点の名前>-<終点の名前> transform` |
| 別の System の Container | ID から取った `<full>` |

名前がぶつかったら、後から作る方に ` (<role>)` を付ける(さらにぶつかれば ` (<role>) 2`、`3`…)。作る順は副系、Composite、Process、Container とその ContainerProcess、CompositeContainer、Topic、EC、Transform、Pose。metamodel.md §11 の「同じ名前の Process と Container」はこれで分ける(juiz 側は juiz:name と role で区別できる)。

### 4.2 形

| conf | モデル |
| --- | --- |
| Process | role process。result で value(output)、System から exhibition |
| Container | role container。System から aggregation。Anchor の `rate` は juiz:rate |
| ContainerProcess | role container_process。持ち主への effect(readonly かどうかは conf に無いので、書き込みとする。§8)、System から exhibition |
| CompositeContainer | role composite_container。System から aggregation、スロットの先の Container へ aggregation(juiz:slot_name) |
| `publishes` | Topic(role topic、juiz:name・juiz:namespace)への result。Topic は名前ごとに 1 つで、System から aggregation |
| `subscribes` | Topic から event の instrument(juiz:arg) |
| `connections` | 送り手の value(output)から受け手への instrument(juiz:connection・juiz:arg・juiz:connection_name)。push なら送り手から受け手への invocation(組ごとに 1 本) |
| `p_apply` | その引数の instrument の 1 本目に juiz:applied。引数に他の源がなければ value(input)を作ってそこから instrument |
| `ecs` | role ec(State stopped・started、`auto_start` で initial)と role ec_cycle。started から condition の instrument、bind 先へ invocation(juiz:order)。main_loop_ec は自分への invocation、one_shot_ec は ec を started から stopped へ変える effect |
| `composites` | role composite の Process(juiz:type が型名)と InZoom。steps はノードを深さ優先(引数を先に)で 1 段ずつ。parts は中間の値、const の値、スロット。ノードは role node(juiz:type が子の型名、juiz:path)。`container` はスロットへの effect。公開引数は value(input)で、juiz:name に公開名、juiz:default に既定値 |
| `transforms` | role transform(juiz:translation・juiz:rotation・juiz:writers・params)。System から aggregation、始点・終点の Container へ tagged(tag と juiz:end が source / destination)。別の System の Container は environmental の role container(juiz:id に ID)で、System は持たない |
| `pose_listeners` | Container が exhibition で持つ role pose の Object(Container ごとに 1 つ)から Process への event の instrument(juiz:arg・juiz:reference・juiz:initial・juiz:on_update) |
| `subsystems` | role system(juiz:name、juiz:type、params)。System から aggregation |

引数の juiz:arg_index は、Process では出会った順(subscribes、connections、pose_listeners、p_apply の順)、Composite のノードでは `args` に書いた順にする。manifest の引数の順は conf に無い(§8)。

できたモデルは juiz-OPL の正規形に書いて読み直し、id は opm_opl が文の順で作るもの([0012](../docs/decisions/0012-opm-opl-crate.md) の 4)にする。同じ conf からは常に同じモデルになる。

参照の解決に失敗したら(どの Process も指さない bind、Container のないスロットなど)、誤りとしてモデルを作らない。

## 5. モデル → conf

§4 の逆。役割(juiz:role)と注釈とリンクの種類だけを見て、OPM の名前は見ない。だから手で名前を付けたモデル(examples/opl/tutorial.opl など)からも conf を作れる。

- 根の System は、他の role system から aggregation されていない role system の Thing。ちょうど 1 つでなければ誤り。
- System が aggregation・exhibition で持つものを role ごとに conf の項目にする。ContainerProcess は持ち主(effect の先か instrument の元の Container)の `processes` に入れる。Topic は Process の側から読む。
- Composite は role composite の Thing をすべて読む。根のノードは value(output)へ result するノード。
- 接続は juiz:connection を持つ instrument、pose_listeners は role pose からの instrument から読む。
- 書けない形(根の System がない、ContainerProcess の持ち主が 1 つでない、Transform の端がそろわない、params が JSON のオブジェクトでないなど)は誤りにする。

## 6. juiz の規則の検査

metamodel.md §9.6 の J1〜J13 を `juiz_opm check` が検査する。役割のある Thing だけを見る。

- 注釈の名前と値も見る(`error[juiz]`): 要素の種類(Thing、State、リンク)に付かない juiz の注釈と、role・value_kind・connection・end・status の範囲外の値(xml.md §8)。

- J8 は、environmental の role container(別の System の Container、§4.2)を数えない。
- J10 はプラグインの型名の一覧が要るので、モデルだけからは検査しない(Step 8 で profile_full を読むときに見る)。
- J11 は木の規則のうち「終点は 1 つの Transform の終点にしかならない」と「循環しない」を見る。

## 7. 往復テスト

`juiz_opm round-trip <conf>` と `cargo test -p juiz_opm` が確かめること:

1. conf → モデル → conf、conf → juiz-OPL → conf、conf → XML → conf が、どれも conf の正規形(§3)と一字一句同じ。
2. conf から作ったモデルが、コアの規則(C1〜C12)と juiz の規則(J1〜J13)に誤りを出さない。
3. juiz-OPL(正規形)→ conf → juiz-OPL が同じ文字列。
4. conf から書いた XML が XSD に通る(CI)。

題材はサンプル 4 リポジトリの conf すべて(56 本、[tests/conf/](../tests/conf/))。各 conf から書いた juiz-OPL を隣に置き、出力が変わったら差分で分かるようにした。juiz の規則の誤りの例は [tests/juiz/invalid/](../tests/juiz/invalid/)(ファイル名の先頭が破る規則)。

## 8. conf だけでは分からないこと

次は plugin の manifest にあり、conf には無い。conf から作るモデルには出さず、Step 8(動いている System の profile_full から書き出す)で埋める。

- つながっていない引数と、その既定値(juiz:default)。conf に出てくる引数だけがモデルに出る。
- 引数の宣言順(juiz:arg_index)。§4.2 の順で代わりにする。
- ContainerProcess が readonly かどうか。effect(書き込み)として作る。モデル → conf ではどちらでも同じ conf になる。
- 値の型(juiz:datatype)。
- juiz:id。conf からは作らない(別の System の Container だけは ID をそのまま持つ)。
