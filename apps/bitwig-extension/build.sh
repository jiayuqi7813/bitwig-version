#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="$ROOT_DIR/build"
CLASSES_DIR="$BUILD_DIR/classes"
DIST_DIR="$ROOT_DIR/dist"

API_JAR="${BITWIG_API_JAR:-}"
if [[ -z "$API_JAR" ]]; then
  echo "ERROR: Please set BITWIG_API_JAR to your Bitwig extension-api jar path."
  echo "Example: export BITWIG_API_JAR=\"$HOME/Bitwig Studio/resources/extension-api.jar\""
  exit 1
fi

if [[ ! -f "$API_JAR" ]]; then
  echo "ERROR: BITWIG_API_JAR does not exist: $API_JAR"
  exit 1
fi

rm -rf "$BUILD_DIR" "$DIST_DIR"
mkdir -p "$CLASSES_DIR" "$DIST_DIR"

# Compile Java extension classes
find "$ROOT_DIR/src/main/java" -name '*.java' > "$BUILD_DIR/sources.list"
javac --release 11 -cp "$API_JAR" -d "$CLASSES_DIR" @"$BUILD_DIR/sources.list"

# Copy resource files (META-INF/services)
cp -R "$ROOT_DIR/src/main/resources/." "$CLASSES_DIR/"

# Package as .bwextension (zip/jar format)
EXTENSION_FILE="$DIST_DIR/BitwigVersions.bwextension"
(
  cd "$CLASSES_DIR"
  jar cf "$EXTENSION_FILE" .
)

echo "Built: $EXTENSION_FILE"
