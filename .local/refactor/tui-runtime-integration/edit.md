目的
- live TUI のメモリ一覧で、検索フィルタ中も選択対象と検索対象が一致するようにする。
- ローカル ledger セットアップで minter 用 identity を分離し、通常利用 identity と役割を分ける。

振る舞い
- live モードのメモリ一覧では、検索フィルタ後に見えている候補だけを選択移動対象にする。
- Enter による検索は、現在表示中の候補に対応する memory canister を対象に実行する。
- ローカル setup は `local-minter` を既定の minter identity として使い、初期残高は元のユーザー principal に配る。

実装概要
- TUI provider に可視メモリ一覧と active memory の同期処理を追加し、選択移動と検索対象解決をフィルタ済み一覧基準に変更した。
- provider の回帰テストを追加し、フィルタ時の選択維持・移動・検索対象決定を検証した。
- `scripts/setup.sh` と `scripts/mint.sh` を更新して、minter identity を分離したローカル ledger 構成に合わせた。

検証
- `cargo test --lib tui::provider::tests -- --nocapture`

フォローアップ
- `scripts/mint.sh` の利用者向け説明は、通常利用で第3引数を意識させない文面に整理の余地がある。
