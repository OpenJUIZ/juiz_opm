# juiz-OPL

- 状態: 案(2026-11-06、Step 3。2026-11-07 に言い回しを入門書に合わせた)。決めたことは [docs/decisions/0007](../docs/decisions/0007-opl-grammar.md) と [0008](../docs/decisions/0008-opl-wording-from-book.md) に記録した
- 関連: [metamodel.md](metamodel.md)(この文法が写す表)、[xml.md](xml.md)(同じモデルの XML)、[0002](../docs/decisions/0002-open-points.md) の 5(英語の OPL を正本にする)
- 例: [examples/opl/](../examples/opl/)(Step 2 の XML の例 3 つを書き起こしたもの)
- 言い回しの参照: D. Dori, *Model-Based Systems Engineering with OPM and SysML*, Springer, 2016(ISBN 978-1-4939-3295-5。以下「入門書」)
- 仮置き: §13 の点は ISO 19450 の本文か Dov Dori 先生の返事で変える

## 1. 目的と範囲

juiz-OPL は、[metamodel.md](metamodel.md) のコアと juiz 拡張を英語の文で書く言語である。人と Claude が読み書きする正本で、道具(Step 5 の opm_opl)はこれを解析・検査し、正規形で書き出す。

- 1 つの文は 1 つの事実(Thing 1 つ、State の並び、リンク 1 本、InZoom 1 つ、Fan 1 つ)を書く。正規形では例外なくそうする(§9)。
- 文は入門書の OPL の文型に合わせる(入門書は ISO 19450 の記法を使い、その上位集合とされる)。合わせなかった点と、入門書で確かめられなかった点は §13 に書く。ISO 19450 の本文はまだ読めていない。
- juiz の注釈は文の末尾の `[...]` に書く(§7)。`[...]` を全部取り除くと、コアだけの OPL になる(metamodel §9.1 の 3)。
- 図の層(OPD、Appearance、配置)は書かない。図はモデルから描くもので、文から図の分け方は作らない([opm-mapping.md](opm-mapping.md) §7)。図を保ちたいときは XML を使う(§10.3)。
- 文書 1 つにモデル 1 つを書く。ファイルの拡張子は `.opl`、文字コードは UTF-8。

## 2. 文書の形

```text
Model "Capture and Filter".

Camera is a physical object [role: container, name: camera0, type: video_capture].

Capture is a process [role: container_process, name: camera0_readandget, type: video_capture_readandget].
Capture affects Camera.
Capture yields Image.
Capture invokes Filter.
```

- 文は `.` で終わる。改行と空白の量に意味はない(正規形は §9.4)。
- 最初の文に `Model` の見出しを置いてよい(§5.1)。
- 行の最初の空白でない文字が `#` なら、その行の終わりまでを注釈(コメント)として読み飛ばす。コメントはモデルに入らないので、正規形には残らない。

## 3. 字句

### 3.1 名前

| 種類 | 引用しない形(bare) | 例 |
| --- | --- | --- |
| Thing の名前 | 空白で区切った語の並び。最初の語は大文字(`A`〜`Z`)で始まり、2 つ目からの語は大文字か数字で始まる。語の中は英字・数字・`_`・`-` | `Camera`、`Main Loop EC Cycle`、`Base-Arm Transform`、`Camera 2` |
| State の名前 | 1 語。小文字(`a`〜`z`)で始まり、中は小文字・数字・`_`。§3.3 の予約語は除く | `started`、`cold` |
| 経路ラベル(path) | State の名前と同じ | `p1` |
| tag | 小文字の語の並び(§6.3) | `is located in` |

この形に合わない名前は `"` で囲む(引用する)。引用の中では `\"` と `\\` だけを特別に扱い、改行は書けない。引用した名前は空でない。

```text
"talker0" is a process [role: process, name: talker0, type: talker].
"robot1/arm" is a physical object [role: container, name: arm, namespace: robot1, type: arm].
Water can be cold, hot, or "almost boiling".
```

- juiz の名前(`talker0`)は、引用すれば OPM の名前にそのまま使える。OPM の名前を `Talker 0` のように付け直すかどうかは、conf から作るときの決めごと(Step 7)で、文法は縛らない。
- 名前の比較は、引用を外し、bare の名前では語の間の空白を 1 つにしてから行う。`"Camera"` と `Camera` は同じ名前である。
- Thing の名前の最初の語が `Model`、`State`、`Following`、`Either`、`Exactly`、`At` のときは、bare で書けても引用する(見出しと、大文字で始まる文頭の語と区別するため)。

### 3.2 その他の字句

| 字句 | 形 |
| --- | --- |
| 多重度 | 名前の前に置く。入門書の語で書く(§5.7)。入力では `1`、`*`、`0..1`、`2..*`、`3..5` の記号の形([xml.md](xml.md) §6.3 と同じ)も受ける |
| 注釈の値 | §7.3 |
| 空白 | 空白、タブ、改行。字句の間に 1 つ以上置ける。`.`、`,`、`:`、`[`、`]`、`{`、`}` の前後は省いてよい |

### 3.3 予約語

次の小文字の語は文の骨組みに使うので、bare の State の名前と経路ラベルにはできない(引用すれば使える)。Thing の名前は大文字で始まるので、予約語とはぶつからない。

```text
a affects an and are as at be can case changes consists consumed consumes default
duration either environmental exactly exceeds exhibits exists falls final finally from
handle handles if in informatical initial initially initiates instance into invokes is
itself least many nothing object of one optional or occurs otherwise parallel path
physical process requires sequence short skipped systemic that to well which yields
zooms
```

## 4. 文法(EBNF)

ISO/IEC 14977 の EBNF で書く。終端記号は `"..."`、`{ }` は 0 回以上、`[ ]` は省略可、`|` は選択。空白とコメントは字句の間にいくらでも入るものとし、文法には書かない。予約語は語として一致するときだけ予約語である(`object` は予約語、`objects` は違う)。

```ebnf
(* 文書 *)
document        = [ header ] , { statement } ;
header          = "Model" , quoted , [ core_block ] , "." ;
statement       = [ path_prefix ] , sentence , [ core_block ] , [ juiz_block ] , "." ;
path_prefix     = "Following" , "path" , label , "," ;

sentence        = declaration | state_list | state_ends | state_sentence | zoom
                | procedural | structural | fan ;

(* Thing と State *)
declaration     = thing , "is" , article , { qualifier } , ( "object" | "process" ) ;
article         = "a" | "an" ;
qualifier       = "physical" | "informatical" | "environmental" | "systemic" ;
state_list      = thing , "can" , "be" , states ;
states          = state , [ { "," , state } , [ "," ] , "or" , state ] ;
state_ends      = thing , "is" , ( "initially" , states , [ "and" , "finally" , states ]
                                 | "finally" , states ) ;
state_sentence  = "State" , state , "of" , thing , [ "is" , ( "default" | flags ) ] ;
flags           = flag , [ { "," , flag } , [ "," ] , "and" , flag ] ;   (* 入力だけ。§9.1 *)
flag            = "initial" | "final" | "default" ;

(* InZoom *)
zoom            = thing , "zooms" , "into" , zoom_body ;
zoom_body       = ( "nothing" | steps | things ) , [ "," , "as" , "well" , "as" , things ] ;
steps           = step , [ { "," , step } , [ "," ] , "and" , step , [ "," ] ,
                  "in" , "that" , "sequence" ]
                | "parallel" , things ;
step            = thing | "parallel" , things ;
things          = thing , [ { "," , thing } , [ "," ] , "and" , thing ] ;

(* 手続きリンク *)
procedural      = consumption | result | effect | instrument | agent
                | invocation | exception ;
end             = [ multiplicity ] , [ state ] , thing ;   (* State は Object の端だけ *)
plain_end       = [ multiplicity ] , thing ;
condition       = thing , ( "exists" | "is" , state ) ;
skip            = "otherwise" , thing , "is" , "skipped" ;

consumption     = plain_end , "consumes" , ends
                | end , "initiates" , thing , "," , "which" , "consumes" , end
                | thing , "occurs" , "if" , condition , "," , "in" , "which" , "case" ,
                  ( thing , "consumes" , end | thing , "is" , "consumed" ) , "," , skip ;
                                         (* "is consumed" は入力だけ *)
result          = plain_end , "yields" , ends ;
effect          = plain_end , ( "affects" , plain_ends | "changes" , change )
                | thing , "initiates" , thing , "," , "which" ,
                  ( "affects" , thing | "changes" , change )
                | thing , "occurs" , "if" , condition , "," , "in" , "which" , "case" ,
                  thing , ( "affects" , thing | "changes" , change ) , "," , skip ;
change          = thing , ( "from" , state , [ "to" , state ] | "to" , state ) ;
instrument      = plain_end , "requires" , ends
                | end , "initiates" , thing , "," , "which" , "requires" , end
                | thing , "occurs" , "if" , condition , "," , skip ;
agent           = ends , "handles" , plain_end
                | end , "initiates" , "and" , "handles" , thing
                | end , "handles" , thing , "if" , condition , "," , skip ;
invocation      = plain_end , "invokes" , ( plain_ends | "itself" ) ;
exception       = thing , "occurs" , "if" , "duration" , "of" , thing ,
                  ( "exceeds" | "falls" , "short" , "of" ) , number , unit ;
ends            = end , [ { "," , end } , [ "," ] , "and" , end ] ;          (* 入力だけ。§9.1 *)
plain_ends      = plain_end , [ { "," , plain_end } , [ "," ] , "and" , plain_end ] ;

(* 構造リンク *)
structural      = aggregation | exhibition | generalization | classification | tagged ;
aggregation     = plain_end , "consists" , "of" , plain_ends ;
exhibition      = plain_end , "exhibits" , plain_ends ;
generalization  = thing , "is" , article , thing ;
classification  = thing , "is" , "an" , "instance" , "of" , thing ;
tagged          = plain_end , tag , plain_end , [ "," , "and" , thing , tag , thing ]
                | thing , "and" , thing , "are" , tag ;              (* reciprocal *)
tag             = quoted | tag_word , { tag_word } ;

(* Fan *)
fan             = thing , fan_verb , fan_ends
                | fan_ends , ( "handles" | "handle" ) , thing ;
fan_verb        = "consumes" | "yields" | "affects" | "requires" | "invokes" ;
fan_ends        = "either" , end , "or" , end
                | ( "exactly" | "at" , "least" ) , "one" , "of" ,
                  end , { "," , end } , [ "," ] , "or" , end ;
                  (* 文頭では Either、Exactly、At と大文字で書く *)

(* 注釈 *)
core_block      = "{" , entry , { "," , entry } , "}" ;
juiz_block      = "[" , entry , { "," , entry } , "]" ;
entry           = key , ":" , value ;
key             = lower , { lower | digit | "_" } ;
value           = json | atom , { atom } ;   (* json は §7.3 の鍵だけ *)
atom            = bare_atom | quoted ;

(* 字句(§3) *)
thing           = bare_thing | quoted ;
state           = bare_state | quoted ;
label           = bare_state | quoted ;
bare_thing      = upper , { word_char } , { space , ( upper | digit ) , { word_char } } ;
bare_state      = lower , { lower | digit | "_" } ;               (* 予約語を除く *)
tag_word        = lower , { lower | "-" } ;
word_char       = upper | lower | digit | "_" | "-" ;
multiplicity    = digits , [ "to" , ( digits | "many" ) ]
                | "optional" | "an" , "optional" | "at" , "least" , "one"
                | "*" | digits , ".." , ( digits | "*" ) ;     (* 記号の形は入力だけ *)
number          = digits , [ "." , digits ] ;
unit            = lower , { lower } ;
bare_atom       = atom_char , { atom_char } ;
atom_char       = ? 空白・「,」・「[」・「]」・「{」・「}」・「"」以外の文字 ? ;
quoted          = '"' , { ? 「"」「\」改行以外の文字 ? | "\" , ( '"' | "\" ) } , '"' ;
json            = ? RFC 8259 の JSON の値 1 つ ? ;
digits          = digit , { digit } ;
upper           = ? "A" から "Z" ? ;
lower           = ? "a" から "z" ? ;
digit           = ? "0" から "9" ? ;
space           = ? 空白 1 つ以上 ? ;
```

解析の補足:

- この文法は文の種類を前から順に試せば一意に決まる(順序付きの選択、PEG で書ける)。`is` の後は、`an instance of` なら classification、qualifier か `object` / `process` なら declaration、`initially` / `finally` なら state_ends、Thing なら generalization、どれでもなければ tagged の tag の一部として読む。
- 文頭の語は、入門書に合わせて大文字で書く(`Either`、`Exactly`、`At`)。ただし State の名前は文頭でも小文字のまま書く(`started Main Loop EC initiates ...`)。入門書は `Crashed Vehicle initiates ...` と大文字にするが、それでは Thing の名前と区別できないため(§13)。
- 手続きリンクの文で、名前が 2 つ続いたら(`started Main Loop EC`、`"started" "Main Loop EC"`)、前が State で後が Thing である。
- 名前の後に `and` が来たときは、並び(`things`、`ends`)の続きとして読む。Thing の名前は大文字で始まる語だけでできているので、`and` は名前に入らない。
- どの名前が Object でどれが Process かは、文法では決めない。宣言(§5.2)を全部読んでから照らし合わせる(§8)。

## 5. 文と事実の対応

表の「読み」は日本語の定型文([0002](../docs/decisions/0002-open-points.md) の 5)。日本語の文は今は読み書きの対象にせず、説明のために対応だけを置く。

### 5.1 見出し

| 文 | 事実 | 読み |
| --- | --- | --- |
| `Model "Coffee Making".` | Model の name | モデル「Coffee Making」 |

見出しは省略してよい。`{...}` に Model の `id` と `description` を書ける(§6)。

### 5.2 Thing と State

| 文 | 事実 | 読み |
| --- | --- | --- |
| `Camera is a physical object.` | Thing(kind object、essence physical) | Camera は物理的な Object |
| `Barista is an environmental physical object.` | 上に affiliation environmental | Barista は環境の物理的な Object |
| `Capture is a process.` | Thing(kind process) | Capture は Process |
| `Water can be cold or hot.` | Water の states の並び | Water は cold か hot になりうる |
| `Vehicle is initially intact and finally crashed.` | State の initial・final | Vehicle は初め intact で、終わりに crashed |
| `Water is initially cold.` / `Frog is finally adult.` | initial だけ / final だけ | |
| `State cold of Water is default.` | State の default | Water の既定の State は cold |
| `State hot of Water {id: water_hot}.` | State の id・description だけを書く | |

- Thing はどれも宣言の文をちょうど 1 つ持つ。Thing の id、description、juiz の注釈は宣言の文に書く。
- qualifier は既定(informatical、systemic)なら書かない。入力では書いてもよい。physical と informatical、environmental と systemic は同時に書けない。
- 冠詞は入力ではどちらでもよい。正規形は次の語が母音字(a, e, i, o, u)で始まれば `an`、それ以外は `a`。
- `can be` の並びが Object の states の並び(metamodel §4.2)である。1 つの Object に `can be` の文は高々 1 つ。
- `is initially ... and finally ...` は、Object の initial の State と final の State を 1 文に書く(入門書の形)。initial か final が 2 つ以上なら `or` で並べる(`is initially s1 or s2`)。1 つの Object にこの文は高々 1 つ。
- State の文は、default が真か、State に `{...}` か `[...]` を付けるときに書く。入力では `State s of A is initial and final.` のように initial と final も書けるが、正規形では `is initially` の文に書く。

### 5.3 手続きリンク

`P` は Process、`A` は Object、`s`・`s1`・`s2` は A の State。端に State を書くと、そのリンクの端は State になる(metamodel §6.1)。

| kind | modifier | 文 | 読み |
| --- | --- | --- | --- |
| consumption | なし | `P consumes A.` / `P consumes s A.` | P は A を消費する |
| consumption | event | `A initiates P, which consumes A.` | A が P を起こし、P は A を消費する |
| consumption | condition | `P occurs if A exists, in which case P consumes A, otherwise P is skipped.`(State なら `if A is s, in which case P consumes s A`) | A があれば P が起こって A を消費し、なければ P は飛ばされる |
| result | なし | `P yields A.` / `P yields s A.` | P は A を生じる |
| effect | なし | `P affects A.` | P は A に作用する |
| effect | なし | `P changes A from s1 to s2.` / `P changes A from s1.` / `P changes A to s2.` | P は A を s1 から s2 に変える |
| effect | event | `A initiates P, which affects A.` / `A initiates P, which changes A from s1 to s2.` | |
| effect | condition | `P occurs if A exists, in which case P affects A, otherwise P is skipped.` / `P occurs if A is s1, in which case P changes A from s1 to s2, otherwise P is skipped.` | |
| instrument | なし | `P requires A.` / `P requires s A.` | P は A を使う |
| instrument | event | `A initiates P, which requires A.` | A が P を起こし、P は A を使う |
| instrument | condition | `P occurs if A exists, otherwise P is skipped.` / `P occurs if A is s, otherwise P is skipped.` | A が(s で)あれば P が起こり、なければ飛ばされる |
| agent | なし | `A handles P.` | A が P を扱う |
| agent | event | `A initiates and handles P.` | |
| agent | condition | `A handles P if A exists, otherwise P is skipped.` | |
| invocation | | `P1 invokes P2.` / `P invokes itself.` | P1 は P2 を起こす / P は自分を起こす |
| overtime_exception | | `P2 occurs if duration of P1 exceeds 60 min.` | P1 の時間が 60 分を超えたら P2 が起こる |
| undertime_exception | | `P2 occurs if duration of P1 falls short of 30 min.` | P1 の時間が 30 分に満たなければ P2 が起こる |

- 文の中で同じ Thing を 2 回書く形(event の `A ... A`、condition の `P ... P`)は、2 回とも同じ名前(と同じ State)でなければならない。
- condition の effect で from_state があるときは `if A is s1` と書き、`s1` は from_state と同じ。from_state がなければ `if A exists` と書く。
- invocation の source と target が同じなら `itself` と書く(metamodel §6.2、ISO 19450:2024 §9.5.2.5.2 の self-invocation)。正規形では `P invokes P.` とは書かない。
- 例外リンクの source は時間を超えた(足りない)側の P1、target は起こる側の P2(metamodel §6.2 の向き)。数と単位は P1 の duration の最大(overtime)か最小(undertime)と同じでなければならず、P1 はそれを持たなければならない。数と単位は P1 の宣言の `{...}`(§6)が正本で、例外の文の数は照らし合わせに使う。
- 多重度は名前の前に書き、その名前の側の端の多重度になる(`P consumes 2 A.` は source_multiplicity が `2`、§5.7)。modifier のある文と Fan には書けない(§10.2)。
- 経路ラベルは文頭に `Following path p1,` と書く。

### 5.4 構造リンク

| kind | 文 | 読み |
| --- | --- | --- |
| aggregation | `W consists of A.` / `W consists of 4 A.` | W は A からなる |
| exhibition | `A exhibits X.` | A は X を持つ |
| generalization | `B is an A.` | B は A の一種 |
| classification | `B is an instance of A.` | B は A の実体 |
| tagged | `Coffee Machine is located in Kitchen.` | tag の文そのまま |
| tagged(backward_tag あり) | `A is located in B, and B contains A.` | |
| tagged(backward_tag が tag と同じ、reciprocal) | `Sarah and Isaac are family.` | |

- tagged の tag は、bare で書くと他の文に読めてしまうとき(tag が `is` に冠詞・qualifier・`an instance`・`initially`・`finally` が続く形、または §3.3 の動詞 `consists`・`exhibits`・`consumes`・`yields`・`affects`・`changes`・`requires`・`handles`・`invokes`・`initiates`・`occurs`・`can`・`zooms` で始まる形)と、小文字と `-` 以外を含むときに引用する。
- backward_tag のある tagged は、`, and` の後に終点・backward_tag・始点の順で書く。後半の 2 つの名前は前半と逆の順で一致しなければならない。入門書は 2 つの文(`Sarah is mother of Isaac.` `Isaac is son of Sarah.`)に分けて書くが、それでは片向きの tagged 2 本と区別できないので、1 文にする(§13)。
- backward_tag が tag と同じ(reciprocal)ときは、入門書の形 `A and B are tag.` で書く。A が source、B が target。
- aggregation の `ordered` は `{ordered: true}` で書く(§6)。

### 5.5 InZoom

| 文 | 事実 |
| --- | --- |
| `Blur Gray zooms into Cvt Color Node and Gaussian Blur Node, in that sequence, as well as Code Const and Gray.` | steps = [{Cvt Color Node}, {Gaussian Blur Node}] と parts |
| `Coffee Making zooms into parallel Grinding and Water Heating, and Brewing, in that sequence.` | steps = [{Grinding, Water Heating}, {Brewing}] |
| `P zooms into SP1, parallel SP2 and SP3, SP4, and parallel SP5, SP6, and SP7, in that sequence.` | 入門書の例。steps が 4 つ |
| `P zooms into Q.` / `P zooms into parallel Q and R.` | step が 1 つ |
| `P zooms into nothing, as well as A.` | steps が空で parts だけ |
| `Car zooms into Engine and Wheel.` | refinee が Object。並びは parts |

- step が 2 つ以上なら並びの後に `in that sequence` を書く(入門書の形)。`parallel` で始まる 1 まとまりが並行の step で、そのまとまりは `and` の付いた最後の名前で終わる。
- `as well as` の後が parts である。
- refinee が Process のとき、`in that sequence` も `parallel` もない 2 つ以上の並びは書けない(並行か順か分からないため)。
- refinee が Object のときは、`zooms into` の後に parts を 1 つの並びで書き、`in that sequence`、`parallel`、`nothing`、`as well as` は使わない。
- 1 つの Thing に `zooms into` の文は高々 1 つ(metamodel C9)。

### 5.6 Fan

| 文 | 事実 |
| --- | --- |
| `Brewing consumes either Ground Coffee or Instant Coffee.` | XOR の Fan と consumption 2 本 |
| `P yields exactly one of A, B, or C.` | XOR の Fan と result 3 本 |
| `P yields at least one of A, B, or C.` | OR の Fan と result 3 本 |
| `Either Alice or Bob handles Brewing.` | XOR の Fan と agent 2 本 |
| `At least one of Alice and Bob handles Brewing.` | 入力だけ。正規形は `At least one of Alice or Bob handles Brewing.` |

- XOR は、links が 2 本なら `either A or B`、3 本以上なら `exactly one of A, B, or C` と書く。OR は本数によらず `at least one of A, B, or C`(2 本なら `at least one of A or B`)。入門書は OR の 2 本の例で `and` も使うので、入力では `and` も受ける。
- agent の Fan は、入門書の例には `handle` もあるが、正規形は `handles` にする。入門書の組み合わせの XOR・OR(`exactly 2 of`、`at least 2 of`)はメタモデルにないので書けない。

- Fan の文は、Fan 1 つとその links を一度に書く。Fan の links は Fan の文にだけ書き、別の文に重ねて書かない。
- Fan の links には modifier、経路ラベル、多重度、`[...]` を付けられず、links の description も書けない(§10.2)。links の id は `{link_ids: ...}` で並びの順に書く(§6)。
- effect の Fan は `affects` だけで、状態変化は書けない。

### 5.7 多重度

| 多重度(xml.md §6.3) | 正規形の書き方 | 例 |
| --- | --- | --- |
| `n` | 数 | `Robot consists of 2 Arm.` |
| `n..m` | `n to m` | `Robot consists of 2 to 4 Arm.` |
| `1..*` | `at least one` | `Body consists of at least one Section.` |
| `n..*`(n が 2 以上) | `n to many` | `Vehicle consists of 2 to many Front Sensor.` |
| `0..1` | `an optional` | `Robot consists of an optional Gripper.` |
| `*`、`0..*` | `optional` | `Robot consists of optional Sensor.` |

- 入門書は多重度の後の名前を複数形にする(`2 to many Front Sensors`)が、juiz-OPL は名前を変えない(§13)。
- 入門書は 10 以下の数を語で書く書き方(`Three`)も示すが、juiz-OPL は数字で書く。

## 6. コアの属性の書き方 `{...}`

文の形を持たないコアの属性は、文の末尾(`.` の前、`[...]` の前)の `{...}` に書く。

| 鍵 | 付く文 | 値 | メタモデル |
| --- | --- | --- | --- |
| `id` | すべて(見出しを含む) | id の文字(xml.md §4) | 各要素の id |
| `description` | 見出し、宣言、State の文、リンクの文 | 引用した文字列 | description |
| `ordered` | aggregation | `true` / `false` | ordered |
| `duration_min`、`duration_expected`、`duration_max` | Process の宣言 | 数 | duration |
| `duration_unit` | Process の宣言 | 語(`min`、`s` など) | duration の単位。duration のどれかを書くなら必須 |
| `link_ids` | Fan の文 | id の並び(空白区切り、文の中の並びの順) | Fan の links の id |

例: `Brewing is a process {duration_min: 2, duration_expected: 3, duration_max: 5, duration_unit: min}.`

## 7. juiz の注釈の書き方 `[...]`

### 7.1 置き場所

juiz の注釈は、その要素を書く文の末尾の `[...]` に書く。

| 要素 | 文 |
| --- | --- |
| Thing | 宣言の文 |
| State | State の文 |
| リンク | そのリンクの文(1 文 1 本の文。並びや Fan の文には付けられない) |

### 7.2 鍵

鍵は metamodel §9.3・§9.4 の注釈の名前から `juiz:` を外したもの(`role`、`name`、`arg`、`connection` など)。`juiz:` 以外の名前空間の注釈は OPL では書けない(§10.2)。

`{...}` の `id` はモデルの id、`[...]` の `id` は juiz の ID(`juiz:id`)で、別のものである。

### 7.3 値

| 値の種類 | 鍵 | 書き方 |
| --- | --- | --- |
| 1 つの語 | `role`、`id`、`name`、`namespace`、`type`、`datatype`、`value_kind`、`path`、`arg`、`connection`、`connection_name`、`slot_name`、`reference`、`end`、`status` | atom 1 つ。空白・`,`・`[`・`]`・`{`・`}`・`"` を含むなら引用する |
| 数 | `rate`、`arg_index`、`order` | atom 1 つ(`10`、`0.5`) |
| 真偽 | `use_memo`、`initial` | `true` / `false` |
| JSON | `value`、`applied` | JSON の値 1 つ(`"COLOR_BGR2GRAY"`、`3`、`[1, 2]`、`{"a": 1}`)。JSON の文字列は JSON の引用であり、OPL の引用ではない |
| 並び | `writers`、`refs` | atom を空白で区切って並べる(`refs: juiz#376 juiz#418`) |
| 成分 | `translation` | 数 3 つ(x y z、`translation: 0.1 0 0.5`) |
| 成分 | `rotation` | 数 4 つ(x y z w、`rotation: 0 0 0 1`) |

`[...]` の中の改行は空白と同じに扱う。

### 7.4 juiz の形の例

metamodel §9.5 の形を OPL で書くと次のとおり(注釈は要点だけ)。

| juiz | OPL |
| --- | --- |
| Process と戻り値 | `Increment is a process [role: process, name: inc0, type: increment].` `Increment yields Count.` `Count is an object [role: value, value_kind: output, datatype: juiz.Int64].` |
| つながっていない引数 | `Increment requires Arg [arg: arg, arg_index: 0].` |
| push 接続 | `Filter requires Image [arg: img, arg_index: 0, connection: push].` `Capture invokes Filter.` |
| pull 接続 | `Filter requires Image [arg: img, arg_index: 0, connection: pull].` |
| p_apply | `Filter requires Filter Code [arg: code, arg_index: 1, applied: "COLOR_BGR2GRAY"].` |
| 書き込み / readonly の ContainerProcess | `Capture affects Camera.` / `Read Pose requires Arm.` |
| Topic | `Talker yields Chatter.` `Chatter initiates Listener, which requires Chatter [arg: msg, arg_index: 0].` |
| EC | `Main Loop EC can be stopped or started.` `Main Loop EC is initially started.` `Main Loop EC Cycle occurs if Main Loop EC is started, otherwise Main Loop EC Cycle is skipped.` `Main Loop EC Cycle invokes itself.` `Main Loop EC Cycle invokes Capture [order: 0].` |
| one_shot_ec | `One Shot EC Cycle changes One Shot EC from started to stopped.` |
| System | `Vision System consists of Camera.` `Vision System exhibits Capture.` |
| CompositeContainer | `Robot consists of Arm [slot_name: arm].` |
| Transform | `Base-Arm Transform source Base [end: source].` `Base-Arm Transform destination Arm [end: destination].`(tag は metamodel §9.5 のとおり `source` / `destination`) |
| pose_listener | `Arm exhibits Arm Pose.` `Arm Pose initiates Tracker, which requires Arm Pose [arg: pose, arg_index: 0, reference: world, initial: true].` |
| 実装状況 | `Filter is a process [role: process, name: cvt_color0, type: cv_cvt_color, status: in_progress, refs: juiz#999].` |

## 8. 整形式の規則

文法(§4)に合うことに加え、次を満たす文書を整形式とする。O の規則は OPL を読むときに見る。読んだ結果のモデルは、さらに metamodel のコアの規則 C1〜C12 と、juiz の注釈があれば J1〜J13 を満たす。

| 番号 | 規則 |
| --- | --- |
| O1 | 文に出る Thing の名前は、どれも宣言の文をちょうど 1 つ持つ。宣言は使う文より後にあってもよい |
| O2 | Object ごとに `can be` の文は高々 1 つで、State の名前はその中で重ならない。文に出る State は、その Object の `can be` にある |
| O3 | State の文は State ごとに高々 1 つで、`is initially` / `is finally` の文は Object ごとに高々 1 つ。同じ State の flag を重ねて書かない |
| O4 | 文の形が求める kind に従う。`consumes`・`yields`・`affects`・`changes`・`requires`・`invokes`・`handles`・`occurs` の Process の位置は Process、Object の位置は Object。`zooms into` の steps は Process、Process の refinee の parts は Object |
| O5 | 1 つの文の中で同じ Thing を 2 回書く形は、両方が一致する(§5.3、§5.4) |
| O6 | 並びの文(`ends`、`plain_ends`)には `{...}` と `[...]` を付けない |
| O7 | `{...}` と `[...]` の鍵は、その文が書く要素に付けられるもの(§6、§7、metamodel §9.3・§9.4)だけで、同じ鍵を 2 回書かない。値の形は §6・§7.3 に従う |
| O8 | `{id: ...}` の id はモデルの中で一意で、xml.md §4 の文字に合う。書いていない id は §10.4 の規則で作る |
| O9 | Fan の文の並びは 2 つ以上。`{link_ids: ...}` を書くなら数が並びと同じ |
| O10 | modifier のある文は多重度を書かない |
| O11 | `Following path` は手続きリンクの文にだけ付ける。宣言の qualifier は physical と informatical、environmental と systemic を同時に書かず、同じ語を 2 回書かない |
| O12 | 例外リンクの文の数と単位は、P1 の duration の最大か最小と一致する(§5.3) |

規則に反したときの報告の形と、誤りと警告の区別は Step 5 で決める。

## 9. 正規形

同じモデルからはいつも同じ文字列ができるように、書き出しの形を 1 つに決める。往復テスト(opm-mapping.md §7)はこの形で比べる。

### 9.1 文の選び方

- 1 つの文に 1 つの事実を書く。リンクは 1 本 1 文で、並びの形(`P requires A and B.`)は使わない。例外は Fan の文で、Fan と links を 1 文で書く。
- Thing はどれも宣言の文を書く。State を持つ Object は `can be` の文を書く。initial か final の State があれば `is initially ... and finally ...` の文を書く。State の文は §5.2 の条件のときだけ書く。
- 並びは入門書に合わせ、3 つ以上なら最後の `and` / `or` の前にも `,` を置く(`A, B, and C`、`cold, warm, or hot`)。2 つなら `,` を置かない。
- 既定の値は書かない(qualifier の informatical・systemic、`ordered: false`、State の flag の偽)。qualifier は affiliation、essence の順に書く(`an environmental physical object`)。
- InZoom は step が 2 つ以上なら `in that sequence` を付け、並行の step は `parallel` で書く。
- reciprocal の tagged は `A and B are tag.`、XOR の Fan は 2 本なら `either`、3 本以上なら `exactly one of` で書く。多重度は §5.7 の語で書く。
- 引用は §3.1 の bare の形で書けないとき、と §3.1・§5.4 で引用すると決めたときだけ使う。

### 9.2 文の並び

1. 見出し(Model に name か description があるとき。id を書く形(§10.4)では必ず書く)。
2. Thing ごとのまとまり。Thing は名前の Unicode の符号位置の順に並べる。

まとまりは、その Thing を「持ち主」とする文からなる。

| 文 | 持ち主 |
| --- | --- |
| 宣言、`can be`、`is initially`、State の文、`zooms into` | その Thing(State の文は State の owner) |
| 手続きリンク(invocation と例外を除く)と Fan | Process の側の端の Thing |
| invocation、例外リンク | source の Process |
| 構造リンク | source の Thing |

まとまりの中の並び:

1. 宣言
2. `can be`
3. `is initially ... and finally ...`
4. State の文(`can be` の並びの順)
5. `zooms into`
6. リンクの文と Fan の文を、kind の順(consumption、effect、result、agent、instrument、invocation、overtime_exception、undertime_exception、aggregation、exhibition、generalization、classification、tagged)に。同じ kind の中は、正規形で書いた文の文字列の符号位置の順

### 9.3 文の中の並び

- InZoom の step の中の Process、parts、Fan の並びの中の端は、名前の符号位置の順。`is initially` / `is finally` の State は `can be` の並びの順。
- `{...}` は §6 の表の順、`[...]` は `role` を最初にし、続けて metamodel §9.3(Thing)か §9.4(リンク)の表の順、最後に `status`、`refs`。
- 数は、整数なら小数点なしで、それ以外は元の値に戻る最も短い 10 進の表記で書く。translation と rotation は省いた成分を補って全部書く。
- JSON の値は、空白を入れない形(`{"a":1,"b":[1,2]}`)で、オブジェクトの鍵の順は元のまま書く。

### 9.4 空白

- 1 行に 1 文を書き、まとまりの間に空行を 1 つ置く。見出しの後にも空行を 1 つ置く。
- 字句の間は空白 1 つ。`,` と `.` と `:` の前には空白を置かず、`,` と `:` の後に空白を 1 つ置く。`{` と `[` の前に空白を 1 つ置き、括弧の内側には空白を置かない。
- ファイルは最後の文の後の改行 1 つで終わる。行末に空白を置かない。

## 10. XML との対応

### 10.1 文と要素

| OPL | XML(xml.md §6) |
| --- | --- |
| 見出し | `<opm:model>` の `name`、`{id}`、`{description}` |
| 宣言 | `<opm:object>` / `<opm:process>`、qualifier は `essence`・`affiliation` |
| `can be` | `<opm:object>` の子の `<opm:state>` の並び |
| `is initially ... and finally ...`、State の文 | `<opm:state>` の `initial`・`final`・`default` |
| 手続きリンク・構造リンクの文 | §5.3・§5.4 の kind の要素。modifier は `modifier`、状態変化は `from_state`・`to_state`、`itself` は source と target が同じ、tag は `tag`・`backward_tag`、多重度は `source_multiplicity`・`target_multiplicity`、経路ラベルは `path` |
| `zooms into` | `<opm:in_zoom>`。`in that sequence` の並びの 1 つずつ(`parallel` のまとまりは 1 つ)が `<opm:step>`、`as well as` の後が `<opm:part>`。refinee が Object なら並びが `<opm:part>` |
| Fan の文 | `<opm:fan>` と、links の要素 |
| `{duration_...}` | `<opm:duration>` |
| `{description}` | `<opm:description>` |
| `[...]` | juiz の名前空間の属性と子要素(xml.md §7.2) |

### 10.2 OPL で書けないもの

| もの | OPL から XML にしたとき | XML から OPL にしたとき |
| --- | --- | --- |
| OPD(`<opm:diagram>`)、Appearance、配置 | 書かない | 落とし、落としたことを知らせる |
| juiz 以外の名前空間の属性と要素 | 書かない | 落とし、落としたことを知らせる |
| modifier・経路ラベル・多重度・注釈・description のある links を持つ Fan | | 誤りにする |
| modifier と多重度を両方持つリンク | | 誤りにする |
| 状態変化を持つ effect の Fan | | 誤りにする |
| 例外リンクの source に duration の最大・最小がない | | 誤りにする |

最後の 4 つは juiz では使わず、OPCAT のモデルにも少ない見込みなので、今は書けないままにする。必要になったら文の形を足す。

### 10.3 往復

| 往復 | 一致するもの |
| --- | --- |
| OPL → モデル → OPL | 正規形にした OPL の文字列が一致する |
| XML → OPL → XML(id を書く形) | §10.2 で落とすものを除き、モデルとして一致する |
| XML → OPL → XML(id を書かない形) | 上に加え、id の付け替えを除いて一致する |

モデルとしての一致は、id の値、並びに意味のない要素の順(links の順など)、既定値を書くかどうかを区別しない。

### 10.4 id

- 既定では、OPL は `{id: ...}` を書かない。読むときは次の規則で id を作る。id を保ちたいとき(OPCAT などから来た XML を往復させるとき)は、すべての要素に `{id: ...}` を書く形で書き出す(Step 5 の選択肢)。
- `{id: ...}` が書いてある要素はその id を使う。作る id がそれとぶつかるときは、作る側に `_2`、`_3` を足す。

| 要素 | 作る id |
| --- | --- |
| Model | `opm-model` |
| Thing | 名前の英数字を小文字にし、それ以外の文字の連なりを `_` 1 つにして、前後の `_` を除く(`Main Loop EC Cycle` → `main_loop_ec_cycle`)。空なら `t`、数字で始まるなら前に `t_` を付ける。ぶつかったら名前の順で後のものに `_2`、`_3` |
| State | `<owner の id>.<State の名前を同じ規則で変えたもの>`(空なら `s`)。ぶつかったら `_2` |
| リンク | `<kind>-<source の id>-<target の id>`(`result-capture-image`、`instrument-main_loop_ec.started-main_loop_ec_cycle`)。同じ id になるリンクが 2 本以上あるときは、正規形の文の順で 2 本目から `-2`、`-3` |
| InZoom | `in_zoom-<refinee の id>` |
| Fan | `fan-<Process の端の id>`。2 つ以上あれば正規形の文の順で `-2`、`-3` |

Thing と State の id には `-` が入らないので、`-` でつないだリンクの id は Thing と State の id とぶつからない。

## 11. 例

Step 2 の XML の例を書き起こしたもの。id を書かない形で、XML の OPD と配置は書いていない(§10.2)。

| OPL | XML | 中身 |
| --- | --- | --- |
| [capture_filter.opl](../examples/opl/capture_filter.opl) | [capture_filter.opm.xml](../examples/xml/capture_filter.opm.xml) | Container、push 接続、p_apply、main_loop_ec |
| [composite.opl](../examples/opl/composite.opl) | [composite.opm.xml](../examples/xml/composite.opm.xml) | Composite の型(InZoom、node、const、expose) |
| [coffee_making.opl](../examples/opl/coffee_making.opl) | [coffee_making.opm.xml](../examples/xml/coffee_making.opm.xml) | juiz の注釈のないコア(agent、consumption、状態変化、XOR の Fan、tagged、duration、並行の step) |

capture_filter.opl の一部を id を書く形で書くと次のとおり。

```text
Capture yields Image {id: l_capture_image}.
Capture invokes Filter {id: l_capture_filter}.
Filter requires Image {id: l_image_filter} [arg: img, arg_index: 0, connection: push].
```

## 12. 標準の OPL との関係

- `[...]` を取り除いた文書は、juiz の注釈のないコアのモデルを書いた juiz-OPL になる(metamodel §9.1 の 3 を OPL で確かめる形)。
- `{...}` は juiz-OPL が足したもので、標準の OPL にはない。標準の OPL として読ませるときは `{...}` も取り除く。
- ISO の OPL は OPD ごとに文を並べるが、juiz-OPL はモデル全体を 1 つの文書に書く。OPD ごとの OPL は、図を描く道具(Step 6)がモデルから作る。

## 13. 入門書との違いと仮置き

2026-11-07 に入門書(冒頭の参照)と照らし合わせ、文型を合わせた([0008](../docs/decisions/0008-opl-wording-from-book.md))。入門書の表のいくつか(invocation、condition の agent・instrument、例外リンクの undertime など)は図で、本文から読めなかった。

### 13.1 入門書に合わせたもの

| 項目 | 入門書の形 |
| --- | --- |
| リンクの基本の文 | `consumes`、`yields`、`affects`、`changes ... from ... to`、`requires`、`handles`、`consists of`、`exhibits`、`is a`、`is an instance of` |
| event | `A initiates P, which consumes A.`、`Miner initiates and handles Copper Mining.`、`Drill initiates Copper Mining, which requires Drill.` |
| condition | `P occurs if A exists, in which case P consumes A, otherwise P is skipped.`、`Testing occurs if Raw Material is pre-tested, in which case Testing changes Raw Material from pre-tested to tested, otherwise Testing is skipped.` |
| self-invocation | 「the process immediately invokes itself」(§22.4.4) |
| 例外リンク | `Overtime Exception Handling occurs if duration of Processing exceeds 60.0 min.` |
| initial・final | `Vehicle is initially intact and finally crashed.` |
| InZoom | `zooms into A, B, and C, in that sequence, as well as X and Y.`、並行は `parallel SP2 and SP3` |
| Fan | `either A or B`、`exactly one of A, B, or C`、`at least one of A, B, or C` |
| reciprocal の tagged | `Sarah and Isaac are family.` |
| 多重度 | `2 to many`、`at least one`、`optional` |
| 並び | 3 つ以上で最後の `and` / `or` の前に `,` |

### 13.2 合わせなかったもの

| 項目 | 入門書 | juiz-OPL | 理由 |
| --- | --- | --- | --- |
| Thing の kind | OPL に書かない(図の形と太字で分かる)。essence は `Vehicle is physical.`、affiliation は `Crashing is environmental and physical.` | `Vehicle is a physical object.` と kind も書く | 図のない文書だけで Object か Process かを決めるため |
| 文頭の State | `Crashed Vehicle initiates ...` と大文字 | `crashed Vehicle initiates ...` と小文字のまま | 大文字にすると Thing の名前と区別できない |
| backward_tag のある tagged | 2 つの文 | `A t B, and B t' A.` の 1 文 | 2 文では片向き 2 本と区別できない |
| 多重度の後の名前 | 複数形(`Front Sensors`) | 名前を変えない | 名前を一意に保つため |
| steps のない Process の InZoom | 例がない | `nothing, as well as` | |
| ordered の aggregation | `..., in that sequence.` | `{ordered: true}` | メタモデルの ordered はリンクごとの値で、文の並び全体には付かないため。合わせ方は Step 5 で見直す |

### 13.3 入門書で確かめられなかったもの(仮置き)

- default の State の文(`State cold of Water is default.`)。
- condition の agent(`A handles P if A exists, otherwise P is skipped.`)と、State を指定した condition の instrument の後半(入門書の本文には `Message Creating occurs if Crash Severity is ...` までの形しかない)。
- undertime の例外の動詞(`falls short of`。入門書の本文の説明の言い回しから取った)。
- duration の書き方(入門書は図で示し、OPL の文がない。今は `{duration_...}`)と経路ラベル(`Following path`)。
- ISO 19450 の本文(OPL の形式文法の附属書があるとされる)か Dov Dori 先生の返事で分かったら、それに合わせ、今の形は入力の別名として残す。
- `{...}` と `[...]`: 交換形式の決め方(xml.md §12)が Dori 先生の返事で変わっても、OPL の注釈の書き方は変えない見込み。名前空間を OPL に書く方法(§10.2 で落としているもの)は、必要になったら決める。

## 14. この Step で決めないこと

- 解析器の誤りの報告の形、警告と誤りの区別、コメントを `fmt` で残すか(Step 5)。
- conf から作るときの OPM の名前の付け方と、同じ名前の Process と Container の分け方(Step 7。metamodel §11)。
- 日本語の文を読み書きの対象にするか(今は §5 の読みを置くだけ)。
- OPD ごとの OPL と、上の階層の図のまとめた線(Step 6)。
