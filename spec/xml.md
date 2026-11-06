# モデル交換形式(XML)

- 状態: 案(2026-11-06、Step 2)。決めたことは [docs/decisions/0006](../docs/decisions/0006-xml-format.md) に記録した
- 関連: [metamodel.md](metamodel.md)(この形式が写す表)、[0004](../docs/decisions/0004-xml-interchange.md)(XML にする決定)
- スキーマ: [schema/opm.xsd](schema/opm.xsd)(コアと図)、[schema/juiz.xsd](schema/juiz.xsd)(juiz 拡張)
- 例: [examples/xml/](../examples/xml/)
- 仮置き: §12 の点は Dov Dori 先生の返事次第で変える

## 1. 目的と範囲

[metamodel.md](metamodel.md) の 3 層(コア、図、juiz 拡張)を、道具の間で受け渡す XML に写す。メタモデルが持つものはすべて書け、メタモデルにないものは足さない。

- XSD で検査できる整形式の規則(id の一意、参照先の種類など)は XSD で検査する。残りは Step 4(コア)と Step 7(juiz)の検査器が見る(§9)。
- 文書 1 つにモデル 1 つを入れる。

## 2. 名前空間とファイル

| 層 | 名前空間 URI | 接頭辞 | スキーマ |
| --- | --- | --- | --- |
| コアと図 | `https://openjuiz.github.io/ns/opm/v0` | `opm` | [schema/opm.xsd](schema/opm.xsd) |
| juiz 拡張 | `https://openjuiz.github.io/ns/juiz/v0` | `juiz` | [schema/juiz.xsd](schema/juiz.xsd)(opm.xsd を import する) |

- コアの名前空間は juiz_opm が決めたもので、ISO が決めたものではない。名前は ISO 19450 の用語に従う。
- URI は名前で、その場所にファイルを置くことは今は約束しない。
- 接頭辞は例と文書の慣習で、読み手は接頭辞ではなく URI で判断する。
- ファイルの拡張子は `.opm.xml`、文字コードは UTF-8。
- juiz の注釈を含む文書は juiz.xsd で検査する(juiz.xsd が opm.xsd も読む)。opm.xsd だけで検査すると juiz の注釈は検査せずに通す(§8)。

## 3. 名前の付け方

- 要素名と属性名は metamodel.md の名前をそのまま使う(`object`、`from_state`、`in_zoom`、`juiz:value_kind` など)。小文字と `_` で、juiz の conf とも揃う。
- リンクは種類ごとに要素を分ける(`<opm:instrument>`、`<opm:effect>` など)。`kind` 属性は持たない。種類ごとに持てる属性が違うのを XSD で縛れ、参照先の種類も種類ごとに検査できるため。
- Thing も `<opm:object>` と `<opm:process>` に分ける。
- 真偽は `true` / `false`、既定値と同じなら書かなくてよい。

## 4. id と参照

| 項目 | 決めごと |
| --- | --- |
| 文字 | `[A-Za-z_][A-Za-z0-9_.-]*`。ASCII だけにし、URI の末尾にそのまま使えるようにする |
| 一意 | 文書(= モデル)の中で一意。XSD の `xs:ID` で検査する |
| 付ける要素 | `model`、Thing、State、リンク、`fan`、`in_zoom`、`diagram`、`appearance` |
| 参照 | 属性(`source`、`target`、`from_state`、`refinee`、`thing`)か、`ref` 属性を持つ子要素(`link_ref`、`member`、`part`、`suppressed_state`)。参照先の種類は XSD の keyref で検査する |
| 名前との関係 | id は名前から作らなくてよい。名前を変えても id は変えない |

RDF へ写すときは、モデルごとの基底 URI に id をつなぐ(基底 URI の決め方は RDF を扱う Step で決める)。

## 5. 文書の形

```xml
<opm:model xmlns:opm="https://openjuiz.github.io/ns/opm/v0"
           xmlns:juiz="https://openjuiz.github.io/ns/juiz/v0"
           format_version="0.1" id="m" name="...">
  <opm:description>...</opm:description>      <!-- 0..1 -->
  <!-- 他の名前空間の要素(0 個以上) -->
  <opm:things>                                <!-- 1 -->
    <opm:object id="..." name="...">
      <opm:state id="..." name="..." initial="true"/>
    </opm:object>
    <opm:process id="..." name="..."/>
  </opm:things>
  <opm:links>...</opm:links>                  <!-- 0..1 -->
  <opm:fans>...</opm:fans>                    <!-- 0..1 -->
  <opm:in_zooms>...</opm:in_zooms>            <!-- 0..1 -->
  <opm:diagram id="sd" name="SD">...</opm:diagram>  <!-- 0..1、OPD の根 -->
</opm:model>
```

id を持つ要素はどれも、最初に `<opm:description>`(0..1)、次に他の名前空間の要素(0 個以上)を置き、その後に自分の子を置く。

## 6. 要素の対応

### 6.1 model

| メタモデル | XML |
| --- | --- |
| Model | `<opm:model>`。属性 `id`(必須)、`name`(任意)、`format_version`(必須、今は `0.1`) |

### 6.2 Thing と State(metamodel §4、§5)

| メタモデル | XML |
| --- | --- |
| Thing(kind = object) | `<opm:object>` |
| Thing(kind = process) | `<opm:process>` |
| id、name | 属性 `id`、`name`(どちらも必須) |
| essence、affiliation | 属性 `essence`(既定 `informatical`)、`affiliation`(既定 `systemic`) |
| description | 子 `<opm:description>` |
| states | `<opm:object>` の子 `<opm:state>` の並び。owner は親の `<opm:object>` |
| State の initial、final、default | `<opm:state>` の属性(既定はどれも `false`) |
| duration | `<opm:process>` の子 `<opm:duration min expected max unit/>`(`unit` は必須、ほかは任意) |

### 6.3 リンク(metamodel §6)

`<opm:links>` の子に、種類ごとの要素を並べる。並びに意味はない。

| kind | 要素 | source → target の参照先(XSD で検査) | 種類ごとの属性 |
| --- | --- | --- | --- |
| consumption | `<opm:consumption>` | object か state → process | `modifier` |
| result | `<opm:result>` | process → object か state | |
| effect | `<opm:effect>` | process → object | `modifier`、`from_state`、`to_state`(state を指す) |
| instrument | `<opm:instrument>` | object か state → process | `modifier` |
| agent | `<opm:agent>` | object か state → process | `modifier` |
| invocation | `<opm:invocation>` | process → process(同じでもよい。self-invocation) | |
| overtime_exception | `<opm:overtime_exception>` | process → process | |
| undertime_exception | `<opm:undertime_exception>` | process → process | |
| aggregation | `<opm:aggregation>` | Thing → Thing | `ordered`(既定 `false`) |
| exhibition | `<opm:exhibition>` | Thing → Thing | |
| generalization | `<opm:generalization>` | Thing → Thing | |
| classification | `<opm:classification>` | Thing → Thing | |
| tagged | `<opm:tagged>` | Thing → Thing | `tag`(必須)、`backward_tag` |

どのリンクも `id`、`source`、`target`(必須)と、`path`、`source_multiplicity`、`target_multiplicity`(任意)、子 `<opm:description>` を持てる。

- `modifier` は `event` か `condition`。表で書いた要素にしか書けない(C4 を XSD で検査する)。
- 多重度は `1`、`*`、`0..1`、`2..*`、`3..5` の形。ISO の記号の多重度(`n` など)は今は書けない。

### 6.4 Fan(metamodel §6.4)

```xml
<opm:fans>
  <opm:fan id="f" operator="xor">
    <opm:link_ref ref="l1"/>
    <opm:link_ref ref="l2"/>
  </opm:fan>
</opm:fans>
```

`operator` は `xor` か `or`。`link_ref` は 2 つ以上で、リンクの id を指す。

### 6.5 InZoom(metamodel §7.1)

```xml
<opm:in_zooms>
  <opm:in_zoom id="iz" refinee="making">
    <opm:step><opm:member ref="heating"/><opm:member ref="grinding"/></opm:step>
    <opm:step><opm:member ref="brewing"/></opm:step>
    <opm:part ref="ground"/>
  </opm:in_zoom>
</opm:in_zooms>
```

- `step` の並びが実行順。1 つの `step` の中の `member` は並行。`member` は process を指す。
- `part` は Thing を指す。
- 1 つの Thing を `refinee` にする `in_zoom` は高々 1 つ(XSD で検査する)。

### 6.6 OPD(metamodel §7.2)

OPD の木は要素の木で表す。`<opm:model>` の直下の `<opm:diagram>` が根(SD)で、子の OPD はその中に入れ子で書く。メタモデルの `parent` は親の要素で決まり、属性には書かない。根が 1 つで循環しないこと(C10 の前半)は XML の形で保たれる。

| メタモデル | XML |
| --- | --- |
| OPD(根) | `<opm:model>` の子の `<opm:diagram id name>`。`refinement` は書かない(`root`) |
| OPD(子) | `<opm:diagram>` の子の `<opm:diagram id name refinement refinee>`。`refinement` は `in_zoom` / `unfold` / `view` で必須 |
| Appearance | `<opm:appearance id thing>`。子に `<opm:suppressed_state ref/>`(0 個以上)と `<opm:layout x y width height/>`(0..1) |

`<opm:diagram>` の中は、`<opm:appearance>` を先に、子の `<opm:diagram>` を後に置く。

## 7. juiz の注釈(metamodel §9)

### 7.1 書き方

- 値が 1 つの文字・数・真偽の注釈は、juiz の名前空間の属性にする(`juiz:role="container"`)。
- 構造のある値は、juiz の名前空間の子要素にする。今は `juiz:value`・`juiz:applied`(JSON の文字列)と `juiz:translation`・`juiz:rotation` の 4 つ。子要素は `<opm:description>` の後、自分の子(`<opm:state>` など)の前に置く。
- 値が複数の注釈(`juiz:writers`、`juiz:refs`)は空白区切りの並びにする。並べる値は空白を含まない。

### 7.2 一覧

Thing に付けるもの(metamodel §9.2、§9.3):

| 注釈 | XML | 値 |
| --- | --- | --- |
| juiz:role | 属性 | metamodel §9.2 の 15 種 |
| juiz:id | 属性 | juiz の ID |
| juiz:name | 属性 | juiz の名前 |
| juiz:namespace | 属性 | ネームスペース |
| juiz:type | 属性 | type_name |
| juiz:datatype | 属性 | juiz の型の書き方(juiz の datatype.md §5)。完全名か、`<完全名>@sha256:<64 桁>` |
| juiz:value_kind | 属性 | `output` / `input` / `intermediate` / `const` |
| juiz:value | 子要素 `<juiz:value>` | JSON の文字列 |
| juiz:rate | 属性 | 正の数(Hz) |
| juiz:use_memo | 属性 | 真偽 |
| juiz:path | 属性 | 式の木の中の位置 |
| juiz:translation | 子要素 `<juiz:translation x y z/>` | 省略した成分は 0 |
| juiz:rotation | 子要素 `<juiz:rotation x y z w/>` | 省略した成分は x・y・z が 0、w が 1 |
| juiz:writers | 属性(並び) | Container の名前か ID |
| juiz:status | 属性(リンクにも付く) | `released` / `merged` / `in_progress` / `planned` / `defect` |
| juiz:refs | 属性(並び、リンクにも付く) | `juiz#123` の形 |

リンクに付けるもの(metamodel §9.4):

| 注釈 | XML | 値 |
| --- | --- | --- |
| juiz:arg | 属性 | 引数名 |
| juiz:arg_index | 属性 | 0 以上の整数 |
| juiz:connection | 属性 | `push` / `pull` |
| juiz:connection_name | 属性 | 文字列 |
| juiz:applied | 子要素 `<juiz:applied>` | JSON の文字列 |
| juiz:order | 属性 | 0 以上の整数 |
| juiz:slot_name | 属性 | スロット名 |
| juiz:reference | 属性 | Container の名前 |
| juiz:initial | 属性 | 真偽 |
| juiz:end | 属性 | `source` / `destination` |

`juiz:translation` と `juiz:rotation` の `x` などは名前空間なしの属性である(要素が既に juiz の名前空間にあるため)。

### 7.3 例

```xml
<opm:object id="camera" name="Camera" essence="physical"
            juiz:role="container" juiz:name="camera0" juiz:type="video_capture"/>
<opm:instrument id="l_image_filter" source="image" target="filter"
                juiz:arg="img" juiz:arg_index="0" juiz:connection="push"/>
<opm:instrument id="l_code_filter" source="filter_code" target="filter"
                juiz:arg="code" juiz:arg_index="1">
  <juiz:applied>"COLOR_BGR2GRAY"</juiz:applied>
</opm:instrument>
```

全体は [examples/xml/capture_filter.opm.xml](../examples/xml/capture_filter.opm.xml)(Container、push、main_loop_ec)と [examples/xml/composite.opm.xml](../examples/xml/composite.opm.xml)(Composite の型)。

## 8. 拡張の決まり

- id を持つ要素は、どれも他の名前空間の属性と子要素を持てる(XSD の `##other` を `lax` で受ける)。juiz の注釈はこの仕組みに乗っている。juiz 以外の道具も自分の名前空間で同じように足せる。
- 読み手は、知らない名前空間の属性と要素を無視してよい。juiz の名前空間を全部取り除いた文書は opm.xsd に通る(metamodel §9.1 の 3 を XML で確かめる形。§10 のスクリプトが確かめる)。
- 書き手は、読んだ文書の知らない名前空間の属性と要素を、できるだけ残して書き戻す。
- `lax` なので、juiz の名前空間の綴りの誤り(`juiz:rol` など)は XSD では見つからない。juiz の検査器(Step 7)が知らない注釈を誤りにする。

## 9. XSD が検査すること・しないこと

| 規則(metamodel) | XSD | 残りを見るもの |
| --- | --- | --- |
| C1 id の一意と参照先の実在 | 検査する | |
| C2 Thing の名前の一意、State の名前の owner の中での一意 | 検査する | |
| C3 手続きリンクの向きと端の種類 | 検査する(§6.3 の参照先) | |
| C4 modifier の付く種類 | 検査する | |
| C5 effect の from_state・to_state が target の State | State であることだけ | Step 4(target の State であること) |
| C6 構造リンクの端の種類 | Thing であることだけ | Step 4(両端が同じ kind など) |
| C7 default の State は高々 1 つ | しない | Step 4 |
| C8 何も変えない Process(警告) | しない | Step 4 |
| C9 InZoom の steps と parts | step の member が process、1 つの Thing に InZoom 1 つ | Step 4(重なり、入れ子の循環) |
| C10 OPD の木 | 根が 1 つ、循環なし、子の refinement が必須 | Step 4(in_zoom の refinee が InZoom を持つ、in_zoom・unfold で refinee が必須) |
| C11 Fan のリンク | 2 本以上、リンクを指す | Step 4(kind と Process 側の端が同じ) |
| C12 generalization・classification の循環 | しない | Step 4 |
| juiz の注釈の値の形 | 検査する(§7.2 の値) | |
| どの注釈がどの要素に付くか(metamodel §9.3、§9.4)、J1〜J13 | しない | Step 7 |

XSD 1.0 の範囲で書いている(1.1 の assert は使わない)。libxml2(xmllint、lxml)で検査できるようにするため。

## 10. 検証の仕方

```sh
pip install lxml
python3 scripts/validate_xml.py
```

- `examples/xml/` と `tests/xml/valid/`(今は空。置けば同じく検査する)の文書が juiz.xsd に通り、juiz の名前空間を取り除くと opm.xsd に通ることを確かめる。
- `tests/xml/invalid/` の文書が juiz.xsd に通らないことを確かめる。各ファイルの最初の注釈に、どの規則を破る例かを書いている。
- CI(`.github/workflows/validate-xml.yml`)でも同じスクリプトを走らせる。
- xmllint なら `xmllint --noout --schema spec/schema/juiz.xsd examples/xml/capture_filter.opm.xml` で 1 つずつ確かめられる。

## 11. 版

- `format_version` は形式の版で、今は `0.1` だけを受ける。
- 名前空間 URI の `v0` は、0.x の間は互換性のない変更をしてよいことを表す。形式を固めたら `v1` にし、以後は互換性のない変更のときだけ URI を変える。

## 12. 仮置き(Dori 先生の返事次第)

2026-11-06 時点で、OPM の XML 交換形式の発表や計画があるかを Dov Dori 先生に問い合わせている([0004](../docs/decisions/0004-xml-interchange.md))。既存の形式があれば、次の点はそちらに合わせて変える。

- 名前空間 URI と、コアを juiz_opm 独自の名前空間に置くこと。
- 要素名・属性名(ISO 19450 の用語の綴り)と、リンクを種類ごとの要素に分けること。
- 図(OPD)と配置の持ち方。

メタモデル(metamodel.md)と juiz の注釈の中身は、形式が変わっても変えない見込み。

## 13. この Step で決めないこと

- OPL の文法と、XML との相互変換の規則(Step 3)。→ [opl.md](opl.md) §10。
- 検査器の誤りの書き方と、警告と誤りの区別の細目(Step 4)。
- RDF の基底 URI と語彙(RDF を扱う Step)。
- OPCAT の OPX との対応(Step 9)。
