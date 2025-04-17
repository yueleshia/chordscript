#!/bin/sh

WD="$( dirname "$0"; printf a )"; WD="${WD%?a}"
cd "${WD}" || { printf "Could not cd to directory of '%s'" "$0" >&2; exit 1; }

publish="${1:-publish}"
[ -d "${publish}" ] && rm -r "${publish}"
wasm-pack build --target web --out-dir "${publish}" || exit "$?"
rsync -r "public/" "${publish}"
