#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

SKILLS_DIR="$ROOT_DIR/skills"
OUTPUT_DIR="$ROOT_DIR/target/skills-archive"
OUTPUT_FILE="$OUTPUT_DIR/px-skills.zip"

echo "Packaging skills from $SKILLS_DIR"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Create zip archive with skills (excluding templates)
cd "$SKILLS_DIR"
zip -r "$OUTPUT_FILE" \
    ask-px \
    character-reference-sheet \
    px-repo \
    px-resolve \
    px-update \
    -x "*.DS_Store" \
    -x "templates/*"
cd "$ROOT_DIR"

echo "Skills packaged to $OUTPUT_FILE"

# Show contents
echo "Archive contents:"
unzip -l "$OUTPUT_FILE"
