#!/usr/bin/env bash
# generate-fixtures.sh
# Generates 30GB of synthetic OPNsense firewall logs for testing
# - 10GB RFC3164 format
# - 10GB RFC5424 format
# - 10GB CSV filterlog format

set -e

SIZE_MB=${1:-10240}  # Default: 10GB per format (30GB total)

echo "=== OPNsense Log Fixture Generator ==="
echo "Generating 3 x ${SIZE_MB}MB = $((SIZE_MB * 3 / 1024))GB total"
echo ""

FIXTURES_DIR="$(cd "$(dirname "$0")/../tests/fixtures" && pwd)"
mkdir -p "$FIXTURES_DIR"

# Realistic data distributions
ACTIONS=("pass" "block" "reject")
ACTION_WEIGHTS=(60 35 5)

PROTOCOLS=("tcp" "udp" "icmp")
PROTOCOL_NUMS=(6 17 1)
PROTOCOL_WEIGHTS=(70 25 5)

INTERFACES=("vtnet0" "vtnet1" "em0" "ix0")

COMMON_PORTS=(443 80 22 53 3389 8080 8443 25 110 143)

# Helper function to get weighted random action
get_weighted_action() {
    local rand=$((RANDOM % 100))
    if [ $rand -lt 60 ]; then
        echo "pass"
    elif [ $rand -lt 95 ]; then
        echo "block"
    else
        echo "reject"
    fi
}

# Helper function to get weighted random protocol
get_weighted_protocol() {
    local rand=$((RANDOM % 100))
    if [ $rand -lt 70 ]; then
        echo "6 tcp"
    elif [ $rand -lt 95 ]; then
        echo "17 udp"
    else
        echo "1 icmp"
    fi
}

# Helper function to generate random IP
get_random_ip() {
    local private=${1:-0}
    local rand=$((RANDOM % 100))

    if [ $rand -lt 50 ] || [ $private -eq 1 ]; then
        # RFC1918 private ranges
        local range=$((RANDOM % 3))
        case $range in
            0) echo "192.168.$((RANDOM % 256)).$((RANDOM % 254 + 1))" ;;
            1) echo "10.$((RANDOM % 256)).$((RANDOM % 256)).$((RANDOM % 254 + 1))" ;;
            2) echo "172.$((RANDOM % 16 + 16)).$((RANDOM % 256)).$((RANDOM % 254 + 1))" ;;
        esac
    else
        # Public IP (avoiding reserved ranges)
        echo "$((RANDOM % 223 + 1)).$((RANDOM % 256)).$((RANDOM % 256)).$((RANDOM % 254 + 1))"
    fi
}

# Helper function to get random port
get_random_port() {
    local rand=$((RANDOM % 100))
    if [ $rand -lt 55 ]; then
        # 55% common ports
        echo "${COMMON_PORTS[$((RANDOM % ${#COMMON_PORTS[@]}))]}"
    else
        # 45% random high ports
        echo "$((RANDOM % 64511 + 1024))"
    fi
}

BYTES_PER_LINE=200
LINE_COUNT=$((SIZE_MB * 1024 * 1024 / BYTES_PER_LINE))

# 1. Generate RFC3164 format (10GB)
echo "[1/3] Generating RFC3164 format (${SIZE_MB}MB)..."
RFC3164_PATH="$FIXTURES_DIR/rfc3164_${SIZE_MB}mb.log"

{
    for ((i=0; i<LINE_COUNT; i++)); do
        if [ $((i % 100000)) -eq 0 ]; then
            echo -ne "\r  Progress: $((i * 100 / LINE_COUNT))%" >&2
        fi

        ACTION=$(get_weighted_action)
        read -r PROTO_NUM PROTO <<< "$(get_weighted_protocol)"
        INTERFACE=${INTERFACES[$((RANDOM % ${#INTERFACES[@]}))]}
        SOURCE_IP=$(get_random_ip)
        DEST_IP=$(get_random_ip)
        SOURCE_PORT=$(get_random_port)
        DEST_PORT=$(get_random_port)

        PRIORITY=134
        TIMESTAMP=$(date '+%b %d %H:%M:%S')
        TRACKER=$((1000000000 + i))
        PACKET_ID=$((RANDOM % 64535 + 1000))
        SEQ=$((RANDOM % 9000000000 + 1000000000))

        # RFC3164 format
        echo "<$PRIORITY>$TIMESTAMP fw1 filterlog: 5,,,$TRACKER,$INTERFACE,match,$ACTION,in,4,0x0,,64,$PACKET_ID,0,none,$PROTO_NUM,$PROTO,60,$SOURCE_IP,$DEST_IP,$SOURCE_PORT,$DEST_PORT,0,S,$SEQ,,64240,,mss;sackOK;TS"
    done
} > "$RFC3164_PATH"

echo -e "\r  ✓ RFC3164 generated: $RFC3164_PATH"

# 2. Generate RFC5424 format (10GB)
echo "[2/3] Generating RFC5424 format (${SIZE_MB}MB)..."
RFC5424_PATH="$FIXTURES_DIR/rfc5424_${SIZE_MB}mb.log"

{
    for ((i=0; i<LINE_COUNT; i++)); do
        if [ $((i % 100000)) -eq 0 ]; then
            echo -ne "\r  Progress: $((i * 100 / LINE_COUNT))%" >&2
        fi

        ACTION=$(get_weighted_action)
        read -r PROTO_NUM PROTO <<< "$(get_weighted_protocol)"
        INTERFACE=${INTERFACES[$((RANDOM % ${#INTERFACES[@]}))]}
        SOURCE_IP=$(get_random_ip)
        DEST_IP=$(get_random_ip)
        SOURCE_PORT=$(get_random_port)
        DEST_PORT=$(get_random_port)

        PRIORITY=134
        TIMESTAMP_ISO=$(date -u '+%Y-%m-%dT%H:%M:%SZ')
        TRACKER=$((1000000000 + i))
        PACKET_ID=$((RANDOM % 64535 + 1000))
        SEQ=$((RANDOM % 9000000000 + 1000000000))

        # RFC5424 format
        echo "<$PRIORITY>1 $TIMESTAMP_ISO fw1 filterlog - - [meta sequenceId=\"$TRACKER\"] 5,,,$TRACKER,$INTERFACE,match,$ACTION,in,4,0x0,,64,$PACKET_ID,0,none,$PROTO_NUM,$PROTO,60,$SOURCE_IP,$DEST_IP,$SOURCE_PORT,$DEST_PORT,0,S,$SEQ,,64240,,mss;sackOK;TS"
    done
} > "$RFC5424_PATH"

echo -e "\r  ✓ RFC5424 generated: $RFC5424_PATH"

# 3. Generate CSV filterlog format (10GB)
echo "[3/3] Generating CSV filterlog format (${SIZE_MB}MB)..."
CSV_PATH="$FIXTURES_DIR/csv_filterlog_${SIZE_MB}mb.log"

{
    for ((i=0; i<LINE_COUNT; i++)); do
        if [ $((i % 100000)) -eq 0 ]; then
            echo -ne "\r  Progress: $((i * 100 / LINE_COUNT))%" >&2
        fi

        ACTION=$(get_weighted_action)
        read -r PROTO_NUM PROTO <<< "$(get_weighted_protocol)"
        INTERFACE=${INTERFACES[$((RANDOM % ${#INTERFACES[@]}))]}
        SOURCE_IP=$(get_random_ip)
        DEST_IP=$(get_random_ip)
        SOURCE_PORT=$(get_random_port)
        DEST_PORT=$(get_random_port)

        TRACKER=$((1000000000 + i))
        PACKET_ID=$((RANDOM % 64535 + 1000))
        SEQ=$((RANDOM % 9000000000 + 1000000000))

        # CSV filterlog format
        echo "5,,,$TRACKER,$INTERFACE,match,$ACTION,in,4,0x0,,64,$PACKET_ID,0,none,$PROTO_NUM,$PROTO,60,$SOURCE_IP,$DEST_IP,$SOURCE_PORT,$DEST_PORT,0,S,$SEQ,,64240,,mss;sackOK;TS"
    done
} > "$CSV_PATH"

echo -e "\r  ✓ CSV filterlog generated: $CSV_PATH"

echo ""
echo "=== Generation Complete ==="
echo "Total size: $((SIZE_MB * 3 / 1024))GB"
echo "Files created in: $FIXTURES_DIR"
echo ""
echo "⚠️  Remember: Add tests/fixtures/*.log to .gitignore"
