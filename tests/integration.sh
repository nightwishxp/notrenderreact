
#!/bin/bash

set -eu
set -o pipefail # If anything in a pipeline fails, the pipe's exit status is a failure
#set -x # Print all commands for debugging

declare -a KEY=(a b c d)

declare -A FROM=(
    [a]='-y --from a'
    [b]='-y --from b'
    [c]='-y --from c'
    [d]='-y --from d'
)

# This means we don't need to configure the cli since it uses the preconfigured cli in the docker.
# We define this as a function rather than as an alias because it has more flexible expansion behavior.
# In particular, it's not possible to dynamically expand aliases, but `tx_of` dynamically executes whatever
# we specify in its arguments.
function secretcli() {
    docker exec secretdev /usr/bin/secretcli "$@"
}

# Just like `echo`, but prints to stderr
function log() {
    echo "$@" >&2
}

# suppress all output to stdout and stderr for the command described in the arguments
function silent() {
    "$@" >/dev/null 2>&1
}

# Pad the string in the first argument to 256 bytes, using spaces
function pad_space() {
    printf '%-256s' "$1"
}

function assert_eq() {
    local left="$1"
    local right="$2"
    local message

    if [[ "$left" != "$right" ]]; then
        if [ -z ${3+x} ]; then
            local lineno="${BASH_LINENO[0]}"
            message="assertion failed on line $lineno - both sides differ. left: ${left@Q}, right: ${right@Q}"
        else
            message="$3"
        fi
        log "$message"
        return 1
    fi

    return 0
}

function assert_ne() {
    local left="$1"
    local right="$2"
    local message

    if [[ "$left" == "$right" ]]; then
        if [ -z ${3+x} ]; then
            local lineno="${BASH_LINENO[0]}"
            message="assertion failed on line $lineno - both sides are equal. left: ${left@Q}, right: ${right@Q}"
        else
            message="$3"
        fi

        log "$message"
        return 1
    fi

    return 0
}

declare -A ADDRESS=(
    [a]="$(secretcli keys show --address a)"
    [b]="$(secretcli keys show --address b)"
    [c]="$(secretcli keys show --address c)"
    [d]="$(secretcli keys show --address d)"
)

declare -A VK=([a]='' [b]='' [c]='' [d]='')

# Generate a label for a contract with a given code id
# This just adds "contract_" before the code id.
function label_by_id() {
    local id="$1"
    echo "contract_$id"
}

# Keep polling the blockchain until the tx completes.
# The first argument is the tx hash.
# The second argument is a message that will be logged after every failed attempt.
# The tx information will be returned.
function wait_for_tx() {
    local tx_hash="$1"
    local message="$2"

    local result

    log "waiting on tx: $tx_hash"
    # secretcli will only print to stdout when it succeeds
    until result="$(secretcli query tx "$tx_hash" 2>/dev/null)"; do
        log "$message"
        sleep 1
    done

    # log out-of-gas events
    if jq -e '.raw_log | startswith("execute contract failed: Out of gas: ") or startswith("out of gas:")' <<<"$result" >/dev/null; then
        log "$(jq -r '.raw_log' <<<"$result")"
    fi

    echo "$result"
}

# This is a wrapper around `wait_for_tx` that also decrypts the response,
# and returns a nonzero status code if the tx failed
function wait_for_compute_tx() {
    local tx_hash="$1"
    local message="$2"
    local return_value=0
    local result
    local decrypted

    result="$(wait_for_tx "$tx_hash" "$message")"
    # log "$result"
    if jq -e '.logs == null' <<<"$result" >/dev/null; then
        return_value=1
    fi
    decrypted="$(secretcli query compute tx "$tx_hash")" || return
    log "$decrypted"
    echo "$decrypted"

    return "$return_value"
}

# If the tx failed, return a nonzero status code.
# The decrypted error or message will be echoed
function check_tx() {
    local tx_hash="$1"
    local result
    local return_value=0

    result="$(secretcli query tx "$tx_hash")"
    if jq -e '.logs == null' <<<"$result" >/dev/null; then
        return_value=1
    fi
    decrypted="$(secretcli query compute tx "$tx_hash")" || return
    log "$decrypted"
    echo "$decrypted"

    return "$return_value"
}

# Extract the tx_hash from the output of the command
function tx_of() {
    "$@" | jq -r '.txhash'
}

# Extract the output_data_as_string from the output of the command
function data_of() {
    "$@" | jq -r '.output_data_as_string'
}

function get_generic_err() {
    jq -r '.output_error.generic_err.msg' <<<"$1"
}

# Send a compute transaction and return the tx hash.
# All arguments to this function are passed directly to `secretcli tx compute execute`.
function compute_execute() {
    tx_of secretcli tx compute execute "$@"
}

# Send a query to the contract.
# All arguments to this function are passed directly to `secretcli query compute query`.
function compute_query() {
    secretcli query compute query "$@"
}

function upload_code() {
    local directory="$1"
    local tx_hash
    local code_id

    tx_hash="$(tx_of secretcli tx compute store "code/$directory/contract.wasm.gz" ${FROM[a]} --gas 10000000)"
    code_id="$(
        wait_for_tx "$tx_hash" 'waiting for contract upload' |
            jq -r '.logs[0].events[0].attributes[] | select(.key == "code_id") | .value'
    )"

    log "uploaded contract #$code_id"

    echo "$code_id"
}

function instantiate() {
    local code_id="$1"
    local init_msg="$2"

    log 'sending init message:'
    log "${init_msg@Q}"

    local tx_hash
    tx_hash="$(tx_of secretcli tx compute instantiate "$code_id" "$init_msg" --label "$(label_by_id "$code_id")" ${FROM[a]} --gas 10000000)"
    wait_for_tx "$tx_hash" 'waiting for init to complete'
}

# This function uploads and instantiates a contract, and returns the new contract's address
function create_contract() {
    local dir="$1"
    local init_msg="$2"

    local code_id
    code_id="$(upload_code "$dir")"

    local init_result
    init_result="$(instantiate "$code_id" "$init_msg")"

    if jq -e '.logs == null' <<<"$init_result" >/dev/null; then
        log "$(secretcli query compute tx "$(jq -r '.txhash' <<<"$init_result")")"
        return 1
    fi

    jq -r '.logs[0].events[0].attributes[] | select(.key == "contract_address") | .value' <<<"$init_result"
}

function deposit() {
    local contract_addr="$1"
    local key="$2"
    local amount="$3"

    local deposit_message='{"deposit":{"padding":":::::::::::::::::"}}'
    local tx_hash
    local deposit_response
    tx_hash="$(compute_execute "$contract_addr" "$deposit_message" --amount "${amount}uscrt" ${FROM[$key]} --gas 150000)"
    deposit_response="$(data_of wait_for_compute_tx "$tx_hash" "waiting for deposit to \"$key\" to process")"
    assert_eq "$deposit_response" "$(pad_space '{"deposit":{"status":"success"}}')"
    log "deposited ${amount}uscrt to \"$key\" successfully"
}

function get_balance() {
    local contract_addr="$1"
    local key="$2"

    log "querying balance for \"$key\""
    local balance_query='{"balance":{"address":"'"${ADDRESS[$key]}"'","key":"'"${VK[$key]}"'"}}'
    local balance_response
    balance_response="$(compute_query "$contract_addr" "$balance_query")"
    log "balance response was: $balance_response"
    jq -r '.balance.amount' <<<"$balance_response"
}

# Redeem some SCRT from an account
# As you can see, verifying this is happening correctly requires a lot of code
# so I separated it to its own function, because it's used several times.
function redeem() {
    local contract_addr="$1"
    local key="$2"
    local amount="$3"

    local redeem_message
    local tx_hash
    local redeem_tx
    local transfer_attributes
    local redeem_response

    log "redeeming \"$key\""
    redeem_message='{"redeem":{"amount":"'"$amount"'"}}'
    tx_hash="$(compute_execute "$contract_addr" "$redeem_message" ${FROM[$key]} --gas 150000)"
    redeem_tx="$(wait_for_tx "$tx_hash" "waiting for redeem from \"$key\" to process")"
    transfer_attributes="$(jq -r '.logs[0].events[] | select(.type == "transfer") | .attributes' <<<"$redeem_tx")"
    assert_eq "$(jq -r '.[] | select(.key == "recipient") | .value' <<<"$transfer_attributes")" "${ADDRESS[$key]}"
    assert_eq "$(jq -r '.[] | select(.key == "amount") | .value' <<<"$transfer_attributes")" "${amount}uscrt"
    log "redeem response for \"$key\" returned ${amount}uscrt"

    redeem_response="$(data_of check_tx "$tx_hash")"
    assert_eq "$redeem_response" "$(pad_space '{"redeem":{"status":"success"}}')"
    log "redeemed ${amount} from \"$key\" successfully"
}

function get_token_info() {
    local contract_addr="$1"

    local token_info_query='{"token_info":{}}'
    compute_query "$contract_addr" "$token_info_query"
}

function increase_allowance() {
    local contract_addr="$1"
    local owner_key="$2"
    local spender_key="$3"
    local amount="$4"

    local owner_address="${ADDRESS[$owner_key]}"
    local spender_address="${ADDRESS[$spender_key]}"
    local allowance_message='{"increase_allowance":{"spender":"'"$spender_address"'","amount":"'"$amount"'"}}'
    local allowance_response

    tx_hash="$(compute_execute "$contract_addr" "$allowance_message" ${FROM[$owner_key]} --gas 150000)"
    allowance_response="$(data_of wait_for_compute_tx "$tx_hash" "waiting for the increase of \"$spender_key\"'s allowance to \"$owner_key\"'s funds to process")"
    assert_eq "$(jq -r '.increase_allowance.spender' <<<"$allowance_response")" "$spender_address"
    assert_eq "$(jq -r '.increase_allowance.owner' <<<"$allowance_response")" "$owner_address"
    jq -r '.increase_allowance.allowance' <<<"$allowance_response"
    log "Increased allowance given to \"$spender_key\" from \"$owner_key\" by ${amount}uscrt successfully"
}

function decrease_allowance() {
    local contract_addr="$1"
    local owner_key="$2"
    local spender_key="$3"
    local amount="$4"

    local owner_address="${ADDRESS[$owner_key]}"
    local spender_address="${ADDRESS[$spender_key]}"
    local allowance_message='{"decrease_allowance":{"spender":"'"$spender_address"'","amount":"'"$amount"'"}}'
    local allowance_response

    tx_hash="$(compute_execute "$contract_addr" "$allowance_message" ${FROM[$owner_key]} --gas 150000)"
    allowance_response="$(data_of wait_for_compute_tx "$tx_hash" "waiting for the decrease of \"$spender_key\"'s allowance to \"$owner_key\"'s funds to process")"
    assert_eq "$(jq -r '.decrease_allowance.spender' <<<"$allowance_response")" "$spender_address"
    assert_eq "$(jq -r '.decrease_allowance.owner' <<<"$allowance_response")" "$owner_address"
    jq -r '.decrease_allowance.allowance' <<<"$allowance_response"
    log "Decreased allowance given to \"$spender_key\" from \"$owner_key\" by ${amount}uscrt successfully"
}

function get_allowance() {
    local contract_addr="$1"
    local owner_key="$2"
    local spender_key="$3"

    log "querying allowance given to \"$spender_key\" by \"$owner_key\""
    local owner_address="${ADDRESS[$owner_key]}"
    local spender_address="${ADDRESS[$spender_key]}"
    local allowance_query='{"allowance":{"spender":"'"$spender_address"'","owner":"'"$owner_address"'","key":"'"${VK[$owner_key]}"'"}}'
    local allowance_response
    allowance_response="$(compute_query "$contract_addr" "$allowance_query")"
    log "allowance response was: $allowance_response"
    assert_eq "$(jq -r '.allowance.spender' <<<"$allowance_response")" "$spender_address"
    assert_eq "$(jq -r '.allowance.owner' <<<"$allowance_response")" "$owner_address"
    jq -r '.allowance.allowance' <<<"$allowance_response"
}

function log_test_header() {
    log " # Starting ${FUNCNAME[1]}"
}

function test_viewing_key() {
    local contract_addr="$1"

    log_test_header

    # common variables
    local result
    local tx_hash

    # query balance. Should fail.
    local wrong_key
    wrong_key="$(xxd -ps <<<'wrong-key')"
    local balance_query
    local expected_error='{"viewing_key_error":{"msg":"Wrong viewing key for this address or viewing key not set"}}'
    for key in "${KEY[@]}"; do
        log "querying balance for \"$key\" with wrong viewing key"
        balance_query='{"balance":{"address":"'"${ADDRESS[$key]}"'","key":"'"$wrong_key"'"}}'
        result="$(compute_query "$contract_addr" "$balance_query")"
        assert_eq "$result" "$expected_error"
    done

    # Create viewing keys
    local create_viewing_key_message='{"create_viewing_key":{"entropy":"MyPassword123"}}'
    local viewing_key_response
    for key in "${KEY[@]}"; do
        log "creating viewing key for \"$key\""
        tx_hash="$(compute_execute "$contract_addr" "$create_viewing_key_message" ${FROM[$key]} --gas 1400000)"
        viewing_key_response="$(data_of wait_for_compute_tx "$tx_hash" "waiting for viewing key for \"$key\" to be created")"
        VK[$key]="$(jq -er '.create_viewing_key.key' <<<"$viewing_key_response")"
        log "viewing key for \"$key\" set to ${VK[$key]}"
        if [[ "${VK[$key]}" =~ ^api_key_ ]]; then
            log "viewing key \"$key\" seems valid"
        else
            log 'viewing key is invalid'
            return 1
        fi
    done

    # Check that all viewing keys are different despite using the same entropy
    assert_ne "${VK[a]}" "${VK[b]}"