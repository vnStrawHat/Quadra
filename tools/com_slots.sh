#!/usr/bin/env bash
# Print the vtable slot index of every method of a COM interface, walking the
# inheritance chain, read straight from the Windows SDK headers.
# Usage: tools/com_slots.sh IDWriteFactory2 [method-filter-regex]
set -euo pipefail
SDK_INC="${SDK_INC:-/c/Program Files (x86)/Windows Kits/10/Include/10.0.22621.0}"
iface="$1"
filter="${2:-.}"

# Echo "Base<TAB>method..." for one interface declaration (C++ form).
methods_of() {
	local name="$1" hit file line
	hit=$(grep -n -E "(^|[ \t\"\)])${name} : public [A-Za-z0-9_]+" "$SDK_INC"/um/*.h "$SDK_INC"/shared/*.h | head -1) || true
	[ -n "$hit" ] || { echo "interface not found: $name" >&2; exit 1; }
	file="${hit%%:*}"; line="${hit#*:}"; line="${line%%:*}"
	awk -v start="$line" -v name="$name" '
		NR == start {
			match($0, name " : public [A-Za-z0-9_]+"); base = substr($0, RSTART + length(name) + 10, RLENGTH - length(name) - 10)
			printf "%s", base
		}
		NR >= start {
			if (match($0, /STDMETHOD_?\([^)]*\)/)) {
				s = substr($0, RSTART, RLENGTH); gsub(/STDMETHOD_?\(|\)| /, "", s); n = split(s, p, ","); printf "\t%s", p[n]
			} else if (match($0, /virtual .*STDMETHODCALLTYPE [A-Za-z0-9_]+\(/)) {
				s = substr($0, RSTART, RLENGTH); sub(/.*STDMETHODCALLTYPE /, "", s); sub(/\(/, "", s); printf "\t%s", s
			}
			o = gsub(/\{/, "{"); c = gsub(/\}/, "}"); depth += o - c; if (o > 0) opened = 1
			if (opened && depth <= 0) { print ""; exit }
		}' "$file"
}

chain=()
cur="$iface"
while [ "$cur" != "IUnknown" ]; do
	row=$(methods_of "$cur")
	chain=("$cur:$row" "${chain[@]}")
	cur="${row%%$'\t'*}"
done

idx=0
for m in QueryInterface AddRef Release; do
	echo "$m" | grep -qE "$filter" && printf '%3d IUnknown::%s\n' $idx $m
	idx=$((idx + 1))
done
for entry in "${chain[@]}"; do
	name="${entry%%:*}"; rest="${entry#*:}"
	IFS=$'\t' read -r -a parts <<< "$rest"
	seen=" "
	for m in "${parts[@]:1}"; do
		case "$seen" in *" $m "*) echo "WARNING: overloaded virtual $name::$m - MSVC orders overloads in reverse, verify by hand" >&2 ;; esac
		seen="$seen$m "
		echo "$m" | grep -qE "$filter" && printf '%3d %s::%s\n' $idx "$name" "$m"
		idx=$((idx + 1))
	done
done
