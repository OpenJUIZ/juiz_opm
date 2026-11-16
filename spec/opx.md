# OPX の書き出し

- 状態: 案(Step 9)
- 関連: [metamodel.md](metamodel.md)(モデル)、[diagram.md](diagram.md) §2(図に何を出すか)、[docs/research/opcat-opx.md](../docs/research/opcat-opx.md)(OPX の形)、[0017](../docs/decisions/0017-opx-export.md)

モデルを OPCAT の保存形式(OPX)に書き出し、OPCAT や OPCloud で開けるようにする。書き出すだけで、OPX を読むことはしない。

OPX にはスキーマが無い。ここに書く形は、OPCAT に同梱の例のファイル(Examples/*.opz)と、その読み書きのソースを読んで確かめたものである。OPCAT のソースは CC BY-NC 4.0 なので、コードは持ち込まず、形だけを参照した。

## 1. コマンド

```sh
opm_graph -T opx -o model.opx model.opl     # OPX
opm_graph -T opz -o model.opz model.xml     # 同じものを gzip で圧縮(OPCAT の .opz)
```

- 入力は juiz-OPL か交換形式の XML。1 つのモデルを 1 つのファイルにする(`--diagram` は使えない)。
- OPX に書けないものは、標準エラーに `opm_graph: note: ...` として 1 種類 1 行で出す(§4.4)。
- `creationDate` と `lastUpdate` は書き出した時刻(UTC)。ライブラリの `Options::default()` は決まった日時を使い、同じモデルから同じファイルを作る。

## 2. ファイルの形

```text
OPX version="3"
└ OPMSystem  name, author, creationDate, lastUpdate, maxEntityEntry, maxOpdEntry, global-id
  ├ SystemProperties  SystemConfiguration / GeneralInfo(OPCAT の既定の表示設定)
  ├ LogicalStructure  ObjectSection / ProcessSection / RelationSection / LinkSection
  └ VisualPart        OPD(根)の木
```

- 要素(Thing、State、リンク、関係)には 1 から数える番号を付ける。順は Thing(各 Object の State をその直後に)、モデルのリンク、まとめた線(§4.3)。`maxEntityEntry` はその最大。
- `name` はモデルの名前(無ければ id)、`global-id` はモデルの id、`author` は `juiz_opm`。
- description の無い要素は、OPCAT と同じく `none` と書く。

## 3. OPD

モデルの OPD の木をそのまま OPX の OPD の木にする。根は番号 1 で、深さ優先で番号を付ける。

| refinement | OPX での置き場所 | 名前 | MainEntity |
| --- | --- | --- | --- |
| root | VisualPart の直下 | `SD` | なし |
| in_zoom | 親の `InZoomed` | `SD1 - Coffee Making in-zoomed` | refinee |
| unfold | 親の `Unfolded/UnfoldingProperties`(refinee の番号) | `SD2 - Report unfolded` | refinee |
| view | 親の `View/ViewProperties` | `SD3 View - <description か名前>` | 最初の Appearance |

- OPCAT は名前の ` - ` より前に `View` があるものを view とみなすので、view の名前はこの形にする。
- **OPD の無いモデル**(conf やスナップショットから作ったモデル)には、次の OPD を作ってから書き出す。
  - SD: どの InZoom の中にも無い Thing をすべて出す。
  - 出ている Thing の InZoom ごとに in_zoom の OPD(SD1、SD2、…、その中は SD1.1、…)を作り、refinee、中の Thing、中の Thing とリンクで結ばれた Thing を出す。
- 各 OPD に何を出すかは略式図と同じ規則([diagram.md](diagram.md) §2、`opm_graph::view`)で決める。

## 4. 要素とリンク

### 4.1 Thing と State

| モデル | OPX |
| --- | --- |
| Object | LogicalObject。essence が physical なら `physical="true"`、affiliation が environmental なら `environmental="true"` |
| Process | LogicalProcess |
| State | LogicalState。`initial`・`final`・`default` |

OPD の上では、Thing ごとに VisualThing を書く。State は Object の中に VisualState として必ず書き、Appearance で隠した State は `visible="false"` にする(State を端に取る線が、その State に付けられるように)。InZoom の中の Thing は、refinee の VisualThing の ChildrenContainer に入れ子で書く。座標は親からの相対。

### 4.2 リンクの種類

| kind | modifier | OPX(番号、向き) |
| --- | --- | --- |
| consumption | なし | 301、Object / State → Process |
| consumption | event | 310 |
| consumption | condition | 301(condition は書けない、§4.4) |
| result | | 306、Process → Object / State |
| effect(状態なし) | | 302、**Object → Process**(OPCAT の向き。モデルとは逆) |
| effect(from_state か to_state あり) | | 301(from_state、無ければ Object → Process)と 306(Process → to_state、無ければ Object)の 2 本 |
| instrument | なし / event / condition | 303 / 308 / 304 |
| agent | | 305(modifier は書けない) |
| invocation | | 307 |
| overtime_exception、undertime_exception | | どちらも 309(区別は書けない) |
| aggregation | | 関係 204、全体 → 部分(ordered は書けない) |
| exhibition | | 関係 202、持ち主 → 特徴 |
| generalization | | 関係 201、一般 → 特殊 |
| classification | | 関係 203、型 → 実体 |
| tagged | | 関係 205(backward_tag があれば 206)。tag を `forwardRelationMeaning`、backward_tag を `backwardRelationMeaning` に |

- 関係の向き(201・203・204 の source が一般・型・全体の側であること)は OPCAT の例のファイルで確かめた。metamodel §6.3 の注の確かめはこれで済んだ。
- path はリンクの `path` に、多重度は関係の `sourceCardinality`・`destinationCardinality`(無ければ `1`)に書く。
- effect の event・condition は OPCAT に対応する番号が無いので書かない。
- OPD の上では、aggregation・exhibition・generalization・classification は FundamentalRelationSection に、source と種類が同じものを 1 つの三角(CommonPart)にまとめて書く。tagged は GeneralRelationSection、手続きリンクは VisualLinkSection に書く。

### 4.3 まとめた線

上の階層の OPD に出す「まとめた線」(InZoom の中の Process に付いたリンクを refinee に引き上げたもの、metamodel §7.1)は、モデルには無いが、OPX では OPD の線がどれかの要素を指さなければならない。そこで、まとめた線ごとに要素を 1 つ足す(OPCAT も、in-zoom した Process の外の線を、中の線とは別の要素として持つ)。同じ種類・modifier・両端のまとめた線は、OPD が違っても 1 つの要素にする。

### 4.4 書けないもの

次のものは書かず、§1 の note で知らせる。

- Process の duration
- consumption の condition、effect と agent の event・condition
- overtime と undertime の区別
- aggregation の ordered

juiz の注釈(`juiz:` の属性)は OPX に写さない。juiz の役割は OPCAT や OPCloud では意味を持たないため。OPD の `suppressed_links` は、隠した線を描かないことで写る。

### 4.5 Fan

Fan の線がすべてその OPD にまとめずに出ているとき、VisualLinkSection に OrXorGroup を書く。`type` は or が 0、xor が 1。`isSourceGroup` は、束ねた線の共通の端が線の始点のとき(result など Process から出る線)に `true`。

## 5. 配置

Appearance に layout があればそれを使う。無ければ次の決まった規則で置く。線は `autoArranged="true"` にし、引き方は読む道具に任せる。

- Thing の大きさは名前の長さから決める。State は Object の下側に横に並べる。
- 1 つの OPD の一番上の Thing は 3 列に置く。左: Process に入る線(consumption・instrument・agent)と構造の線しか持たない Object。中: Process と MainEntity。右: その他の Object。列の中は縦に、モデルの順に並べる。
- in-zoom した Thing の中は、InZoom の steps を上から順に 1 段ずつ(同じ step の Process は横に並べる)、中の Object を左に縦に並べる。Process は楕円なので、中身の箱の √2 倍の大きさにして、中身を真ん中に置く。

OPCAT や OPCloud で開いた後に置き直してよい。配置はモデルに戻らない(metamodel §10)。

## 6. 検査

`crates/opm_graph/tests/opx.rs` は、examples/ の 5 つ、tests/conf の conf から作った OPL、tests/system のスナップショットから作った OPL をすべて書き出し、読み手が要るものを確かめる。

- 必要な要素と属性があること。要素の番号が重ならず、`maxEntityEntry` 以下であること。
- リンクと関係の両端が、§4.2 の種類の要素であること。
- OPD の番号が重ならず、根が 1 であること。根以外に MainEntity があること。
- OPD の中で、Thing と State の番号(entityInOpdId)が重ならないこと。State が持ち主の Object の中にあること。中の Thing が親の箱からはみ出さないこと。
- 線(VisualLink・VisualGeneralRelation・VisualFundamentalRelation)が、同じ OPD に描いた Thing か State に付き、その端が論理の要素の両端と同じであること。
- OPD の無いモデルでは、すべての線がどれかの OPD に描かれること。

`OPM_OPX_OUT=<dir>` を付けて走らせると、書き出した .opz をそこに置く。

## 7. OPCAT での確認(2026-11-16)

OPCAT のソースを作業用の環境でビルドし、その読み込み処理(LoaderVer3)で、書き出したファイルを読ませた。OPCAT のコードはこのリポジトリには入れていない。

- examples の 5 つ、tests/conf の conf から作った OPL、tests/system のスナップショットから作った OPL、計 77 ファイルが、読み込みの誤りも確認のダイアログも出さずに読めた。OPCAT 同梱の ATM.opz も同じ方法で読めることを先に確かめた。
- 読んだ OPD を OPCAT に描かせると、in-zoom の楕円と中の Process、State、手続きリンクの矢印、集約などの三角、invocation の稲妻が OPCAT の形で出た。
- 気づいた点: 線の端が楕円の縁ではなく内側で止まるものがある。集約の三角が Thing に重なることがある。どちらも OPCAT の上で動かせば直る。

まだ確かめていないこと: OPCloud で開けるか(OPCloud は契約が要る)。
