#!/bin/bash
# Test script for orthos-daemon

DAEMON="./target/debug/orthos-daemon"

echo "=== Test 1: Initialize with simple boundary ==="
echo '{"method": "initialize", "params": {"source": "Boundary Main { Flux x : Int Flux y : Int Law Positive : x > 0 }"}, "id": 1}' | $DAEMON 2>/dev/null | head -1

echo ""
echo "=== Test 2: Initialize + Query ==="
(
echo '{"method": "initialize", "params": {"source": "Boundary Main { Flux x : Int Law Constraint : x == 42 }"}, "id": 1}'
sleep 0.1
echo '{"method": "query", "params": {"flux": ["Main.x"]}, "id": 2}'
sleep 0.1
echo '{"method": "shutdown", "id": 99}'
) | $DAEMON 2>/dev/null

echo ""
echo "=== Test 3: Constrain + Auto-rollback on UNSAT ==="
(
echo '{"method": "initialize", "params": {"source": "Boundary Main { Flux x : Int Law Positive : x > 0 }"}, "id": 1}'
sleep 0.1
echo '{"method": "constrain", "params": {"laws": ["Main.x < 0"]}, "id": 2}'
sleep 0.1
echo '{"method": "query", "params": {"flux": ["Main.x"]}, "id": 3}'
sleep 0.1
echo '{"method": "shutdown", "id": 99}'
) | $DAEMON 2>/dev/null

echo ""
echo "=== Test 4: Checkpoint + Restore ==="
(
echo '{"method": "initialize", "params": {"source": "Boundary Main { Flux x : Int }"}, "id": 1}'
sleep 0.1
echo '{"method": "constrain", "params": {"laws": ["Main.x == 10"]}, "id": 2}'
sleep 0.1
echo '{"method": "query", "params": {"flux": ["Main.x"]}, "id": 3}'
sleep 0.1
echo '{"method": "checkpoint", "id": 4}'
sleep 0.1
echo '{"method": "constrain", "params": {"laws": ["Main.x == 20"]}, "id": 5}'
sleep 0.1
echo '{"method": "query", "params": {"flux": ["Main.x"]}, "id": 6}'
sleep 0.1
echo '{"method": "restore", "id": 7}'
sleep 0.1
echo '{"method": "query", "params": {"flux": ["Main.x"]}, "id": 8}'
sleep 0.1
echo '{"method": "shutdown", "id": 99}'
) | $DAEMON 2>/dev/null

echo ""
echo "=== Test 5: Ontological Error on Init ==="
echo '{"method": "initialize", "params": {"source": "Boundary Impossible { Law Contradiction : 1 == 2 }"}, "id": 1}' | $DAEMON 2>/dev/null | head -1
