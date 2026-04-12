#!/bin/sh
# cite-lint — mechanical citation key checker for psh
#
# Verifies that every [Key] citation in source files resolves to
# an entry in docs/citations.md, and reports unused bibliography
# entries and alias conflicts.
#
# DOES NOT verify semantic correctness. See docs/citation-workflow.md.

set -eu

BIB="docs/citations.md"
TMPDIR="${TMPDIR:-/tmp}"
tmp="$TMPDIR/cite-lint.$$"
trap 'rm -f "$tmp".*' EXIT

# --- Extract bibliography keys and aliases ---

if [ ! -f "$BIB" ]; then
    printf 'cite-lint: %s not found. Create the bibliography first.\n' "$BIB" >&2
    exit 1
fi

# Canonical keys: lines matching ### `[Key]`
awk '/^### `\[/ { gsub(/^### `\[|\]`.*$/, ""); print }' "$BIB" | sort > "$tmp.bib_keys"

# Aliases: lines matching **Alias:** `[Key]`
awk '/^\*\*Alias:\*\* `\[/ { gsub(/^.*`\[|\]`.*$/, ""); print }' "$BIB" | sort > "$tmp.aliases"

# All valid keys (canonical + aliases)
sort -u "$tmp.bib_keys" "$tmp.aliases" > "$tmp.all_valid"

# --- Check for alias conflicts ---

alias_dupes=$(sort "$tmp.aliases" | uniq -d)
if [ -n "$alias_dupes" ]; then
    printf 'cite-lint: ALIAS CONFLICT — these aliases resolve to multiple entries:\n'
    printf '  %s\n' $alias_dupes
fi

# --- Check for key/alias overlap ---

key_alias_overlap=$(comm -12 "$tmp.bib_keys" "$tmp.aliases" 2>/dev/null) || true
if [ -n "$key_alias_overlap" ]; then
    printf 'cite-lint: WARNING — key is also declared as an alias:\n'
    printf '  %s\n' $key_alias_overlap
fi

# --- Strip fenced code blocks from markdown, then extract keys ---
#
# For .rs files: scan directly (no fenced blocks).
# For .md files: remove ``` ... ``` regions before scanning.
# Excludes: citations.md, citation-workflow.md (meta-documents).

# awk script: strip fenced code blocks from markdown
strip_fences='
/^```/ { infence = !infence; next }
!infence { print }
'

: > "$tmp.found"
: > "$tmp.files"

# Key pattern: [UpperStart][AlphaNum]*[Digits] or [ALLCAPS][OptDigits]
# Matches: [CH00], [CBG24], [MMM], [BTMO23], [CMS], [Duf90], etc.
KEY_RE='\[([A-Z][A-Za-z]*[0-9]{2,4}|[A-Z]{2,}[0-9]*)\]'

for dir in src docs; do
    [ -d "$dir" ] || continue
    find "$dir" -type f \( -name '*.rs' -o -name '*.md' \) \
        ! -name citations.md \
        ! -name citation-workflow.md \
        | while read -r file; do
        case "$file" in
            *.md)
                keys=$(awk "$strip_fences" "$file" \
                    | grep -oE "$KEY_RE" 2>/dev/null \
                    | sed 's/^\[//;s/\]$//' \
                    | sort -u) || true
                ;;
            *.rs)
                keys=$(grep -oE "$KEY_RE" "$file" 2>/dev/null \
                    | sed 's/^\[//;s/\]$//' \
                    | sort -u) || true
                ;;
            *)
                continue
                ;;
        esac
        if [ -n "$keys" ]; then
            printf '%s\n' "$keys" >> "$tmp.found"
            printf '%s\n' "$file" >> "$tmp.files"
        fi
    done
done

sort -u "$tmp.found" > "$tmp.found_uniq"
files_with_cites=$(sort -u "$tmp.files" | wc -l | tr -d ' ')

# --- Check for unresolved keys ---

unresolved_count=0
while IFS= read -r key; do
    [ -z "$key" ] && continue
    if ! grep -qxF "$key" "$tmp.all_valid"; then
        printf 'cite-lint: UNRESOLVED — [%s] not found in %s\n' "$key" "$BIB"
        unresolved_count=$((unresolved_count + 1))
    fi
done < "$tmp.found_uniq"

# --- Check for unused bibliography entries ---

unused_count=0
while IFS= read -r key; do
    [ -z "$key" ] && continue
    if ! grep -qxF "$key" "$tmp.found_uniq" 2>/dev/null; then
        # Check if an alias of this key is used
        # For now, report as unused — alias resolution is TODO
        printf 'cite-lint: UNUSED — [%s] has a bibliography entry but no code citations\n' "$key"
        unused_count=$((unused_count + 1))
    fi
done < "$tmp.bib_keys"

# --- Check for NEEDS BACKFILL entries ---

backfill_count=$(grep -c 'NEEDS BACKFILL' "$BIB" 2>/dev/null || echo 0)
backfill_count=$(echo "$backfill_count" | tr -d ' ')

# --- Summary ---

resolved_count=0
while IFS= read -r key; do
    [ -z "$key" ] && continue
    if grep -qxF "$key" "$tmp.all_valid"; then
        resolved_count=$((resolved_count + 1))
    fi
done < "$tmp.found_uniq"

printf '\ncite-lint: %s keys resolved across %s files (%s unresolved, %s unused entries, %s NEEDS BACKFILL).\n' \
    "$resolved_count" "$files_with_cites" "$unresolved_count" "$unused_count" "$backfill_count"

# --- Unconditional disclaimer — cannot be silenced ---

cat <<'DISCLAIMER'

Mechanical check only. Semantic correctness (whether citations
accurately reflect the code they document) is the reviewer's
and auditor's responsibility. Green cite-lint ≠ correct
citations.
DISCLAIMER

if [ "$unresolved_count" -gt 0 ]; then
    exit 1
fi
exit 0
