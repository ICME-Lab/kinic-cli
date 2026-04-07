# Kinic TUI

This is a terminal UI for operating Kinic memory canisters. You can run `list`, `search`, `create`, `insert`, and settings changes from one screen without remembering every command each time.

## Install

Kinic TUI is included in `kinic-cli`. For OSS users, the standard path is to download a release binary and run it directly.

1. Download the `kinic-cli` release binary for your environment from GitHub Releases or another distribution page
2. Place the extracted binary somewhere executable
3. If you want to use the file chooser, install `yazi` as needed
4. Start the TUI from your terminal with `kinic-cli --identity <name> --ic tui`

Standard example for connecting to the Internet Computer mainnet:

```bash
kinic-cli --identity alice --ic tui
```

To connect to a local replica:

```bash
kinic-cli --identity alice tui
```

> `--identity` is required. The TUI does not support `--ii` yet.
>
> `yazi` is required to open the chooser in File mode. If it is not installed, you can still type the file path manually. Example for macOS:
>
> ```bash
> brew install yazi
> ```

If you want to start it directly from source during development, you can also run:

```bash
cargo run -- --identity alice --ic tui
```

## Prerequisites

- `dfx 0.31+`
- `yazi` if you want to use the chooser in File mode
- On macOS, the PEM for the dfx identity specified with `--identity` must be stored in Keychain
- Have KINIC tokens ready in advance if you plan to create memories
- When using a local environment, make sure the local replica and supporting canisters are already running

For detailed local environment setup, see [docs/cli.md](./cli.md). In particular, the prerequisites for the local replica, supporting canisters, and identity are the same as for the CLI.

## Quickstart

This is the shortest path to try the main flow end to end.

1. Start the TUI with `kinic-cli --identity alice --ic tui`
2. Open the `Settings` tab and confirm `Principal ID` and `KINIC balance`
3. If needed, transfer KINIC tokens to that principal
4. Press `Ctrl+R` in the `Settings` tab and confirm that the balance is updated
5. Create one new memory in the `Create` tab
6. In the `Memories` tab, confirm that the new memory appears in the list
7. Move to the `Insert` tab and add text or a file to that memory
8. Return to the `Memories` tab and search for the inserted content

For an initial check, it is easiest to follow the flow by inserting a short string with `Inline Text` and then searching for it.
If you want to try File mode, either install `yazi` first or type the file path manually.

## What You Can Do

- List existing memories
- Search across all memories or within a single memory
- View memory details
- Ask AI about the selected memory or all searchable memories from the chat panel
- Create a new memory
- Insert text, files, or manual embeddings
- Manage the default memory and saved tags

## Layout

The TUI has five tabs.

- `Memories`: list, search, and details
- `Insert`: add data
- `Create`: create a new memory
- `Market`: reserved for future use and currently not implemented
- `Settings`: view and change current connection info and saved settings

The `Memories` tab opens first when the TUI starts.

## Basic Controls

- `1` to `5`: switch tabs
- `Tab`: move focus forward within the screen
- `Shift+Tab`: move focus backward
- `/`: focus the search field
- `↑` `↓`: move through lists and form fields
- `Enter`: open, confirm, or submit the current item
- `Esc`: go back one step, return to the list, or close the picker
- `?`: show help
- `q`: quit
- `Ctrl+N`: open the `Create` tab
- `Ctrl+R`: refresh the current view
- `Shift+C`: toggle the chat panel in the `Memories` tab
- `Shift+S`: toggle the settings overlay (session info and saved settings)

The status bar at the bottom also shows the keys available in the current context.

`?` and `q` are intended for normal list and tab navigation, not while typing in a search field or form, and not while the chat input is focused.
`Shift+C` (toggle chat on `Memories`) follows the same focus rules.
`Shift+S` toggles the settings overlay from the search field, lists, and other panes, but not while editing a form field or while the chat input is focused.
In multiline text fields, `Enter` inserts a newline instead of submitting. Move to `Submit` with `Tab` or `Shift+Tab` and press `Enter` there.

## Common Workflows

### Search an Existing Memory

1. Open the `Memories` tab
2. Select the target memory in the list
3. If needed, switch the search scope to `selected memory` with `←` `→`
4. Enter a query in the search field and press `Enter`
5. Press `Enter` on a result to open its details

### Set the Default Memory

1. Select the target memory in the `Memories` tab
2. Press `Shift+D`
3. After that, it will be used as the placeholder candidate when `Memory ID` is left empty in the `Insert` tab

### Insert a PDF

1. Move to the `Insert` tab
2. Set `Mode` to `File`
3. Enter `Memory ID` and `Tag`
4. Select a PDF file or type its path
5. Run it with `Submit`

PDFs are converted to Markdown before insertion. This will fail if `pdftotext` is not available in your environment.

### Reuse Saved Tags

1. Select `Tag` in the `Insert` tab
2. Choose the tag you need from the saved tag list
3. If needed, save a new tag with `+ Add new tag`
4. Delete unnecessary tags from tag management in `Settings`

## Memories Tab

In the `Memories` tab, you work with memories using the search field, list, and detail pane.
Press `Shift+C` to toggle the chat panel when focus is not on the search field, a form, or the chat input (see **Basic Controls**).
The chat panel asks AI against either all searchable memories or one memory from the visible items list, restores local history separately for each chat scope, rewrites follow-up prompts like `that one` into a standalone search query before retrieval, and for `all memories` reranks search hits before diversity-aware selection builds the final answer prompt. Chat history is now stored per `network + identity + chat context + thread`, so `all memories` and each scoped memory can keep multiple local threads.

![Memories tab screenshot](./images/tui-memories.png)

- Type in the search field and press `Enter` to search
- Switch the search scope with `←` `→`
  - `all memories`: search across every memory
  - `selected memory`: search only the currently selected memory
- In the chat pane, press `Enter` to send
- In the chat pane, press `Shift+←` or `Shift+→` to cycle the chat scope through `all memories` and the visible memory items
- In the chat pane, type `/new` or `/all` and press `Enter` to run the matching chat command
- When slash commands are visible above the chat input, use `↑` `↓` to move and `Enter` to select
- The TUI restores the last thread used for each scoped memory and for `all memories`
- v1 does not include a thread list yet, so you cannot reopen older threads from the UI
- Existing saved chat history is not migrated; the TUI starts from the new thread store file
- Use `↑` `↓` in the list and `Enter` to open details
- Move focus to the detail pane and press `Enter` on the selected `Name` row to rename the currently selected memory
- Move to `+ Add Existing Memory Canister` at the end of the list and press `Enter` to register an existing memory manually
- In the modal, enter an existing memory canister id and submit it to validate access via `get_users()`
- For manually added memories, move focus to the detail pane and use `Tab` / `Shift+Tab` to jump between actions, including `Remove from list`
- `Remove from list` deletes only the local saved entry
- Press `Esc` while viewing details to return to the list

If you want to search within a single memory, it is easier to select that memory in the list first.

## Insert Tab

In the `Insert` tab, you choose how to add data to a memory. There are three modes.

![Insert tab screenshot](./images/tui-insert.png)

- `File`: load and insert a file
- `Inline Text`: insert text written directly in the UI
- `Manual Embedding`: insert by providing embedding JSON manually

### Shared Fields

- `Mode`: insertion method
- `Memory ID`: target memory for insertion
- `Tag`: tag to save with the content
- `Submit`: run the insertion

If `Memory ID` is left empty and a default memory is set, that value is shown as a placeholder candidate.

### File Mode

Supported extensions:

- `md`, `markdown`, `mdx`
- `txt`, `json`, `yaml`, `yml`, `csv`, `log`
- `pdf`

You can provide `FilePath` in two ways.

- Press `Enter` to open the `yazi` chooser and select a file
- Type the path directly

If `yazi` is not installed, the chooser is unavailable, but manual `FilePath` input still works. On macOS, install it with `brew install yazi` if needed.

If you select a `pdf`, it is converted to Markdown before insertion. On macOS, install `pdftotext` with `brew install poppler` if needed.

### Inline Text Mode

This stores text entered directly in the UI after generating an embedding. It is useful for short notes or test data.
`Inline Text` supports multiple lines.

### Manual Embedding Mode

Use this when you already have an embedding.

- `Text`: body text to save together with the embedding
- `Embedding`: vector in JSON array format

`Text` supports multiple lines. `Embedding` remains a single-line JSON array field.

In this mode, the expected dimension and current dimension are shown when available. If the dimensions do not match, you will get an error before submission.

### Reusing Tags

Tags can be reused from the picker. If saved tags exist, you can choose from the list, and tags added or removed in `Settings` are also reflected here.

## Create Tab

Create a new memory canister.

![Create tab screenshot](./images/tui-create.png)

Fields:

- `Name`: memory name
- `Description`: multi-line description
- `Submit`: create the memory

The creation screen also shows the following information.

- current principal
- KINIC balance
- required creation cost

If the balance is insufficient or retrieval fails, a message is shown on the spot.

## Settings Tab

In the `Settings` tab, you can view the current session information and saved settings.

The `Chat retrieval` section controls how `all memories` chat narrows cross-memory evidence before answering.

- `Chat result limit`: final number of documents passed into the answer prompt
- `Per-memory limit`: maximum number of documents kept from any one memory
- `Chat diversity`: balance between highest-score results and less-overlapping evidence

![Settings tab screenshot](./images/tui-settings.png)

Main items you can review:

- Principal ID
- KINIC balance
- Default memory
- Saved tags
- Embedding API endpoint
- Identity name
- Auth mode
- Network

### What You Can Configure

- Select `Default memory` and press `Enter`: change the default memory
- Select `KINIC balance` and press `Enter`: open the transfer modal
- Select `Saved tags` and press `Enter`: open saved tags and choose one for `Insert`
- `+ Add new tag` in the saved tag list: add a new tag
- `d` in the saved tag list: delete a tag
- `Ctrl+R`: refresh `Principal ID` and `KINIC balance`

The transfer modal accepts a recipient principal and an amount in KINIC. `Max` fills the largest sendable amount after subtracting the current ledger fee, and `Submit` opens a confirmation step before the transfer is sent.

Values saved from `Settings` persist across restarts.

You can check `Principal ID` and `KINIC balance` here. At the moment, there is no dedicated shortcut for copying the Principal ID.

## Saved Settings

The TUI saves its settings to `kinic/tui.yaml` under the config directory. On a typical macOS environment, the path is:

```text
~/.config/kinic/tui.yaml
```

The main saved values are:

- `default_memory_id`
- `saved_tags`

## Known Limitations

- `--identity` is required
- `--ii` is not supported yet
- The `Market` tab is not implemented yet
- `pdftotext` is required for PDF insertion
- There is no dedicated copy shortcut for Principal ID
- Local use requires a prepared local replica and supporting canisters

## Troubleshooting

- You see `--identity is required for the Kinic TUI` at startup  
  Start the TUI with `--identity <name>`.

- You cannot start it with `--ii`  
  The TUI does not support Internet Identity yet. For now, use a dfx identity.

- You are asked to approve Keychain access after startup  
  On macOS, access to Keychain is required to read the identity PEM. On first launch, or when access has not been granted yet, you may see a permission dialog. Approve it and try again.

- The memory list cannot be loaded  
  In a local environment, check that `dfx start` is running and the supporting canisters are deployed. Also confirm that you are connected to the expected target, and use `Ctrl+R` to refresh if needed.

- PDF insertion fails  
  Make sure `pdftotext` is available. If it is not installed, install Poppler. Also verify the file extension and read permissions.

- Saving settings fails  
  Check write permissions for `~/.config/kinic/`. It is also worth confirming that the existing `tui.yaml` is not corrupted.
