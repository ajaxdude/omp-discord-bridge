#!/bin/bash
# Discord Bridge Testing Checklist
# Run this script to verify your setup before testing

set -e

echo "================================"
echo "Discord Bridge Setup Checker"
echo "================================"
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check functions
check_pass() {
    echo -e "${GREEN}✓${NC} $1"
}

check_fail() {
    echo -e "${RED}✗${NC} $1"
}

check_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# 1. Check Rust
echo "1. Checking Rust installation..."
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version)
    check_pass "Rust installed: $RUST_VERSION"
else
    check_fail "Rust not installed"
    echo "   Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
fi
echo ""

# 2. Check Cargo
echo "2. Checking Cargo installation..."
if command -v cargo &> /dev/null; then
    CARGO_VERSION=$(cargo --version)
    check_pass "Cargo installed: $CARGO_VERSION"
else
    check_fail "Cargo not installed"
fi
echo ""

# 3. Check OMP
echo "3. Checking OMP installation..."
if command -v omp &> /dev/null; then
    OMP_PATH=$(which omp)
    check_pass "OMP found at: $OMP_PATH"
    
    # Check OMP version
    if omp --version &> /dev/null; then
        OMP_VERSION=$(omp --version 2>&1 | head -1)
        check_pass "OMP version: $OMP_VERSION"
    else
        check_warn "Could not determine OMP version"
    fi
else
    check_fail "OMP not found in PATH"
    echo "   Please install OMP first"
fi
echo ""

# 4. Check OMP RPC mode
echo "4. Checking OMP RPC mode..."
if command -v omp &> /dev/null; then
    if omp --mode rpc --help &> /dev/null 2>&1; then
        check_pass "OMP supports --mode rpc"
    else
        check_fail "OMP does not support --mode rpc"
    fi
    
    # Test RPC mode with correlation ID
    echo "   Testing RPC mode with correlation ID..."
    if echo '{"type":"prompt","id":"test-123","message":"hello"}' | omp --mode rpc 2>&1 | grep -q "id"; then
        check_pass "OMP RPC mode supports correlation IDs"
    else
        check_fail "OMP RPC mode does not support correlation IDs"
        echo "   Apply patches from omp-patch/ directory"
    fi
fi
echo ""

# 5. Check .env file
echo "5. Checking .env configuration..."
if [ -f ".env" ]; then
    check_pass ".env file exists"
    
    # Check DISCORD_TOKEN
    if grep -q "DISCORD_TOKEN=" .env; then
        if grep -q "DISCORD_TOKEN=your_discord_bot_token_here" .env; then
            check_fail "DISCORD_TOKEN not configured (still has placeholder)"
        elif grep -q "^DISCORD_TOKEN=MTA" .env; then
            check_pass "DISCORD_TOKEN appears to be configured"
        else
            check_warn "DISCORD_TOKEN format looks unusual"
        fi
    else
        check_fail "DISCORD_TOKEN not found in .env"
    fi
    
    # Check DISCORD_PREFIX
    if grep -q "DISCORD_PREFIX=" .env; then
        PREFIX=$(grep "DISCORD_PREFIX=" .env | cut -d'=' -f2)
        check_pass "DISCORD_PREFIX configured: $PREFIX"
    fi
    
    # Check OMP_PATH
    if grep -q "^OMP_PATH=" .env; then
        OMP_PATH=$(grep "^OMP_PATH=" .env | cut -d'=' -f2)
        check_pass "OMP_PATH configured: $OMP_PATH"
    fi
else
    check_fail ".env file not found"
    echo "   Create with: cp .env.example .env"
fi
echo ""

# 6. Check source files
echo "6. Checking source files..."
FILES=(
    "src/main.rs"
    "src/discord.rs"
    "src/config.rs"
    "src/error.rs"
    "src/rpc/client.rs"
    "src/rpc/types.rs"
    "src/rpc/mod.rs"
)

ALL_FILES_PRESENT=true
for file in "${FILES[@]}"; do
    if [ -f "$file" ]; then
        echo "   ✓ $file"
    else
        echo "   ✗ $file (missing)"
        ALL_FILES_PRESENT=false
    fi
done

if [ "$ALL_FILES_PRESENT" = true ]; then
    check_pass "All source files present"
else
    check_fail "Some source files missing"
fi
echo ""

# 7. Check if already built
echo "7. Checking if project is built..."
if [ -f "target/release/omp_discord_bridge" ]; then
    check_pass "Release binary exists"
    SIZE=$(du -h target/release/omp_discord_bridge | cut -f1)
    echo "   Size: $SIZE"
else
    check_warn "Release binary not built yet"
    echo "   Build with: cargo build --release"
fi
echo ""

# 8. Check Discord bot setup
echo "8. Discord bot setup reminder..."
echo "   Ensure you have:"
echo "   • Created Discord application"
echo "   • Generated bot token"
echo "   • Enabled Message Content Intent"
echo "   • Invited bot to your server"
echo "   • Bot has permissions: Read Messages, Send Messages"
echo ""

# 9. Summary
echo "================================"
echo "Setup Check Complete!"
echo "================================"
echo ""
echo "Next steps:"
echo "1. If all checks passed: ./test_checklist.sh passed"
echo "2. Build the bot: cargo build --release"
echo "3. Run the bot: cargo run"
echo "4. Test with Discord commands: !ping, !help, !omp test"
echo ""
echo "For detailed testing instructions, see: DETAILED_TESTING_GUIDE.md"
echo ""