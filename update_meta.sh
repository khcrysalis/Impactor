#!/usr/bin/env bash
set -euo pipefail

# ------------------------------------------------------------
# Config
# ------------------------------------------------------------
REPO="khcrysalis/Impactor"
API="https://api.github.com/repos/$REPO/releases"
OUT_DIR="upload"
OUT_FILE="$OUT_DIR/dev.khcrysalis.PlumeImpactor.metainfo.xml"

mkdir -p "$OUT_DIR"

# ------------------------------------------------------------
# Helpers
# ------------------------------------------------------------
xml_escape() {
  sed -e 's/&/\&amp;/g' \
      -e 's/</\&lt;/g' \
      -e 's/>/\&gt;/g'
}

# ------------------------------------------------------------
# Fetch releases
# ------------------------------------------------------------
echo "Fetching releases…"
RELEASES_JSON="$(curl -fsSL -H "Accept: application/vnd.github+json" "$API")"

LATEST_RELEASE="$(echo "$RELEASES_JSON" | jq '.[0]')"
VERSION="$(echo "$LATEST_RELEASE" | jq -r '.name')"
DATE="$(echo "$LATEST_RELEASE" | jq -r '.published_at | split("T")[0]')"
RELEASE_URL="$(echo "$LATEST_RELEASE" | jq -r '.html_url')"
BODY="$(echo "$LATEST_RELEASE" | jq -r '.body')"

echo "Latest version: $VERSION ($DATE)"

# ------------------------------------------------------------
# Download an existing metainfo.xml from releases
# ------------------------------------------------------------
META_URL="$(
  echo "$RELEASES_JSON" |
    jq -r '
      .[] |
      .assets[]? |
      select(.name | endswith(".metainfo.xml")) |
      .browser_download_url
    ' |
    head -n1
)"

if [[ -z "$META_URL" ]]; then
  echo "❌ Could not find an existing metainfo.xml in any release"
  exit 1
fi

echo "Downloading existing metainfo.xml…"
curl -fsSL "$META_URL" -o "$OUT_FILE"

# ------------------------------------------------------------
# Abort if release already exists
# ------------------------------------------------------------
if xmllint --xpath "//release[@version='$VERSION']" "$OUT_FILE" >/dev/null 2>&1; then
  echo "Latest release already present, nothing to do"
  exit 0
fi

# ------------------------------------------------------------
# Build <description> XML
# ------------------------------------------------------------
DESCRIPTION_TMP="$(mktemp)"
IN_LIST=0

while IFS= read -r line; do
  line="${line//$'\r'/}"

  [[ "$line" == *"New Contributors"* ]] && break

  if [[ "$line" == "## "* ]]; then
    [[ $IN_LIST -eq 1 ]] && echo "</ul>" >> "$DESCRIPTION_TMP" && IN_LIST=0
    printf "<p>%s</p>\n" \
      "$(echo "${line#\#\# }" | xml_escape)" >> "$DESCRIPTION_TMP"

  elif [[ "$line" == "* "* ]]; then
    [[ $IN_LIST -eq 0 ]] && echo "<ul>" >> "$DESCRIPTION_TMP" && IN_LIST=1
    CLEAN="$(echo "${line#\* }" | sed 's/in https:\/\/github.com.*//')"
    printf "<li>%s</li>\n" \
      "$(echo "$CLEAN" | xml_escape)" >> "$DESCRIPTION_TMP"

  else
    [[ $IN_LIST -eq 1 ]] && echo "</ul>" >> "$DESCRIPTION_TMP" && IN_LIST=0
  fi
done <<< "$BODY"

[[ $IN_LIST -eq 1 ]] && echo "</ul>" >> "$DESCRIPTION_TMP"

# ------------------------------------------------------------
# Create <release> block
# ------------------------------------------------------------
RELEASE_TMP="$(mktemp)"
cat > "$RELEASE_TMP" <<EOF
<release version="$VERSION" date="$DATE" type="stable">
  <url>$RELEASE_URL</url>
  <description>
$(sed 's/^/    /' "$DESCRIPTION_TMP")
  </description>
</release>
EOF

# ------------------------------------------------------------
# Insert release after <releases>
# ------------------------------------------------------------
OUTPUT_TMP="$(mktemp)"

awk '
  /<releases>/ && !done {
    print
    while ((getline line < "'"$RELEASE_TMP"'") > 0)
      print line
    close("'"$RELEASE_TMP"'")
    done = 1
    next
  }
  { print }
' "$OUT_FILE" > "$OUTPUT_TMP"

mv "$OUTPUT_TMP" "$OUT_FILE"

# ------------------------------------------------------------
# Format XML
# ------------------------------------------------------------
xmllint --format "$OUT_FILE" --output "$OUT_FILE"

# ------------------------------------------------------------
# Cleanup
# ------------------------------------------------------------
rm -f "$DESCRIPTION_TMP" "$RELEASE_TMP"

echo "✅ Updated metainfo written to $OUT_FILE"
