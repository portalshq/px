#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

SKILLS_DIR="$ROOT_DIR/skills"
OUTPUT_DIR="$ROOT_DIR/target/skills-archive"
OUTPUT_FILE="$OUTPUT_DIR/px-skills.tar.gz"

echo "Packaging skills from $SKILLS_DIR"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Create tar.gz archive with skills (excluding templates)
tar -czf "$OUTPUT_FILE" \
    -C "$SKILLS_DIR" \
    --exclude='.DS_Store' \
    --exclude='templates' \
    ask-px \
    character-reference-sheet \
    px-repo \
    px-resolve \
    px-update

echo "Skills packaged to $OUTPUT_FILE"

# Show contents
echo "Archive contents:"
tar -tzf "$OUTPUT_FILE"
