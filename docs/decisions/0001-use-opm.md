# 0001 juiz の設計を語る言語に OPM を使う

- 状態: 採用
- 日付: 2026-11-06

## 背景

juiz のシステムの設計や実装状況について、図で会話するための共通の言語が必要になった。候補は OPM、SysML v2、C4、UML、Mermaid / PlantUML。

## 決めたこと

OPM(Object-Process Methodology, ISO 19450)を使う。意味の決まりは OPM、正本は OPL 風の文、描画は Graphviz とする。

## 理由

- juiz の「Process(冪等な関数)」と「Container(状態)」の分け方が、OPM の Process と Object の分け方と重なる。UML / SysML は振る舞いが構造に属する作りで、Process を主役にする juiz とずれる。
- 引数 = instrument、戻り値 = result、ContainerProcess の書き込み = effect、Composition = in-zooming と素直に対応する。
- OPL(図と一対一の文)があるので、Claude が文で書き、人が図と文の両方で確かめられる。
- C4 は配置には向くが Process と Container の意味を表せない。Mermaid / PlantUML は意味の決まりがない。SysML v2 は重い。

## 影響

- 対応の詳細は [spec/opm-mapping.md](../../spec/opm-mapping.md)。
- OPM が苦手な配置(System とブローカーのつながり)は別の図にする([0002](0002-open-points.md) の 3)。
