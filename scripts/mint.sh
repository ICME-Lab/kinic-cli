#!/bin/bash
set -e  # Stop script on errors

MINTER_NAME="${3:-local-minter}"

echo Mint $2 KINIC to $1 using identity ${MINTER_NAME}
KINIC=$(( $2 * 100000000 ))

dfx --identity "${MINTER_NAME}" canister call 73mez-iiaaa-aaaaq-aaasq-cai icrc1_transfer '
(
  record {
    25_979 = record {
      947_296_307 = principal "'$1'";
      1_349_681_965 = null;
    };
    5_094_982 = opt (0 : nat);
    1_213_809_850 = null;
    1_835_347_746 = null;
    3_258_775_938 = null;
    3_573_748_184 = '$KINIC' : nat;
  },
)'
