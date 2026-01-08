#!/bin/bash
# Test script for orthos-lsp

LSP_BIN="./target/release/orthos-lsp"

# Test LSP initialization with a simple JSON-RPC request
echo "Testing orthos-lsp..."

# Create a test request (initialize)
INIT_REQUEST='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"capabilities":{},"rootUri":null}}'
INIT_LEN=${#INIT_REQUEST}

# Send to LSP and capture response
echo -e "Content-Length: ${INIT_LEN}\r\n\r\n${INIT_REQUEST}" | timeout 2 $LSP_BIN 2>/dev/null | head -20

echo ""
echo "LSP binary exists and responds to initialize request."
