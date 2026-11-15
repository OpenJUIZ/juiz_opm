# 動いている System からの書き出し

juiz で動いている System を、HTTP の API から読んで juiz-OPL・XML・conf に書き出す決まり。conf だけでは分からないこと([conf.md](conf.md) §8)を、プラグインの manifest から埋める。決めた経緯は [0016](../docs/decisions/0016-system-snapshot.md)。

## 1. 範囲

- 読むのは HTTP ブローカー(`http://<host>:<port>`)が返す profile。juiz の本体には手を入れない。
- 1 つの System を読む。副系(`subsystems`)は警告を出すだけで、中身はその System のスナップショットを別に取る。
- conf にしか無いもの(§3.2)はスナップショットからは分からない。conf と併せて読めば([4.3](#43-conf-とスナップショットを併せる))両方を持つモデルになる。

## 2. スナップショット

動いている System をそのまま読むと、取るたびに値が変わり、テストにも使えない。そこで読んだ profile を 1 つの JSON(スナップショット)にまとめ、書き出しはスナップショットから行う。`juiz_opm fetch <url>` が取る。

```json
{
  "format": "juiz_opm/system/0",
  "system": { ... },          // GET /api/system/profile_full
  "connections": [ ... ],     // GET /api/connection/list の各 ID の /api/connection/profile_full
  "composites": [ ... ],      // 同じく composite
  "transforms": [ ... ],      // 同じく transform
  "pose_listeners": [ ... ]   // 同じく pose_listener
}
```

- `system/profile_full` には接続・Composite の一覧・Transform・PoseListener が入っていないので、それぞれの API で取って足す。
- `format` が違えば警告を出して読み進める。
- スナップショットには引数の今の値(`value`)も入るが、書き出しには使わない。

## 3. スナップショット → conf

`juiz_opm from-system --conf` が書く。結果は conf の正規形([conf.md](conf.md) §3)。

### 3.1 読み方

| conf | スナップショットのどこから |
| --- | --- |
| `name` | 無い(System は `core` としか出ない)。`--name` で与える |
| `processes` | `core_store.processes` と `container_processes`(`container_name` の無いもの)。`publishes`・`subscribes` は Topic 名から先頭の `/` を取る |
| `containers` | `core_store.containers`。持つ ContainerProcess は `container_name` とネームスペースが同じもの。Anchor の `rate` は `anchor.rate` |
| `composite_containers` | 型名が `composite_container` の Container。`slots` の ID はこの System の Container の名前にする |
| `composites` | `composition` を持つファクトリ(と Composite の一覧)。`description` はファクトリの説明 |
| `connections` | 接続の一覧。端が Topic のもの(`publishes`・`subscribes` の配線)は除く |
| `ecs` | `core_store.ecs`。`targets` を `bind` にする。型名が `型:Container` の参照は分けて `container_name` にする |
| `transforms` | Transform の一覧。端の ID はこの System の Container なら名前、別の System なら ID のまま([conf.md](conf.md) §4.2)。単位の変換(0 と `w = 1`)は書かない |
| `pose_listeners` | PoseListener の一覧。`initial`・`on_update` は true のときだけ書く |

### 3.2 スナップショットからは分からないこと

次は profile に出ないので、スナップショットから書いた conf には無い。比べるときは conf の側からも除く(`system::profile_part`)。

- 各項目の params(conf の決まったキー以外)と description。
- `p_apply`(profile には今の値しか出ない)。
- 接続の名前。
- `use_memo` は profile に必ず出るが、conf では書かないことが多いので比べない。
- juiz が埋める既定値: 単位の変換、PoseListener の `initial`・`on_update` の false。

## 4. スナップショット → モデル

`juiz_opm from-system [--xml]` が書く。§3 の conf から [conf.md](conf.md) §4 のとおりにモデルを作り、manifest から次を足す。

### 4.1 足すもの

| 何を | どこに | スナップショットのどこから |
| --- | --- | --- |
| juiz:id | process・container_process・container・composite_container・ec・transform | 各 profile の `identifier` |
| 引数の順(juiz:arg_index) | 引数への instrument | Process の `arguments` の順(manifest の宣言順)。manifest に無い引数はその後ろ |
| つながっていない引数 | value(input) と instrument | `arguments` のうち conf の何もつながっていないもの。名前は `<Process> <引数>` |
| 引数の既定値(juiz:default) | value(input) | `arguments[].default` |
| 引数の型(juiz:datatype) | value(input) | `arguments[].schema` の `name@fingerprint` |
| 出力の型(juiz:datatype) | value(output) | `output_schema` |
| readonly | ContainerProcess と持ち主の間の線 | `readonly: true` なら持ち主からの instrument、でなければ持ち主への effect |
| Composite のノードの引数 | node への instrument | 子の型のファクトリの `arguments` の順。conf に書いてない引数は、その名前で公開した value(input)(juiz の composition.md §3) |

- juiz:datatype は型の完全名に fingerprint を付けた形([metamodel.md](metamodel.md) §9.3)。型なしの引数には付けない。
- つながっている引数の型は値の Object が無いので持たない(送り手の出力の型で分かる)。

### 4.2 モデル → conf

足したものは conf に写らない。value(input) に juiz:applied が無ければ `p_apply` にならず、readonly の instrument も effect も同じ `containers.processes` になる([conf.md](conf.md) §5)。Composite のノードの、名前のとおりに公開した既定値の無い引数は、正規形では書かない([conf.md](conf.md) §3)。だから §4.1 のモデルから書く conf は §3 の conf と同じになる。

### 4.3 conf とスナップショットを併せる

`juiz_opm from-conf --system <snapshot.json> <file.conf>` は conf を読み、スナップショットから §4.1 を足す。conf の params・`p_apply`・description などと manifest の両方を持つモデルになる。このモデルから書く conf は conf の正規形と同じ(§6)。

スナップショットと conf がずれているとき(conf に無い Process がスナップショットにある、など)は、conf の側にあるものだけを書き、ずれは見ない。

## 5. J10

J10([metamodel.md](metamodel.md) §9.6)は、Composite の型の一覧が要るのでモデルだけからは検査しない。`juiz_opm check --system <snapshot.json>` と `check_with_system` は、スナップショットの Composite の型名(`composition` を持つファクトリと Composite の一覧)で J10 を検査する。

## 6. テスト

- `tests/system/<リポジトリ>/<名前>.json` は `tests/conf/<リポジトリ>/<名前>.conf` を動かして取ったスナップショット。juiz_examples_rust の 12 本と juiz_examples_python の 4 本がある(cpp・haskell はプラグインのビルドが要るので後に回す)。transformations_robot は transformations_map の System(port 8000)と並べて動かして取った。
- 次を確かめる(`crates/juiz_opm/tests/system_files.rs`):
  1. スナップショットのモデルはコアの規則と J1〜J13(J10 を含む)を破らず、その OPL は隣の `.opl` と同じ(`JUIZ_OPM_BLESS=1` で書き直す)。OPL と XML から書く conf は同じ。
  2. スナップショットから書く conf は、元の conf から §3.2 を除いたものと同じ。
  3. conf とスナップショットを併せたモデル(§4.3)は J10 を含めて規則を破らず、OPL を経て書く conf は conf の正規形と同じ。
- CI はスナップショットから書いた XML も XSD に通す。
- スナップショットの取り直しは、juiz を `cargo build --release -p juiz_app` でビルドし、サンプルの conf を `juiz -f <conf> -d` で動かして `juiz_opm fetch http://127.0.0.1:8000 -o <名前>.json`。
