#!/bin/bash

# Vibe Coded this script, because I was lazy... ^^
# https://de.wikipedia.org/wiki/Vibe_Coding

# Exit on error
set -e

# Colors and styles for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# ASCII Art
print_banner() {
    echo -e "${CYAN}"
    echo " _____       ____ _   _ _____ ____ ____ "
    echo "| ____|     / ___| | | | ____/ ___/ ___|"
    echo "|  _| _____| |   | |_| |  _| \\___ \\___ \\"
    echo "| |__|_____| |___|  _  | |___ ___) |__) |"
    echo "|_____|     \\____|_| |_|_____|____/____/"
    echo -e "${NC}"
    echo -e "${YELLOW}ESP32 Firmware Builder and OTA Deployer${NC}"
    echo -e "${BLUE}=========================================${NC}\n"
}

print_step() {
    echo -e "\n${BOLD}${BLUE}▶${NC} ${BOLD}$1${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_info() {
    echo -e "${CYAN}ℹ $1${NC}"
}

print_progress() {
    echo -e "${BLUE}⟳ $1${NC}"
}

# Supported chips
SUPPORTED_CHIPS=("esp32" "esp32s3")

# Function to get partition table for flash size
get_partition_table() {
    local flash_size="$1"
    case "$flash_size" in
        "8mb")
            echo "partitions_8.csv"
            ;;
        "16mb")
            echo "partitions_16.csv"
            ;;
        *)
            print_error "Unsupported flash size: $flash_size. Supported sizes: 8MB, 16MB"
            exit 1
            ;;
    esac
}

# Function to show usage for run subcommand
show_run_help() {
    echo -e "\n${BOLD}Usage:${NC} $0 run <chip> [options]"
    echo -e "\n${BOLD}Description:${NC}"
    echo -e "  Build, flash and watch via USB"
    echo -e "\n${BOLD}Arguments:${NC}"
    echo -e "  ${CYAN}<chip>${NC}           Chip to build for (${SUPPORTED_CHIPS[*]})"
    echo -e "\n${BOLD}Options:${NC}"
    echo -e "  ${CYAN}-h, --help${NC}     Show this help message"
    echo -e "  ${CYAN}-f, --features${NC} Comma-separated list of features to enable (optional)"
    echo -e "  ${CYAN}--release${NC}       Build in release mode (default is debug mode)"
    echo -e "  ${CYAN}--flash-size${NC}    Flash size to use (8MB or 16MB, defaults to 16MB)"
    echo -e "\n${BOLD}Examples:${NC}"
    echo -e "  ${GREEN}$0 run ${SUPPORTED_CHIPS[0]}${NC}          # Build, flash and watch for ${SUPPORTED_CHIPS[0]} (debug build)"
    echo -e "  ${GREEN}$0 run ${SUPPORTED_CHIPS[0]} --release${NC} # Build, flash and watch for ${SUPPORTED_CHIPS[0]} (release build)"
    echo -e "  ${GREEN}$0 run ${SUPPORTED_CHIPS[0]} --flash-size 8MB${NC} # Use 8MB flash configuration"
}

# Function to show usage for ota subcommand
show_ota_help() {
    echo -e "\n${BOLD}Usage:${NC} $0 ota <ip> <chip> [options]"
    echo -e "\n${BOLD}Description:${NC}"
    echo -e "  Build and deploy firmware via OTA to the specified IP address"
    echo -e "\n${BOLD}Arguments:${NC}"
    echo -e "  ${CYAN}<ip>${NC}             IP address of the device for OTA deployment"
    echo -e "  ${CYAN}<chip>${NC}           Chip to build for (${SUPPORTED_CHIPS[*]})"
    echo -e "\n${BOLD}Options:${NC}"
    echo -e "  ${CYAN}-h, --help${NC}     Show this help message"
    echo -e "  ${CYAN}-f, --features${NC} Comma-separated list of features to enable (optional)"
    echo -e "  ${CYAN}--flash-size${NC}    Flash size to use (8MB or 16MB, defaults to 16MB)"
    echo -e "  ${CYAN}--watch${NC}         Watch USB debug logs after update"
    echo -e "  ${CYAN}--release${NC}       Build in release mode (default is debug mode)"
    echo -e "\n${BOLD}Examples:${NC}"
    echo -e "  ${GREEN}$0 ota 192.168.4.1 ${SUPPORTED_CHIPS[0]}${NC} # Build and deploy via OTA"
    echo -e "  ${GREEN}$0 ota 192.168.4.1 ${SUPPORTED_CHIPS[0]} --watch${NC} # Build, deploy via OTA, and watch logs"
    echo -e "  ${GREEN}$0 ota 192.168.4.1 ${SUPPORTED_CHIPS[0]} --flash-size 8MB${NC} # Use 8MB flash configuration"
}

# Function to show usage for build subcommand
show_build_help() {
    echo -e "\n${BOLD}Usage:${NC} $0 build <chip> [options]"
    echo -e "\n${BOLD}Description:${NC}"
    echo -e "  Build the firmware without running or deploying it"
    echo -e "\n${BOLD}Arguments:${NC}"
    echo -e "  ${CYAN}<chip>${NC}           Chip to build for (${SUPPORTED_CHIPS[*]})"
    echo -e "\n${BOLD}Options:${NC}"
    echo -e "  ${CYAN}-h, --help${NC}     Show this help message"
    echo -e "  ${CYAN}-f, --features${NC} Comma-separated list of features to enable (optional)"
    echo -e "  ${CYAN}--partition-table${NC} Path to partition table CSV (optional)"
    echo -e "  ${CYAN}--watch${NC}         Watch USB debug logs after build"
    echo -e "  ${CYAN}--release${NC}       Build in release mode (default is debug mode)"
    echo -e "\n${BOLD}Examples:${NC}"
    echo -e "  ${GREEN}$0 build ${SUPPORTED_CHIPS[0]}${NC}          # Build for ${SUPPORTED_CHIPS[0]} (debug build)"
    echo -e "  ${GREEN}$0 build ${SUPPORTED_CHIPS[0]} --release${NC} # Build for ${SUPPORTED_CHIPS[0]} (release build)"
    echo -e "  ${GREEN}$0 build ${SUPPORTED_CHIPS[0]} --watch${NC}   # Build and watch logs"
}

# Function to show usage for watch subcommand
show_watch_help() {
    echo -e "\n${BOLD}Usage:${NC} $0 watch"
    echo -e "\n${BOLD}Description:${NC}"
    echo -e "  Watch debug logs without building or deploying"
    echo -e "\n${BOLD}Examples:${NC}"
    echo -e "  ${GREEN}$0 watch${NC}         # Watch debug logs"
}

# Function to show main usage
show_usage() {
    echo -e "\n${BOLD}Usage:${NC} $0 <subcommand> [args] [options]"
    echo -e "\n${BOLD}Subcommands:${NC}"
    echo -e "  ${CYAN}run <chip>${NC}           Build, flash and watch via USB"
    echo -e "  ${CYAN}ota <ip> <chip>${NC}      Build and deploy via OTA"
    echo -e "  ${CYAN}build <chip>${NC}         Build the firmware"
    echo -e "  ${CYAN}watch${NC}                Watch the debug logs"
    echo -e "\n${BOLD}Options:${NC}"
    echo -e "  ${CYAN}-h, --help${NC}     Show this help message"
    echo -e "  ${CYAN}--help <cmd>${NC}   Show help for specific subcommand (run|ota|build|watch)"
    echo -e "\n${BOLD}Examples:${NC}"
    echo -e "  ${GREEN}$0 --help run${NC}       # Show help for run subcommand"
    echo -e "  ${GREEN}$0 --help ota${NC}       # Show help for ota subcommand"
    echo -e "  ${GREEN}$0 --help build${NC}     # Show help for build subcommand"
    echo -e "  ${GREEN}$0 --help watch${NC}     # Show help for watch subcommand"
}

# Parse command line arguments
SUBCOMMAND=""
OTA_ADDRESS=""
TARGET_CHIP=""
WATCH_LOGS=false
CARGO_FLAGS=()
ARGS=()
FLASH_SIZE=""

# Parse subcommand and positional arguments
if [[ $# -gt 0 && ( "$1" == "run" || "$1" == "ota" || "$1" == "build" || "$1" == "watch" ) ]]; then
    SUBCOMMAND="$1"
    shift
fi

if [[ "$SUBCOMMAND" == "ota" ]]; then
    if [[ $# -gt 0 && ! "$1" =~ ^- ]]; then
        OTA_ADDRESS="$1"
        # Store protocol and strip it from address
        if [[ "$OTA_ADDRESS" =~ ^https?:// ]]; then
            OTA_PROTOCOL="${OTA_ADDRESS%%://*}"
            OTA_ADDRESS="${OTA_ADDRESS#*://}"
        else
            OTA_PROTOCOL="http"
        fi
        shift
    fi
    if [[ $# -gt 0 && ! "$1" =~ ^- ]]; then
        TARGET_CHIP="$1"
        shift
    fi
elif [[ "$SUBCOMMAND" == "watch" ]]; then
    # No arguments needed for watch
    :
else
    if [[ $# -gt 0 && ! "$1" =~ ^- ]]; then
        TARGET_CHIP="$1"
        shift
    fi
fi

# Parse remaining arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            if [[ $# -gt 1 && ! "$2" =~ ^- ]]; then
                case "$2" in
                    run)
                        print_banner
                        show_run_help
                        exit 0
                        ;;
                    ota)
                        print_banner
                        show_ota_help
                        exit 0
                        ;;
                    build)
                        print_banner
                        show_build_help
                        exit 0
                        ;;
                    watch)
                        print_banner
                        show_watch_help
                        exit 0
                        ;;
                esac
            fi
            print_banner
            show_usage
            exit 0
            ;;
        -f|--features)
            CARGO_FLAGS+=("--features" "$2")
            shift 2
            ;;
        --watch)
            WATCH_LOGS=true
            shift
            ;;
        --release)
            CARGO_FLAGS+=("--release")
            shift
            ;;
        --flash-size)
            FLASH_SIZE=$(echo "$2" | tr '[:upper:]' '[:lower:]')
            shift 2
            ;;
        -* )
            CARGO_FLAGS+=("$1")
            shift
            ;;
        *)
            ARGS+=("$1")
            shift
            ;;
    esac
done

# Print banner
print_banner

# Show help if no subcommand is given
if [ -z "$SUBCOMMAND" ]; then
    show_usage
    exit 0
fi

# Check if chip is specified and valid (for run/ota/build)
if [[ "$SUBCOMMAND" == "run" || "$SUBCOMMAND" == "ota" || "$SUBCOMMAND" == "build" ]]; then
    if [ -z "$TARGET_CHIP" ]; then
        print_error "Chip name is required as a positional argument for $SUBCOMMAND (${SUPPORTED_CHIPS[*]})"
        show_usage
        exit 1
    fi
    if [[ ! " ${SUPPORTED_CHIPS[@]} " =~ " ${TARGET_CHIP} " ]]; then
        print_error "Unsupported chip: $TARGET_CHIP. Supported: ${SUPPORTED_CHIPS[*]}"
        show_usage
        exit 1
    fi
fi
if [[ "$SUBCOMMAND" == "ota" && -z "$OTA_ADDRESS" ]]; then
    print_error "IP address is required for ota subcommand"
    show_usage
    exit 1
fi
if [[ "$WATCH_LOGS" = true && -z "$TARGET_CHIP" && "$SUBCOMMAND" != "watch" ]]; then
    print_error "--watch requires a chip to be specified (except with watch subcommand)"
    show_usage
    exit 1
fi

# Check if espflash is installed
if ! command -v espflash &> /dev/null; then
    print_error "espflash is not installed"
    print_info "Please install it with: cargo install espflash"
    exit 1
fi

# Build type for target directory
BUILD_TYPE=debug
for arg in "${CARGO_FLAGS[@]}"; do
    if [[ "$arg" == "--release" ]]; then
        BUILD_TYPE=release
        break
    fi
done

print_step "Building target for $TARGET_CHIP ($BUILD_TYPE mode)"
cargo build "${CARGO_FLAGS[@]}"

# Only continue with firmware creation/deployment if chip is specified and not just watching
if [ -n "$TARGET_CHIP" ] && [ "$SUBCOMMAND" != "watch" ]; then
    # Set target directory for the specific chip
    TARGET_DIR="target/xtensa-${TARGET_CHIP}-espidf/${BUILD_TYPE}/e-chess"
    if [ ! -f "$TARGET_DIR" ]; then
        print_error "Build target not found at $TARGET_DIR"
        exit 1
    fi
    print_step "Creating firmware image"
    # Create firmware file name with chip in the target directory
    TARGET_BASE=$(dirname "$TARGET_DIR")
    FIRMWARE_FILE="${TARGET_BASE}/e-chess_ota_${TARGET_CHIP}.bin"
    print_progress "Generating firmware file: ${FIRMWARE_FILE}"
    # Use espflash to create the ESP32 app image
    if [ -n "$FLASH_SIZE" ]; then
        PARTITION_TABLE=$(get_partition_table "$FLASH_SIZE")
        espflash save-image --chip "$TARGET_CHIP" --partition-table "$PARTITION_TABLE" --flash-size "$FLASH_SIZE" "$TARGET_DIR" "$FIRMWARE_FILE"
    else
        espflash save-image --chip "$TARGET_CHIP" "$TARGET_DIR" "$FIRMWARE_FILE"
    fi
    print_success "Firmware file created successfully!"

    if [[ "$SUBCOMMAND" == "ota" ]]; then
        print_step "Deploying firmware to $OTA_ADDRESS"
        print_progress "Uploading firmware..."
        
        # Create a temporary file for the response
        RESPONSE_FILE=$(mktemp)
        
        # Upload with progress bar
        curl -X POST \
             -H "Content-Type: application/octet-stream" \
             --data-binary "@$FIRMWARE_FILE" \
             -w "\nUpload complete!\n" \
             -o "$RESPONSE_FILE" \
             "${OTA_PROTOCOL}://${OTA_ADDRESS}/upload-firmware"
        
        # Clean up temp file
        rm -f "$RESPONSE_FILE"
        
        print_success "Firmware deployment initiated"
        print_info "Device will restart automatically"
    fi
    if [[ "$SUBCOMMAND" == "run" ]]; then
        print_step "Running cargo run..."
        if [ -n "$FLASH_SIZE" ]; then
            PARTITION_TABLE=$(get_partition_table "$FLASH_SIZE")
            cargo run "${CARGO_FLAGS[@]}" -- --flash-size "$FLASH_SIZE" --partition-table "$PARTITION_TABLE"
        else
            cargo run "${CARGO_FLAGS[@]}"
        fi
    fi
fi

# Watch logs if requested (after any operation) or if watch subcommand
if [ "$WATCH_LOGS" = true ] && [ "$SUBCOMMAND" != "run" ]; then
    print_step "Starting debug log watch"
    print_warning "Press Ctrl+C to stop watching logs"
    espflash monitor
elif [ "$SUBCOMMAND" == "watch" ]; then
    print_step "Starting debug log watch"
    print_warning "Press Ctrl+C to stop watching logs"
    espflash monitor
fi

echo -e "\n${BLUE}=========================================${NC}"
print_success "Process completed successfully!"
echo -e "${BLUE}=========================================${NC}\n"