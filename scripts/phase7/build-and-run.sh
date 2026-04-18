#!/bin/bash
# Build and run StratOS desktop environment
# This script rebuilds stratvm, stratpanel, and launches QEMU

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOTFS_DIR="${SCRIPT_DIR}/../../out/phase7/rootfs-minimal"
OUT_DIR="${SCRIPT_DIR}/../../out/phase7"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== StratOS Build & Run ===${NC}"

# Check if clean build requested
if [ "$1" = "--clean" ] || [ "$1" = "-c" ]; then
    echo -e "${YELLOW}Clean build requested...${NC}"
    CLEAN_BUILD=1
else
    CLEAN_BUILD=0
fi

# Kill any existing QEMU
if pgrep -x "qemu-system-x86_64" > /dev/null 2>&1; then
    echo -e "${YELLOW}Stopping existing QEMU...${NC}"
    pkill -x "qemu-system-x86_64" 2>/dev/null || true
    sleep 2
fi

# Build stratvm
echo -e "${YELLOW}Building stratvm...${NC}"
cd "${SCRIPT_DIR}/../../stratvm"
if [ $CLEAN_BUILD -eq 1 ]; then
    make clean
fi
make

# Copy stratvm to rootfs
cp stratwm "${ROOTFS_DIR}/bin/"
echo -e "${GREEN}stratvm copied to rootfs${NC}"

# Build stratpanel
echo -e "${YELLOW}Building stratpanel...${NC}"
cd "${SCRIPT_DIR}/../../stratpanel"
if [ $CLEAN_BUILD -eq 1 ]; then
    cargo clean
fi
cargo build --release

# Copy stratpanel to rootfs
cp target/release/stratpanel "${ROOTFS_DIR}/bin/"
echo -e "${GREEN}stratpanel copied to rootfs${NC}"

# Build EROFS image
echo -e "${YELLOW}Building EROFS system image...${NC}"
cd "${OUT_DIR}"
rm -f slot-system.erofs
mkfs.erofs -zlz4hc slot-system.erofs rootfs-minimal

# Run QEMU
echo -e "${GREEN}Starting QEMU...${NC}"
cd "${SCRIPT_DIR}/../.."
exec ./scripts/phase7/run-qemu-desktop.sh
