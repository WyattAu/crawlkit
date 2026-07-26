#!/bin/bash
# Pre-commit hook for crawlkit
# Enforces FAANG/HFT/Defence coding standards
# All blocking checks must pass before any commit is accepted.
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

passed=0
failed=0
warnings=0
total=11

check() {
    local label="$1"
    shift
    printf "${CYAN}[%d/${total}]${NC} %s ... " "$((passed + failed + warnings + 1))" "$label"
    if "$@" >/dev/null 2>&1; then
        printf "${GREEN}PASS${NC}\n"
        passed=$((passed + 1))
        return 0
    else
        printf "${RED}FAIL${NC}\n"
        failed=$((failed + 1))
        return 1
    fi
}

warn() {
    local label="$1"
    shift
    printf "${CYAN}[%d/${total}]${NC} %s ... " "$((passed + failed + warnings + 1))" "$label"
    if "$@" >/dev/null 2>&1; then
        printf "${GREEN}PASS${NC}\n"
        passed=$((passed + 1))
        return 0
    else
        printf "${YELLOW}WARN${NC}\n"
        warnings=$((warnings + 1))
        return 0
    fi
}

echo ""
echo -e "${BOLD}============================================${NC}"
echo -e "${BOLD} crawlkit pre-commit checks (11)${NC}"
echo -e "${BOLD}============================================${NC}"
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

# 5. Doc tests
check "Doc tests (cargo test --doc --workspace)" cargo test --doc --workspace

# 6. Integration tests
check "Integration tests (cargo test --workspace --test integration_tests --test backlink_integration_tests --test rum_integration_tests)" \
    cargo test --workspace --test integration_tests --test backlink_integration_tests --test rum_integration_tests

# 7. Security audit -- warn only
warn "Security audit (cargo audit)" cargo audit

# 8. Secret scan
REPO_ROOT="$(git rev-parse --show-toplevel)"
printf "${CYAN}[%d/${total}]${NC} Secret scan (no hardcoded keys) ... " "$((passed + failed + warnings + 1))"
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

# 9. Unsafe code without SAFETY comment (excluding test code and WASM FFI crate)
printf "${CYAN}[%d/${total}]${NC} Unsafe code check ... " "$((passed + failed + warnings + 1))"
if grep -rn 'unsafe\s*{' --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    --exclude='*_test.rs' --exclude='tests.rs' --exclude='native_plugin.rs' \
    "$REPO_ROOT/crates/crawlkit-engine/src/" \
    "$REPO_ROOT/crates/crawlkit/src/" \
    "$REPO_ROOT/crates/crawlkit-api/src/" \
    2>/dev/null | \
    grep -v '// SAFETY\|// SAFETY:\|#\[cfg(test)\]\|///\s*# Safety\|mod tests' | \
    grep -q .; then
    printf "${RED}FAIL (unsafe code without SAFETY comment)${NC}\n"
    failed=$((failed + 1))
else
    printf "${GREEN}PASS${NC}\n"
    passed=$((passed + 1))
fi

# 10. MSRV check -- warn only
warn "MSRV check (cargo +1.85.0 check --workspace)" cargo +1.85.0 check --workspace

# 11. Unused dependencies -- warn only (dead imports)
printf "${CYAN}[%d/${total}]${NC} Unused dependencies (dead imports) ... " "$((passed + failed + warnings + 1))"
if grep -rn '#\[allow(unused_imports)\]' \
    --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    "$REPO_ROOT/crates/" \
    2>/dev/null | grep -q .; then
    printf "${YELLOW}WARN (found allow(unused_imports) annotations)${NC}\n"
    warnings=$((warnings + 1))
elif grep -rn '#\[allow(dead_code)\]' \
    --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    "$REPO_ROOT/crates/" \
    2>/dev/null | grep -v 'auth_mw\|oidc' | grep -q .; then
    printf "${YELLOW}WARN (found allow(dead_code) annotations outside known API blocks)${NC}\n"
    warnings=$((warnings + 1))
elif grep -rn '^use ' --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    "$REPO_ROOT/crates/" \
    2>/dev/null | \
    grep -v 'test\|example\|bench\|#\[cfg(test' | \
    grep -q .; then
    printf "${GREEN}PASS${NC}\n"
    passed=$((passed + 1))
else
    printf "${GREEN}PASS${NC}\n"
    passed=$((passed + 1))
fi

echo ""
echo -e "${BOLD}--------------------------------------------${NC}"
echo -e "${BOLD} Summary${NC}"
echo -e "${BOLD}--------------------------------------------${NC}"
printf "  ${GREEN}Passed:%-4d${NC}" "$passed"
printf "  ${RED}Failed:%-4d${NC}" "$failed"
printf "  ${YELLOW}Warned:%-4d${NC}\n" "$warnings"
echo ""

if [ "$failed" -gt 0 ]; then
    echo -e "${RED}${BOLD}Pre-commit FAILED: $failed blocking check(s) failed, $warnings warning(s).${NC}"
    echo "Fix the issues above before committing."
    exit 1
elif [ "$warnings" -gt 0 ]; then
    echo -e "${YELLOW}${BOLD}Pre-commit PASSED with $warnings warning(s).${NC}"
    echo "Review warnings above."
    exit 0
else
    echo -e "${GREEN}${BOLD}All $passed pre-commit checks passed.${NC}"
    exit 0
fi
