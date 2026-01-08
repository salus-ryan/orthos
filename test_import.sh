#!/bin/bash
# Test the import system with the standard library

set -e

echo "=== Testing ORTHOS Import System ==="

# Build the daemon
echo "Building orthos-daemon..."
cargo build --package orthos-daemon --release 2>/dev/null

# Create a test file that imports physics
cat > /tmp/test_import.orth << 'EOF'
import "std/physics.orth" as Physics

Boundary TestPhysics <-> {
    Flux mass : Int
    Flux velocity : Int
    Flux momentum : Int
    
    // Use physics laws
    Law MassValue : mass == 5
    Law VelocityValue : velocity == 10
    Law MomentumCalc : momentum == mass * velocity
}
EOF

echo ""
echo "=== Test Source ==="
cat /tmp/test_import.orth

echo ""
echo "=== Starting Daemon ==="

# Run daemon with test
cd "$(dirname "$0")"
echo '{"jsonrpc":"2.0","method":"initialize","params":{"source":"import \"std/physics.orth\" as Physics\n\nBoundary Test <-> {\n    Flux x : Int\n    Law Positive : x > 0\n    Goal Small : x < 100 @5\n}"},"id":1}' | timeout 5 cargo run --package orthos-daemon --release 2>/dev/null || echo "(daemon exited)"

echo ""
echo "=== Import System Test Complete ==="
