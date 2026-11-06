# 0004 モデル交換形式は XML で、ISO 19450 の用語と juiz の名前空間を使う

- 状態: 採用(Dov Dori 先生への問い合わせの返事次第で見直す)
- 日付: 2026-11-06

## 背景

OPL がモデル情報の交換言語になり得るかを確かめた結果、全単射は「juiz-OPL と juiz の構成」の間でだけ成り立たせられる([spec/opm-mapping.md](../../spec/opm-mapping.md) §7)。文は人が読むには向くが、道具の間の交換には構造化した形式が要る。ysuga の判断で、まず XML ベースの交換形式を規定する。

OPM の標準的な XML 交換形式は見つからなかった([docs/research/prior-art.md](../research/prior-art.md))。2026-11-06 に ysuga が Dov Dori 先生に、交換形式の発表や計画があるか問い合わせた。

## 決めたこと

- 交換形式は XML とし、XSD を付ける。
- 要素名と種類は数字ではなく ISO 19450 の用語(object、process、state、instrument など)にする。
- juiz 固有の情報(pull・push、Topic、readonly、型名、実装状況など)は別の XML 名前空間に入れる。juiz の名前空間を無視すれば素の OPM として読める。
- モデルと図(配置)は分ける。OPCAT の OPX と同じ考え方([docs/research/opcat-opx.md](../research/opcat-opx.md))。
- 全要素に一意の ID を付ける。後で RDF(OPCloud の OSLC 連携)へ写せるようにするため。

## 影響

- OPL は交換形式の「読める表現」の 1 つとし、XML と相互に変換できるようにする。
- Dori 先生から既存の形式を教わった場合は、その形式との対応を調べ、この決定を置き換えるか決める。
