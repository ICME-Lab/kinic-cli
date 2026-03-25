# edit log (feat/tui-memory-create)

## 目的

タブ切替後のフォーカスがプレースホルダー画面でも memories 用のまま残る問題を修正し、不要になったテーマ切り替え導線を表示・実装の両方から外す。

## 振る舞い

- `SetTab` 成功後、`tab_kind` に応じて `PaneFocus` を正規化（Memories→List、Form→Tabs＋create フィールドリセット、プレースホルダー／Unknown→Tabs）。
- `t` によるテーマ循環を廃止し、Settings オーバーレイは閉じ方だけを案内する。

## 実装概要

- `runtime_loop.rs`: `normalize_focus_after_set_tab`、`open_form_tab` と `dispatch_action` の `SetTab` 成功パスでフォーカス正規化を適用し、`Theme` は固定値へ戻した。
- `lib.rs`: `HostGlobalCommand::ToggleTheme` と `global_command_for_key` の `t` 分岐、関連テストを削除した。
- `overlays.rs` / `types.rs`: Settings からテーマ表示を外し、`close_hint` を `Esc` のみに戻した。

## 検証

- `cargo test -p tui-kit-host --lib`
- `./.local/check.sh`

## フォローアップ

- 効果経路（例: `ResetCreateFormAndSetTab`）でのフォーカス正規化が必要なら共通ヘルパを再利用可能。

## 目的

フルスクリーンタブ化した `Create` / `Market` / `Settings` で、表示されていないペインへフォーカスが移る不整合と、非同期検索完了が別タブ閲覧中のフォーカスを奪う問題を解消する。

## 振る舞い

- `Create` タブでは `Tabs` / `Form` / `Chat` 以外へフォーカスが移らない。
- `Market` / `Settings` タブでは `Tabs` / `Detail` / `Chat` 以外へフォーカスが移らない。
- 検索完了時の `List` フォーカス移動は `Memories` タブ表示中に限って発生し、他タブ閲覧中はステータス更新だけを行う。

## 実装概要

- `tui-kit-runtime/src/lib.rs`: `apply_core_action` 後にタブ種別ごとのフォーカス正規化を追加し、create/placeholder タブで非表示ペインへの遷移を抑止した。
- `tui-kit-host/src/lib.rs`: `SearchCompleted` の `PaneFocus::List` 反映を `Memories` タブ限定にした。
- 両クレートに回帰テストを追加し、create/placeholder のフォーカス制約と off-tab 検索完了時の挙動を固定した。

## 検証

- `cargo test -p tui-kit-runtime -p tui-kit-host`

## フォローアップ

- `Memories` 以外のタブが将来インタラクティブ化した場合は、タブ種別ごとの許可フォーカス集合をその画面仕様に合わせて拡張する。

## 目的

タブごとのフォーカス可否を runtime に集約し、検索完了 effect から UI ナビゲーションを分離して、将来のタブ実装変更でも hidden pane への遷移や off-tab のフォーカス奪取が起きにくい形へ整える。

## 振る舞い

- タブの操作可否は `TabFocusPolicy` で一元管理され、runtime と host が同じ定義を参照する。
- `Create` / `Market` / `Settings` の visible pane 制約は policy に従って適用され、`FocusPane` も hidden pane には反映されない。
- 検索完了は `Notify` / `SelectFirstListItem` / 条件付き `FocusPane(List)` に分離され、`Memories` 表示中だけ一覧へフォーカスする。

## 実装概要

- `tui-kit-runtime/src/lib.rs`: `TabFocusPolicy` と `tab_focus_policy()` を追加し、フォーカス正規化と chat close 復帰先を policy ベースへ置き換えた。
- `tui-kit-host/src/lib.rs`: `execute_effects_to_status()` の `FocusPane` 適用を policy でガードし、`global_command_for_key()` の `Esc` 判定も `allows_form` / `allows_search` / `allows_list` ベースへ寄せた。
- `tui/src/provider.rs`: 背景検索完了時の effect を `SearchCompleted` から分離し、off-tab では `FocusPane(List)` を出さないようにした。

## 検証

- `cargo test -p tui-kit-runtime -p tui-kit-host`
- `cargo test -p kinic-tui`

## フォローアップ

- `Market` / `Settings` が実操作画面になったら `TabKind` ではなく `TabFocusPolicy` の許可集合だけ更新すればよい構成になっている。

## 目的

`tui-kit-runtime` に残っていた旧 create modal API を取り除き、Kinic 向けのフォームタブ移行後に不要な共有状態を減らす。

## 振る舞い

- runtime の `CoreState` から `create_modal_open` を削除した。
- runtime の `CoreAction` から `OpenCreateModal` / `CloseCreateModal` を削除した。
- `rust_inspector` example は create overlay の開閉だけをローカル state で持ち、共有 runtime には依存しないようにした。

## 実装概要

- `tui-kit-runtime/src/lib.rs`: 旧 modal 専用 state / action と reducer 分岐を削除した。
- `tui-kit-host/src/form_tab_flow.rs`: create form reset から不要な modal flag 書き換えを外した。
- `tui/examples/rust_inspector/app/interaction.rs`: create overlay の open/close を example 内の state 更新へ寄せ、core state との同期項目から modal flag を外した。

## 検証

- `cargo test -p tui-kit-host --lib`
- `./.local/check.sh`

## フォローアップ

- `rust_inspector` example 自体も将来フォームタブへ寄せるなら、`create_modal_open` を含むローカル overlay 実装をさらに整理できる。

## 目的

`Create` 画面で描画レイアウトとカーソル座標計算が別々に同じ知識を持っていた状態を解消し、文言や余白変更時の座標ズレを防ぐ。

## 振る舞い

- `Create` 画面の intro/form 分割と form 内カーソル基準位置は共通レイアウトから導出される。
- カーソル計算は root area から body area を求めた上で、描画と同じ form inner area を参照する。

## 実装概要

- `tui/crates/tui-kit-render/src/ui/app/screens/create/mod.rs`: `CreateScreenLayout` を追加し、`render_create_screen` と `create_cursor_position_for_area` の両方で共有するようにした。
- 同ファイルのカーソル座標計算は共通レイアウト経由に置き換え、border 内側矩形と field row index を 1 箇所へ集約した。

## 検証

- `cargo test -p tui-kit-render create_cursor_positions_follow_field_order`

## フォローアップ

- 将来 `Create` フォームの行構成自体を変える場合は、field row index も同じ共通レイアウトの責務としてさらに明示化できる。

## 目的

`Create` 画面で残っていた field row のマジックナンバー管理を解消し、描画本文とカーソル位置を同じ行定義から導出できるようにする。

## 振る舞い

- `Name` / `Description` / `Submit` の描画行とカーソル対象行は同じフォーム行定義に従う。
- error 表示の有無や tabs の有無があっても、既存フィールドのカーソル位置は安定する。

## 実装概要

- `tui/crates/tui-kit-render/src/ui/app/screens/create/mod.rs`: `CreateFormRow` と `CreateFormLines` を追加し、フォーム本文の `Line` 列と focus 対応行 index を共通生成するようにした。
- 同ファイルの `render_create_screen` と `create_cursor_position_for_area` は両方とも共通のフォーム行定義を利用し、個別の row index `match` を削除した。

## 検証

- `cargo test -p tui-kit-render ui::app::screens::create`

## フォローアップ

- 将来 field ごとに異なる横オフセットや複数行入力を導入する場合は、`CreateFormRow` に cursor x/y のメタデータを持たせる形へ拡張できる。

## 目的

`Create` 画面の render/cursor 共通化を維持しつつ、過剰だった抽象化を落として読みやすく保つ。

## 振る舞い

- `Create` フォーム本文と focus 行位置は同じ `create_form_lines()` から導出される。
- 共通化の保証は維持したまま、行 enum や個別 render 分岐なしで追える実装になった。

## 実装概要

- `tui/crates/tui-kit-render/src/ui/app/screens/create/mod.rs`: `CreateFormLines` を `lines + 各 focus の row index` だけ持つ軽量構造へ簡素化した。
- `CreateFormRow` とその `render()` 実装は削除し、フォーム行の組み立てと row 記録を `create_form_lines()` に集約した。
- create 画面テストは `tests.rs` に分離し、`mod.rs` 本体は 300 行に収めた。

## 検証

- `cargo test -p tui-kit-render ui::app::screens::create`

## フォローアップ

- 行構成がさらに複雑になるまでは、今回のような「組み立て時に row index も記録する」程度の薄い共有で維持するのが妥当。

## 目的

`Create` 画面に残っていた横方向の暗黙知と、screen state 導出の重複を小さく整理する。

## 振る舞い

- 入力行のインデント幅は定数で管理され、render 側の空白と cursor 側の `x` 計算が同じ値を使う。
- body/root どちらからでも `Create` 画面の layout と form lines を同じ `CreateScreenState` で導出する。

## 実装概要

- `tui/crates/tui-kit-render/src/ui/app/screens/create/mod.rs`: `CreateScreenLayout::INPUT_INDENT_WIDTH` を追加し、入力行の先頭空白も cursor の `field_x` もこの値基準にした。
- 同ファイルに `CreateScreenState` を追加し、render/cursor の両方が `layout + form_lines` を同じ導出経路から取得するようにした。

## 検証

- `cargo test -p tui-kit-render ui::app::screens::create`

## フォローアップ

- いまの `Create` 画面規模では、この程度の軽い共有で十分で、さらに抽象化を増やす必要はない。

## 目的

`tui/src/provider.rs` の live search で、遅れて返ってきた古い検索結果が現在の UI を上書きする不整合と、検索失敗時に直前の結果が残る問題を防ぐ。

## 振る舞い

- 検索開始時の request id / memory id / query を保持し、完了時に現在 pending な検索と一致する結果だけを反映する。
- query 変更、memory 選択変更、Browser への復帰後に遅延完了した検索結果は黙って破棄する。
- 検索失敗時は search results をクリアして Browser 表示へ戻し、古い結果を残さない。

## 実装概要

- `tui/src/provider.rs`: `SearchRequestContext` と request id を追加し、`run_live_search` / `poll_background` / query・memory 遷移時の pending search 無効化を実装した。
- 同ファイルの provider テストを拡張し、stale result、失敗時クリア、query clear 後の遅延完了、worker disconnect の各ケースを固定した。

## 検証

- `cargo test provider::`
- `./.local/lint.sh`

## フォローアップ

- 今回は UI 反映だけを防いでおり、検索ワーカー自体のキャンセルまでは行っていない。必要になったら request id 管理を土台に中断制御を追加できる。

## 目的

Create 画面の render/cursor 共通化と live search の stale result 対策を仕上げつつ、周辺テストを読みやすい粒度に整理する。

## 振る舞い

- `Create` 画面は body/root どちらからでも同じ screen state を導出し、フォーム本文とカーソル位置が同じ行定義に従う。
- live search は request id と検索文脈が一致する結果だけを反映し、query や選択 memory が変わった後の遅延結果は破棄する。
- 検索失敗時は古い results を残さず browser 表示へ戻る。

## 実装概要

- `tui/crates/tui-kit-render/src/ui/app/screens/create/mod.rs`: `CreateScreenState` / `CreateScreenLayout` / `CreateFormLines` で render と cursor の共通導出を整理し、create 画面テストを `tests.rs` へ分離した。
- `tui/src/provider.rs`: `SearchRequestContext` と request id を追加し、pending search の無効化と stale result 破棄、失敗時の result clear を実装した。
- `tui/crates/tui-kit-host/src/lib.rs`: host テストを責務ごとの module に整理し、既存挙動の確認粒度を揃えた。

## 検証

- `cargo test provider::tests -- --nocapture`
- `cargo test -p tui-kit-render create::tests -- --nocapture`

## フォローアップ

- stale search は UI 反映だけ止めているため、必要になればワーカー側キャンセルの導入余地がある。
