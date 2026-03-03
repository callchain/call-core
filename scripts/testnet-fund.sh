#!/bin/bash
#
# testnet-fund.sh - Fund a testnet account from the genesis account
#
# Usage: ./testnet-fund.sh [OPTIONS] <ADDRESS> <AMOUNT>
#
# Arguments:
#   ADDRESS     Account address to fund (c...)
#   AMOUNT      Amount in drops (e.g., 10000000 = 10 CALL)
#
# Options:
#   -r, --rpc URL         RPC endpoint (default: http://127.0.0.1:5005)
#   -s, --seed SEED       Genesis account seed (or use TESTNET_GENESIS_SEED env)
#   -h, --help            Show this help message
#
# Example:
#   ./testnet-fund.sh cTestAccount123... 1000000000

set -e

# Configuration
RPC_URL="http://127.0.0.1:5005"
GENESIS_SEED="${TESTNET_GENESIS_SEED:-snGenesisMasterSeedForTestingOnly}"
GENESIS_ADDRESS="cGenesisAccount1111111111111111"
CALLED_CMD="${CALLED_CMD:-calld}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

usage() {
    grep '^#' "$0" | tail -n +2 | sed 's/^# //'
    exit 1
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -r|--rpc)
            RPC_URL="$2"
            shift 2
            ;;
        -s|--seed)
            GENESIS_SEED="$2"
            shift 2
            ;;
        -h|--help)
            usage
            ;;
        -*)
            echo "Unknown option: $1"
            usage
            ;;
        *)
            break
            ;;
    esac
done

# Get positional arguments
if [[ $# -lt 2 ]]; then
    echo -e "${RED}Error: Missing required arguments${NC}"
    usage
fi

TARGET_ADDRESS="$1"
AMOUNT="$2"

# Validate address
if [[ ! "$TARGET_ADDRESS" =~ ^c[a-zA-Z0-9]{20,}$ ]]; then
    echo -e "${RED}Error: Invalid address format: $TARGET_ADDRESS${NC}"
    echo "Address should start with 'c' followed by alphanumeric characters"
    exit 1
fi

# Validate amount
if [[ ! "$AMOUNT" =~ ^[0-9]+$ ]]; then
    echo -e "${RED}Error: Amount must be a positive integer (drops)${NC}"
    exit 1
fi

echo "=========================================="
echo "  Fund Testnet Account"
echo "=========================================="
echo ""
echo "Target: $TARGET_ADDRESS"
echo "Amount: $AMOUNT drops ($(echo "scale=6; $AMOUNT / 1000000" | bc) CALL)"
echo "RPC:    $RPC_URL"
echo ""

# Check if RPC is available
echo -n "Checking RPC connection..."
if ! curl -s "$RPC_URL" -X POST \
    -H "Content-Type: application/json" \
    -d '{"method":"server_info"}' > /dev/null 2>&1; then
    echo -e " ${RED}FAILED${NC}"
    echo "Error: Cannot connect to RPC at $RPC_URL"
    echo "Make sure the testnet is running"
    exit 1
fi
echo -e " ${GREEN}OK${NC}"

# Get genesis account sequence
echo -n "Getting genesis account sequence..."
ACCOUNT_INFO=$(curl -s "$RPC_URL" -X POST \
    -H "Content-Type: application/json" \
    -d "{\"method\":\"account_info\",\"params\":[{\"account\":\"$GENESIS_ADDRESS\"}]}")

SEQUENCE=$(echo "$ACCOUNT_INFO" | grep -o '"Sequence":[0-9]*' | cut -d':' -f2 || echo "")
if [[ -z "$SEQUENCE" ]]; then
    echo -e " ${RED}FAILED${NC}"
    echo "Error: Could not get genesis account sequence"
    echo "Response: $ACCOUNT_INFO"
    exit 1
fi
echo -e " ${GREEN}OK${NC} (Sequence: $SEQUENCE)"

# Create and sign transaction
echo -n "Creating transaction..."

TX_JSON=$(cat <<EOF
{
  "TransactionType": "Payment",
  "Account": "$GENESIS_ADDRESS",
  "Destination": "$TARGET_ADDRESS",
  "Amount": "$AMOUNT",
  "Fee": "10",
  "Sequence": $SEQUENCE
}
EOF
)

SIGNED_TX=$($CALLED_CMD sign "$GENESIS_SEED" "$TX_JSON" 2>/dev/null)
if [[ $? -ne 0 ]]; then
    echo -e " ${RED}FAILED${NC}"
    echo "Error: Failed to sign transaction"
    exit 1
fi

TX_BLOB=$(echo "$SIGNED_TX" | grep -o '"tx_blob":"[^"]*"' | cut -d'"' -f4)
if [[ -z "$TX_BLOB" ]]; then
    echo -e " ${RED}FAILED${NC}"
    echo "Error: Could not extract tx_blob from signed transaction"
    echo "Signed TX: $SIGNED_TX"
    exit 1
fi
echo -e " ${GREEN}OK${NC}"

# Submit transaction
echo -n "Submitting transaction..."
SUBMIT_RESULT=$(curl -s "$RPC_URL" -X POST \
    -H "Content-Type: application/json" \
    -d "{\"method\":\"submit\",\"params\":[{\"tx_blob\":\"$TX_BLOB\"}]}")

ENGINE_RESULT=$(echo "$SUBMIT_RESULT" | grep -o '"engine_result":"[^"]*"' | cut -d'"' -f4 || echo "")
if [[ "$ENGINE_RESULT" != "tesSUCCESS" ]]; then
    echo -e " ${RED}FAILED${NC}"
    echo "Error: Transaction failed"
    echo "Engine result: $ENGINE_RESULT"
    echo "Full response: $SUBMIT_RESULT"
    exit 1
fi
echo -e " ${GREEN}OK${NC}"

echo ""
echo -e "${GREEN}Successfully funded account!${NC}"
echo ""
echo "Transaction details:"
echo "  Engine Result: $ENGINE_RESULT"
echo "  Hash: $(echo "$SUBMIT_RESULT" | grep -o '"hash":"[^"]*"' | cut -d'"' -f4 || echo "N/A")"
echo ""
echo "Check balance:"
echo "  curl $RPC_URL -X POST -H 'Content-Type: application/json' \\"
echo "    -d '{\"method\":\"account_info\",\"params\":[{\"account\":\"$TARGET_ADDRESS\"}]}'"
