# 0018 コマンドは juiz_opm 1 本にまとめ、タグを打つと CI が 4 種類の配布物を作る

- 状態: 採用(案)
- 日付: 2026-11-16

## 背景

Step 9 までで、道具は 3 つのコマンドに分かれていた。`opm_opl`(OPL の検査・整形・XML との変換)、`opm_graph`(図と OPX)、`juiz_opm`(conf との変換と juiz の規則)。使う人は、どの作業がどのコマンドかを覚え、3 つを入れる必要があった。入れ方は `cargo run` しか書いておらず、Rust の無い人は使えなかった。

ysuga は「ツールとして確立させたい」と言い、Step 9 の作業で提案した 1〜3(コマンドの統一、配布、使い方の文書)を進めることになった。

## 決めたこと

1. **入口は `juiz_opm` 1 本にする。** 次を足した。
   - `juiz_opm opl <check|fmt|to-xml|from-xml> ...`: `opm_opl` と同じ。
   - `juiz_opm draw ...`: `opm_graph` と同じ。
   - `juiz_opm opx <model> [-o <out>]`: `draw -T opx` の短縮。出力が `.opz` で終わるときは gzip。
   - `juiz_opm --version`。
   conf の命令(`from-conf` など)は前のまま。
2. **`opm_opl` と `opm_graph` のコマンドも残す。** 中身は各クレートの `cli::run(prog, args)` に移し、どのコマンドもそれを呼ぶ。使い方の表示と誤りの文頭は、呼ばれた名前(`juiz_opm opl` など)にする。
3. **配布物は GitHub Actions で作る**(.github/workflows/release.yml)。`v*` のタグを押すと、Linux x86_64、macOS arm64・x86_64、Windows x86_64 の 4 つをビルドし、3 つのコマンド、README、ライセンス、docs/usage.md を圧縮して Release に付ける。手で走らせたときは workflow の成果物に置くだけ。
4. **ソースからは `cargo install --git https://github.com/OpenJUIZ/juiz_opm juiz_opm` で入れる。** crates.io への公開は、版の上げ方と名前を決めてからにする(今はしない)。
5. **使い方は docs/usage.md に、入れ方から conf → 検査 → 図 → OPX → conf までの順で書く。** README には短い例と英語の概要を置く。文書の本体は日本語のまま([CLAUDE.md](../../CLAUDE.md))。

## 理由

- 使う人が覚える名前が 1 つで済む。juiz 本体に `juiz opl` を足すとき(Step 10)も、1 つの入口を呼べばよい。
- 旧コマンドを残すと、これまでの手順や CI がそのまま動く。中身を 1 か所にしたので、2 つが食い違わない。
- 依存は純粋な Rust(ureq は TLS なし、flate2 は miniz_oxide)なので、各 OS で同じ手順でビルドできる。macOS の x86_64 は arm64 の上でクロスビルドする。
- crates.io は名前を取ると戻せないので、急がない。

## 影響

- opm_opl・opm_graph の `src/main.rs` を `src/cli.rs`(`pub fn run`)に移し、main.rs はそれを呼ぶだけにした。
- juiz_opm の依存に opm_graph を足した。
- crates/juiz_opm/tests/command.rs で、`opl`・`draw`・`opx`・`--version` と conf の命令を確かめる。
- 最初の配布物は ysuga がタグ(例: `v0.1.0`)を押したときにできる。
- roadmap に 9.5 として足した。
