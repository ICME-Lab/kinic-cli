目的:
insert タブのモード設計を入力方法ベースに整理し、`Normal` に text と file path が同居する分かりにくさを解消する。

振る舞い:
insert タブを `Text / File / Raw` の 3 モードへ再構成し、`File` モードでは `.pdf` を PDF 経路、それ以外を通常ファイル読込経路として扱う。

実装概要:
runtime の insert mode とフォーカス順を更新し、TUI の描画と provider の request 組み立てを新モードに合わせて調整した。TUI 側では `Text` と `File` に専用のバリデーション文言を追加し、モード別の request 分岐と描画差分をテストで固定した。

検証:
`cargo test`

フォローアップ:
必要なら CLI 側の `insert` / `insert-pdf` も同じ入力モデルに寄せるかを別途検討する。
