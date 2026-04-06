#!/bin/bash
set -e  # Stop script on errors
# set -x  # Print commands for debugging
set -o pipefail  # Fail on pipeline errors

## Local replica bootstrap for launcher, ledger, and Internet Identity.
## This keeps local `--identity` and `--ii` flows aligned with the documented setup.

## -- Ledger --
USER_NAME=$(dfx identity whoami)
USER_PRINCIPAL=$(dfx identity get-principal)
MINTER_NAME="${KINIC_LOCAL_MINTER_IDENTITY:-local-minter}"
echo "Current user: ${USER_NAME}"

if ! dfx identity use "${MINTER_NAME}" >/dev/null 2>&1; then
  dfx identity new "${MINTER_NAME}" --storage-mode plaintext --disable-encryption >/dev/null
  dfx identity use "${MINTER_NAME}" >/dev/null
fi

# Set environment variables
export MINTER_PRINCIPAL=$(dfx identity get-principal)
export TOKEN_NAME="My Token"
export TOKEN_SYMBOL="XMTK"
export PRE_MINTED_TOKENS=10_000_000_000
export TRANSFER_FEE=100_000
export ARCHIVE_CONTROLLER="${MINTER_PRINCIPAL}"
export TRIGGER_THRESHOLD=2000
export NUM_OF_BLOCK_TO_ARCHIVE=1000
export CYCLE_FOR_ARCHIVE_CREATION=10000000000000
export FEATURE_FLAGS=true

# Deploy ledger canister
dfx deploy icrc1_ledger_canister --specified-id 73mez-iiaaa-aaaaq-aaasq-cai --argument "(variant {Init =
record {
     token_symbol = \"${TOKEN_SYMBOL}\";
     token_name = \"${TOKEN_NAME}\";
     minting_account = record { owner = principal \"${MINTER_PRINCIPAL}\" };
     transfer_fee = ${TRANSFER_FEE};
     metadata = vec {};
     feature_flags = opt record{icrc2 = ${FEATURE_FLAGS}};
     initial_balances = vec { record { record { owner = principal \"${USER_PRINCIPAL}\"; }; ${PRE_MINTED_TOKENS}; }; };
     archive_options = record {
         num_blocks_to_archive = ${NUM_OF_BLOCK_TO_ARCHIVE};
         trigger_threshold = ${TRIGGER_THRESHOLD};
         controller_id = principal \"${ARCHIVE_CONTROLLER}\";
         cycles_for_archive_creation = opt ${CYCLE_FOR_ARCHIVE_CREATION};
     };
 }
})"

# Switch back to original identity
dfx identity use "${USER_NAME}"

dfx deploy internet_identity --specified-id rdmx6-jaaaa-aaaaa-aaadq-cai
dfx deploy launcher --specified-id xfug4-5qaaa-aaaak-afowa-cai --argument='(variant {minor})'
# dfx canister call launcher change_key_id '("test_key_1")'
dfx ledger fabricate-cycles --cycles 100T --canister $(dfx canister id launcher)

dfx identity use $USER_NAME

bash scripts/mint.sh "${USER_PRINCIPAL}" 100 "${MINTER_NAME}"
