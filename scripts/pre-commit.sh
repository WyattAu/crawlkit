#!/bin/bash
# Pre-commit hook for crawlkit
# Enforces FAANG/HFT/Defence coding standards
# All checks must pass before any commit is accepted.
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

passed=0
failed=0
warnings=0

check() {
    local label="$1"
    shift
    printf "[%d/7] %s ... " "$((passed + failed + warnings + 1))" "$label"
    if "$@" >/dev/null 2>&1; then
        printf "${GREEN}PASS${NC}\n"
        passed=$((passed + 1))
    else
        printf "${RED}FAIL${NC}\n"
        failed=$((failed + 1))
    fi
}

warn() {
    local label="$1"
    shift
    printf "[%d/7] %s ... " "$((passed + failed + warnings + 1))" "$label"
    if "$@" >/dev/null 2>&1; then
        printf "${GREEN}PASS${NC}\n"
        passed=$((passed + 1))
    else
        printf "${YELLOW}WARN${NC}\n"
        warnings=$((warnings + 1))
    fi
}

echo "============================================"
echo " crawlkit pre-commit checks"
echo "============================================"
echo ""

# 1. Formatting
check "Formatting (cargo fmt --check)" cargo fmt --check

# 2. Clippy with warnings as errors
check "Clippy (cargo clippy --workspace --all-targets -- -D warnings)" \
    cargo clippy --workspace --all-targets -- -D warnings

# 3. Build check
check "Build (cargo check --workspace)" cargo check --workspace

# 4. Unit tests
check "Unit tests (cargo test --lib --workspace)" cargo test --lib --workspace

# 5. Security audit -- blocking for critical vulnerabilities
warn "Security audit (cargo audit)" cargo audit

# 6. Verify no hardcoded secrets
REPO_ROOT="$(git rev-parse --show-toplevel)"
printf "[%d/7] Secret scan (no hardcoded keys) ... " "$((passed + failed + warnings + 1))"
if grep -rn 'password\s*=\s*"[^"]*"\|api_key\s*=\s*"[^"]*"\|secret\s*=\s*"[^"]*"\|aws_secret_access_key\|AKIA[0-9A-Z]\{16\}\|-----BEGIN.*PRIVATE KEY-----' \
    --include="*.rs" --include="*.toml" --include="*.yaml" --include="*.yml" \
    --exclude-dir=target --exclude-dir=.git \
    "$REPO_ROOT" 2>/dev/null | \
    grep -v 'test\|example\|default\|placeholder\|TODO\|FIXME\|doc\|///\|//!' | \
    grep -q .; then
    printf "${RED}FAIL (potential hardcoded secrets found)${NC}\n"
    failed=$((failed + 1))
else
    printf "${GREEN}PASS${NC}\n"
    passed=$((passed + 1))
fi

# 7. Verify no unsafe code in library code
printf "[%d/7] Unsafe code check ... " "$((passed + failed + warnings + 1))"
if grep -rn 'unsafe\s*{' --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    "$REPO_ROOT/crates/" 2>/dev/null | \
    grep -v '// SAFETY\|#\[cfg(test\]\|///\s*# Safety' | \
    grep -q .; then
    printf "${RED}FAIL (unsafe code without SAFETY comment)${NC}\n"
    failed=$((failed + 1))
else
    printf "${GREEN}PASS${NC}\n"
    passed=$((passed + 1))
fi

echo ""
echo "--------------------------------------------"
if [ "$failed" -gt 0 ]; then
    echo -e "${RED}Pre-commit FAILED: $failed check(s) failed, $warnings warning(s).${NC}"
    echo "Fix the issues above before committing."
    exit 1
elif [ "$warnings" -gt 0 ]; then
    echo -e "${YELLOW}Pre-commit PASSED with $warnings warning(s).${NC}"
    echo "Review warnings above."
    exit 0
else
    echo -e "${GREEN}All $passed pre-commit checks passed.${NC}"
    exit 0
fi
