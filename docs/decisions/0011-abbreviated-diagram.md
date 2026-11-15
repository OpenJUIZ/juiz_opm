# 0011 図は juiz の略式図 1 種類にし、何を描くかは図の作成者が選ぶ

- 状態: 採用(案。Step 6 で描く道具を作るときに見直す)
- 日付: 2026-11-15
- 置き換えるもの: [spec/opm-mapping.md](../../spec/opm-mapping.md) §6 の「モデルの要素を Graphviz の形で 1 つずつ描く」描き方

## 背景

チュートリアルの 3 章と 4 章の System を、今のメタモデルのまま OPM の図にしたところ、Process 4 つの System が箱 15 個と線 20 本ほどになった。引数と戻り値がどれも value の Object と線になり、System の aggregation・exhibition、EC の ec_cycle と State、Transform の Object と tagged 2 本が、どの図にも入るためである。描く道具(Step 6)を作っても、要素を 1 つずつ描く限り数は減らない。

ysuga と図の見本を描き比べ、次の方針にした(2026-11-15)。

- 完全に OPM に寄せるのではなく、略式の図法を考える。
- 図の要素は書いても省いてもよい。モデルとしては全部を保ちつつ、図は見やすくする。
- ビューの種類は増やさない。ただし、図の作成者が意図を持って描けるような規則にする。

## 決めたこと

詳細は [spec/diagram.md](../../spec/diagram.md)。

1. **モデルは完全な OPM のまま保つ。** XML と juiz-OPL は今のメタモデルを書く。略式図はモデルから決まった規則で描くもので、図からモデルを作らない。
2. **図の描き方は略式図 1 種類にする。** 決まった形の要素の組(Process と引数・戻り値の value、push 接続の 3 本、EC と ec_cycle、Transform と tagged 2 本など)は、まとめて 1 つの記号で描く。組にならない Thing とリンクは OPM の形で描く。juiz の注釈のないモデルは、まとめる組がないので OPM の図になる。
3. **何を描くかは図の作成者が選ぶ。** 既にある図の層(OPD と Appearance、metamodel §7.2)を使う。OPD に Appearance を載せた Thing だけを描く。リンクは、両端が描かれていて、OPD の `suppressed_links` に挙げられていないときに描く。OPD がないモデルは全部を描く。
4. **省くのは自由だが、描いたものはモデルどおりにする。** 省いたことで、描いたものの意味が変わってはいけない(例: push 接続の線を描くなら、その 3 本の組が揃っていること)。省いたことを図に示す記号は持たない。
5. **メタモデルに 3 つ足す。**
   - OPD の `suppressed_links`(ref(Link) の集合)。両端が出ていても描かないリンク。
   - `juiz:params`(Thing、JSON のオブジェクト)。Container・CompositeContainer・Process・ContainerProcess を作るときに conf で渡す値(`initial_value: 3` など)。
   - `juiz:default`(value(input)、JSON の値)。引数の既定値。conf ではなく manifest から来る。
   - あわせて、pose_listener の `on_update`(transformations.md §8.3)を `juiz:on_update`(真偽)として足す。
6. **見本の形** は spec/diagram.md §3 のとおり。Process は楕円、引数は楕円の左の輪郭の小さな丸、戻り値は右端の小さな黒い三角、注記は丸からぶら下げる。Container は ContainerProcess を中に並べた箱。Timer 系の EC は started の State から `periodically invokes` の線を引く。System は枠。Transform は中ほどに白抜きの三角を置いた太い青の線で、矢じりを付けない。

## 理由

- 交換と往復(Step 5、Step 7)の基準は完全なモデルのままにしたい。図のために情報を落とすと、全単射の確認が崩れる。
- 図の種類を増やすと、どの図を見ればよいかを利用者が選ぶ手間が増える。1 種類にし、描く範囲だけを作成者に任せれば、規則は 1 つで済む。
- 何を描くかは、OPCAT や OPCloud と同じく OPD と Appearance で持てる。新しい仕組みは `suppressed_links` だけで済む。
- 「periodically invokes」は OPM にない関連(State は Process を起こせない)なので、モデルには入れず、図の記号に留める。

## 影響

- metamodel §7.2・§9.3・§9.4・§10、xml.md §6.6・§7.2、opl.md §7、opm.xsd・juiz.xsd に反映した。opm_model は `Diagram::suppressed_links` を読み書きし、C10 で参照先がリンクであることを見る。
- 例に [examples/xml/tutorial.opm.xml](../../examples/xml/tutorial.opm.xml) と [examples/opl/tutorial.opl](../../examples/opl/tutorial.opl) を足した。
- roadmap の Step 6 は、この略式図を描く道具にする。Step 5(opm_opl)は変わらない。
- 残る点: System をまたぐ Transform(transformations.md §19)のメタモデル、注記を線と重ならないように置く規則、`suppressed_links` を OPL に書くか(今は書かない。図の層は OPL に書かない、opl.md §1)。
