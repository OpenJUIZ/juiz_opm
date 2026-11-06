# 0006 交換形式の XML はメタモデルの名前をそのまま使い、注釈は juiz の名前空間の属性にする

- 状態: 採用(案。名前空間と要素名は Dov Dori 先生の返事次第で見直す)
- 日付: 2026-11-06

## 背景

[0004](0004-xml-interchange.md) で交換形式を XML にし、[0005](0005-metamodel-layers.md) でモデルが持つものを表にした。roadmap の Step 2 として、その表を XML と XSD に写す。Dori 先生への問い合わせの返事はまだないが、ysuga の指示(2026-11-06)で、返事次第で変わる点を仮置きにして進める。

## 決めたこと

詳細は [spec/xml.md](../../spec/xml.md)。

1. **名前空間を 2 つにする。** コアと図は `https://openjuiz.github.io/ns/opm/v0`(接頭辞 `opm`)、juiz 拡張は `https://openjuiz.github.io/ns/juiz/v0`(接頭辞 `juiz`)。スキーマは opm.xsd と、それを import する juiz.xsd。
2. **名前はメタモデルの名前をそのまま使う。** `from_state`、`in_zoom`、`juiz:value_kind` のように小文字と `_`。対応表を別に持たずに済み、juiz の conf とも揃う。
3. **リンクと Thing は種類ごとに要素を分ける。** `<opm:instrument>`、`<opm:object>` など。種類ごとの属性(`modifier`、`from_state`、`tag`)と参照先の種類を XSD で縛れる。
4. **id は ASCII の名前(`[A-Za-z_][A-Za-z0-9_.-]*`)にし、`xs:ID` で一意を検査する。** URI の末尾にそのまま使えるようにするため(metamodel §3.2 で Step 2 に残した点)。
5. **参照先の種類は keyref で検査する。** C1〜C4 と、C5・C6・C9〜C11 の一部を XSD で見る。残りは検査器(Step 4、Step 7)。XSD 1.0 の範囲に留め、libxml2 で検査できるようにする。
6. **State は持ち主の Object の子、OPD の木は要素の入れ子で書く。** owner と parent を属性に持たず、XML の形で一意に決める。根の OPD は `refinement` を書かない。
7. **juiz の注釈は、値が 1 つなら juiz の名前空間の属性、構造があれば子要素にする。** 子要素は `juiz:value`・`juiz:applied`(JSON)と `juiz:translation`・`juiz:rotation`。複数の値は空白区切りの並び。
8. **他の名前空間は `lax` で受ける。** 知らない注釈は読み飛ばせ、juiz の名前空間を取り除いた文書は opm.xsd に通る。代わりに juiz の注釈の綴りの誤りは XSD では見つからず、Step 7 の検査器が見る。
9. **版は `format_version` 属性と名前空間の `v0` で表す。** 0.x の間は互換性のない変更をしてよい。

## メタモデルへの反映

Step 2 で表を見直した結果、次の 2 点を metamodel.md に足した。どちらも 0005 の決定の中身は変えない。

- `juiz:type` は role `composite` の Thing にも付ける(Composite の型名)。J10 の「その名前の role composite の Thing」は `juiz:type` で照らし合わせる。
- `juiz:datatype` の値の形は juiz の書き方(完全名か `<完全名>@sha256:<64 桁>`)とする。

## 理由

- 種類ごとの要素にすると、XSD が「その種類で書けること」を直接表せ、読み手も要素名だけで意味が分かる。`kind` 属性 1 つにすると、XSD 1.0 では種類ごとの属性を縛れない。
- 注釈を属性にすると、juiz の名前空間を無視したときに残るものがそのまま素の OPM になる。

## 影響

- Step 4 の opm_model はこの XSD に通る文書を読み書きし、XSD で見ない規則を検査する。
- Dori 先生から既存の形式を教わった場合は、spec/xml.md §12 の点を新しい決定で置き換える。メタモデルと注釈の中身は残す見込み。
