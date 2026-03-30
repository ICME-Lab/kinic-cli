目的:
Insert フォームの Enter 操作に関する案内文言を、実際の target picker 導線に合わせて統一する。

振る舞い:
ヘルプ、ステータス、フォーム下部の案内で、Insert タブの Enter が mode 切替、target picker 起動、submit に使われることを同じ表現で示す。

実装概要:
Insert フォーム向けの共有コピーを `types.rs` に集約し、関連表示からその定義を参照するようにした。合わせて文言の回帰を防ぐテストを追加した。

検証:
`cargo test -p tui-kit-render`

フォローアップ:
なし。
