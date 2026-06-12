# kakkai

Bevy 製のメタバース基盤。3D ワールドを一人称で歩き回り、自分で Blender 等で
モデリングした家具（.glb / .gltf）をインポートして自由に配置できる。

現在はシングルプレイヤー。ただし全ての状態変更がシリアライズ可能なメッセージ
（`PlaceFurniture` / `MoveFurniture` / `RemoveFurniture`）を経由する設計のため、
将来そのままマルチプレイヤーのプロトコルに転用できる。

## 操作

| 入力 | 動作 |
|---|---|
| クリック | （Walkモード）カーソルをキャプチャ |
| WASD + マウス | 移動・視点 / Shift でダッシュ |
| Esc | カーソル解放 / 配置キャンセル |
| Tab | Walk ↔ Build モード切替 |
| （Build）モデル名クリック | ゴーストプレビュー開始 → 地面クリックで配置 |
| （Build）R | プレビューを45°回転 |
| （Build）家具クリック | 選択 → 移動/回転ギズモ表示 |
| （Build）Backspace / Delete | 選択中の家具を削除 |
| F12 | （devビルド）ワールドインスペクタ |

## モデルの追加

1. Build モードの「Import model…」からファイルを選ぶ、または
2. `~/Library/Application Support/kakkai/models/` に .glb/.gltf を直接置いて「Rescan」

dev ビルドではモデルファイルがホットリロードされるので、Blender から再エクスポート
すると配置済みの家具にも即反映される。

## データの保存先

- モデル: `~/Library/Application Support/kakkai/models/`
- ワールド状態: `~/Library/Application Support/kakkai/world.ron`（配置から2秒デバウンスで自動保存・終了時にも保存）

## 開発

```sh
cargo run                 # dev (dynamic_linking + hot reload + inspector)
cargo run --release --no-default-features   # release 相当
cargo build --profile ci --all-features     # CI と同じビルド
```

## アーキテクチャ

```
src/
  main.rs            # user:// アセットソース登録 → DefaultPlugins → 各プラグイン
  paths.rs           # データディレクトリの解決
  states.rs          # ControlMode { Walk, Build }
  world/             # 地面・ライト・グリッド
  player/            # 一人称キネマティックコントローラ
  furniture/
    components.rs    # FurnitureId(Uuid) + Furniture { model } ← 権威状態
    messages.rs      # Place/Move/RemoveFurniture（Serialize 済み = 将来のネットプロトコル）
    apply.rs         # 状態を書き換える唯一のシステム群
    hydrate.rs       # 状態 → 見た目（glTF シーン or プレースホルダー）
    interact.rs      # プレビュー・picking 選択・ギズモ
  library/           # モデルディレクトリのスキャンと rfd インポート
  persistence/       # world.ron への保存/復元
  ui/                # egui ライブラリパネル
  dev/               # インスペクタ等（dev feature のみ）
```

マルチプレイヤー化の設計方針:
- 権威状態は `FurnitureId + Furniture + Transform` の3コンポーネントのみ。
  見た目（SceneRoot 等）は `hydrate.rs` がローカルに導出する
- 状態変更は必ずメッセージ → `apply.rs` の一本道。サーバー権威にするときは
  メッセージをトランスポートに乗せ、apply をサーバー側で動かすだけ
- 候補: bevy_replicon / lightyear
