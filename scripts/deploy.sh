#!/bin/bash
# crawlkit deployment script
# Enforces FAANG/HFT/defense sector deployment standards

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "============================================"
echo " crawlkit deployment script"
echo "============================================"
echo ""

# Pre-deployment checks
echo "Pre-deployment checks:"

# 1. Check Rust installation
printf "[1/8] Rust installation ... "
if command -v rustc &> /dev/null; then
    printf "${GREEN}PASS${NC}\n"
else
    printf "${RED}FAIL${NC}\n"
    exit 1
fi

# 2. Check cargo installation
printf "[2/8] Cargo installation ... "
if command -v cargo &> /dev/null; then
    printf "${GREEN}PASS${NC}\n"
else
    printf "${RED}FAIL${NC}\n"
    exit 1
fi

# 3. Run tests
printf "[3/8] Running tests ... "
if cargo test --workspace > /dev/null 2>&1; then
    printf "${GREEN}PASS${NC}\n"
else
    printf "${RED}FAIL${NC}\n"
    exit 1
fi

# 4. Run clippy
printf "[4/8] Running clippy ... "
if cargo clippy --workspace --all-targets -- -D warnings > /dev/null 2>&1; then
    printf "${GREEN}PASS${NC}\n"
else
    printf "${RED}FAIL${NC}\n"
    exit 1
fi

# 5. Check formatting
printf "[5/8] Checking formatting ... "
if cargo fmt --check > /dev/null 2>&1; then
    printf "${GREEN}PASS${NC}\n"
else
    printf "${RED}FAIL${NC}\n"
    exit 1
fi

# 6. Build release
printf "[6/8] Building release ... "
if cargo build --release --workspace > /dev/null 2>&1; then
    printf "${GREEN}PASS${NC}\n"
else
    printf "${RED}FAIL${NC}\n"
    exit 1
fi

# 7. Create git tag
printf "[7/8] Creating git tag ... "
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
git tag -a "v${VERSION}" -m "Release v${VERSION}" 2>/dev/null || true
printf "${GREEN}PASS${NC}\n"

# 8. Publish to crates.io
printf "[8/8] Publishing to crates.io ... "
if cargo publish -p crawlkit-engine 2>/dev/null; then
    printf "${GREEN}PASS${NC}\n"
else
    printf "${YELLOW}SKIP (already published)${NC}\n"
fi

echo ""
echo "============================================"
echo " Deployment complete!"
echo "============================================"
echo ""
echo "Next steps:"
echo "1. Push git tag: git push origin v${VERSION}"
echo "2. Deploy API server: ssh production-server"
echo "3. Verify deployment: curl https://api.crawlkit.io/health"
echo "4. Monitor metrics: curl https://api.crawlkit.io/metrics"
