#!/usr/bin/env bash

# Binary path
BINARY="./target/debug/chess_turnament_manager"

# Check if binary exists, build if not
if [ ! -f "$BINARY" ]; then
    echo "Building the project..."
    cargo build --quiet
fi

# Define results
RESULTS=(1 0 -1)

echo "Tournament: Alice, Bob, Charlie"
echo "Matches and result interpretation:"
echo "1. Charlie vs Bob (Result R1): 1=Charlie wins, 0=Draw, -1=Bob wins"
echo "2. Alice vs Charlie (Result R2): 1=Alice wins, 0=Draw, -1=Charlie wins"
echo "3. Bob vs Alice (Result R3): 1=Bob wins, 0=Draw, -1=Alice wins"
echo "======================================================================"

count=0
for r1 in "${RESULTS[@]}"; do
    for r2 in "${RESULTS[@]}"; do
        for r3 in "${RESULTS[@]}"; do
            count=$((count+1))
            echo "Combination $count: R1=$r1, R2=$r2, R3=$r3"
            
            # Run the manager and extract the standings
            output=$(printf "Alice\nBob\nCharlie\n\n$r1\n$r2\n$r3\n" | $BINARY)
            
            # Extract standings from the output
            echo "$output" | sed -n '/Rank | Player/,/─────┴/p'
            echo "----------------------------------------------------------------------"
        done
    done
done

echo "Total combinations tested: $count"
