#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IDENTITY=""
QUERY="semantic memory retrieval"
SKIP_START=0
SKIP_SETUP=0
MINT_AMOUNT=100

usage() {
  cat <<'EOF'
Usage: scripts/seed_local_search_demo.sh [options]

Options:
  --identity <name>   DFX identity to use for create/insert/search
  --query <text>      Query to run at the end
  --mint <amount>     KINIC amount to mint to the selected identity before create (default: 100)
  --skip-start        Do not run `dfx start --clean --background`
  --skip-setup        Do not run `scripts/setup.sh` even if launcher is missing
  -h, --help          Show this help

This script:
1. Starts local dfx
2. Ensures launcher / ledger / II are deployed
3. Creates one memory canister
4. Inserts several markdown documents
5. Runs a search and prints the results
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --identity)
      IDENTITY="${2:-}"
      shift 2
      ;;
    --query)
      QUERY="${2:-}"
      shift 2
      ;;
    --mint)
      MINT_AMOUNT="${2:-}"
      shift 2
      ;;
    --skip-start)
      SKIP_START=1
      shift
      ;;
    --skip-setup)
      SKIP_SETUP=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

cd "$ROOT_DIR"

if ! command -v dfx >/dev/null 2>&1; then
  echo "dfx is required but was not found in PATH." >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required but was not found in PATH." >&2
  exit 1
fi

if [[ -z "$IDENTITY" ]]; then
  IDENTITY="$(dfx identity whoami)"
fi

if [[ "$IDENTITY" == "default" ]]; then
  echo "Refusing to use the \`default\` identity. Create or switch to a named dfx identity first." >&2
  exit 1
fi

echo "Using identity: $IDENTITY"
echo "Search query: $QUERY"

if [[ "$SKIP_START" -eq 0 ]]; then
  echo "Starting local dfx replica..."
  dfx start --clean --background
else
  echo "Skipping dfx start."
fi

if dfx canister id launcher >/dev/null 2>&1; then
  echo "Launcher canister already available."
elif [[ "$SKIP_SETUP" -eq 0 ]]; then
  echo "Running local setup script..."
  bash scripts/setup.sh
else
  echo "Launcher canister is missing and --skip-setup was specified." >&2
  exit 1
fi

PRINCIPAL="$(dfx --identity "$IDENTITY" identity get-principal)"
echo "Identity principal: $PRINCIPAL"

if [[ "$MINT_AMOUNT" -gt 0 ]]; then
  echo "Minting ${MINT_AMOUNT} KINIC to $PRINCIPAL..."
  bash scripts/mint.sh "$PRINCIPAL" "$MINT_AMOUNT"
fi

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/kinic-demo.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT

SEED_FILES=("$ROOT_DIR/README.md")
while IFS= read -r doc_file; do
  SEED_FILES+=("$ROOT_DIR/$doc_file")
done < <(find docs -maxdepth 2 -type f -name '*.md' | sort)

if [[ "${#SEED_FILES[@]}" -eq 0 ]]; then
  echo "No markdown files found to seed." >&2
  exit 1
fi

echo "Preparing seed documents:"
for source_file in "${SEED_FILES[@]}"; do
  if [[ ! -f "$source_file" ]]; then
    echo "Missing seed source: $source_file" >&2
    exit 1
  fi
  echo "  - ${source_file#$ROOT_DIR/}"
  cp "$source_file" "$WORK_DIR/$(basename "$source_file")"
done

CREATE_OUTPUT="$(
  cargo run --quiet -- \
    --identity "$IDENTITY" \
    create \
    --name "Local Search Demo" \
    --description "Seeded by scripts/seed_local_search_demo.sh"
)"

echo "$CREATE_OUTPUT"

MEMORY_ID="$(printf '%s\n' "$CREATE_OUTPUT" | awk -F': ' '/Memory canister id:/ {print $2}' | tail -n 1)"

if [[ -z "$MEMORY_ID" ]]; then
  echo "Failed to parse memory canister id from create output." >&2
  exit 1
fi

echo "Created memory canister: $MEMORY_ID"

for file in "$WORK_DIR"/*.md; do
  relative_name="$(basename "$file" .md)"
  tag="${relative_name//[^a-zA-Z0-9_-]/-}"
  echo "Inserting $tag from $(basename "$file")..."
  cargo run --quiet -- \
    --identity "$IDENTITY" \
    insert \
    --memory-id "$MEMORY_ID" \
    --tag "$tag" \
    --file-path "$file"
done

echo
echo "Running search..."
cargo run --quiet -- \
  --identity "$IDENTITY" \
  search \
  --memory-id "$MEMORY_ID" \
  --query "$QUERY"

echo
echo "Done."
echo "Memory canister id: $MEMORY_ID"
echo "Open the TUI with:"
echo "  cargo run -- --identity $IDENTITY tui"
