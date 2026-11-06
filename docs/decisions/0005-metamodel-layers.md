# 0005 メタモデルはコア・図・juiz 拡張の 3 層にし、標準で表せることは注釈にしない

- 状態: 採用(案。Step 2 の XML を書くときに見直す)
- 日付: 2026-11-06

## 背景

交換形式(XML、[0004](0004-xml-interchange.md))と juiz-OPL の文法の前に、モデルが何を持つかを書き方から切り離して決める必要があった(roadmap の Step 1)。全単射を確かめるには、同じ事実を 2 か所に持たないことが大事になる。

## 決めたこと

詳細は [spec/metamodel.md](../../spec/metamodel.md)。

1. **3 層に分ける。** ISO 19450 の部分集合のコア、OPD の木と配置の図、`juiz:` の注釈と juiz の整形式の規則からなる juiz 拡張。注釈を捨てるとコアとして正しいモデルになる。
2. **種類は ISO 19450 の名前で持つ。** OPCAT の番号は対応表に書くだけにする。
3. **in-zooming はモデルの事実としてコアに置く。** 中の Process の順序(並行を含む)と中の Thing を InZoom が持つ。unfolding は図の層だけ。
4. **線は本当の端で 1 回だけ持つ。** in-zoom した Process への線は中の Process に付け、上の階層の図の「まとめた線」は描くときに導く。
5. **状態変化は effect 1 本に from_state・to_state を付ける。** 1 文 1 事実にそろえるため。
6. **標準で表せることは標準で表す。** readonly は instrument、書き込みは effect、Composite は InZoom。注釈は pull・push、引数名、型名、Topic などだけにする。
7. **型は注釈で持つ。** Process・Container の type_name とデータ型は `juiz:type`・`juiz:datatype` で持ち、分類リンクは使わない([0002](0002-open-points.md) の 4)。ただし Composite の型は中身を持つので Thing(role composite)にする。conf に型だけあって実体がない場合もあるため。
8. **push は注釈と invocation の両方に出す。** juiz の接続は (送り手, 引数, 受け手) ごとなので注釈が正本。素の OPM として読む人のために invocation も持ち、両者の一致を規則(J4)で縛る。
9. **戻り値の Object はいつも持つ。** 図に出すかは OPD が決める。正規形を 1 つにするため。

## 理由

- 書き方から切り離せば、XML・OPL・Rust の型が同じ表を写すだけになり、往復テストの基準が 1 つになる。
- 同じ事実を 2 か所に持つと、片方だけ直した不整合が起き、全単射が崩れる。例外は 8 だけで、規則で縛る。

## 影響

- Step 2 の XSD、Step 3 の文法、Step 4 の opm_model はこの表に合わせる。
- 決めていないことは spec/metamodel.md §11 に残した。
