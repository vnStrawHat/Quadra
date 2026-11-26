#!/usr/bin/env bash
# tools/package_mac.sh <example|markdown>: on a Mac, the app as a bundle (specs/16, "On
# macOS"; specs/23, "The app bundle"): target/package/<Name>.app with the binary (the dist
# profile, or the one PACKAGE_PROFILE names: `dev` to try the bundle's making), an
# Info.plist that names it, the kinds of file it opens and, when the system's tools make one
# of the app's .ico, its icon. Signed ad hoc, which runs on the Mac that made it.
# PACKAGE_ID names another identifier than io.github.vnstrawhat.<name>.
# ponytail: no Developer ID signature and no notarization; the bundle of an app that ships
# needs both, and a universal binary if Intel Macs are served.
set -euo pipefail
cd "$(dirname "$0")/.."
[ "$(uname)" = Darwin ] || { echo "package_mac: a bundle is made on a Mac" >&2; exit 1; }
name="${1:?usage: tools/package_mac.sh <example|markdown>}"
kinds="" icon=""
case "$name" in
markdown) crate=markdown bin=markdown title=Markdown kinds="md markdown" icon=apps/markdown/assets/logo.ico ;;
*)
	[ -d "examples/$name" ] || { echo "package_mac: no examples/$name" >&2; exit 1; }
	crate="quadra-$name" bin="$name"
	title="$(printf '%s' "$name" | cut -c1 | tr '[:lower:]' '[:upper:]')$(printf '%s' "$name" | cut -c2-)"
	;;
esac
profile="${PACKAGE_PROFILE:-dist}"
tools/cargo.sh build --profile "$profile" -p "$crate" --bin "$bin"
built="target/$profile"
[ "$profile" = dev ] && built=target/debug
version=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"$/\1/p' Cargo.toml)

app="${PACKAGE_OUT:-target/package}/$title.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$built/$bin" "$app/Contents/MacOS/$bin"

icon_key=""
if [ -n "$icon" ] && sips -s format icns "$icon" --out "$app/Contents/Resources/$bin.icns" > /dev/null 2>&1; then
	icon_key="<key>CFBundleIconFile</key><string>$bin</string>"
else
	rm -f "$app/Contents/Resources/$bin.icns"
fi
types=""
if [ -n "$kinds" ]; then
	extensions=$(for kind in $kinds; do printf '<string>%s</string>' "$kind"; done)
	types="<key>CFBundleDocumentTypes</key><array><dict>
		<key>CFBundleTypeName</key><string>$title document</string>
		<key>CFBundleTypeRole</key><string>Editor</string>
		<key>LSHandlerRank</key><string>Alternate</string>
		<key>CFBundleTypeExtensions</key><array>$extensions</array>
	</dict></array>"
fi
cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key><string>$title</string>
	<key>CFBundleDisplayName</key><string>$title</string>
	<key>CFBundleIdentifier</key><string>${PACKAGE_ID:-io.github.vnstrawhat.$bin}</string>
	<key>CFBundleExecutable</key><string>$bin</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
	<key>CFBundleShortVersionString</key><string>$version</string>
	<key>CFBundleVersion</key><string>$version</string>
	<key>LSMinimumSystemVersion</key><string>11.0</string>
	<key>NSHighResolutionCapable</key><true/>
	<key>NSPrincipalClass</key><string>NSApplication</string>
	$icon_key
	$types
</dict>
</plist>
EOF
plutil -lint "$app/Contents/Info.plist" > /dev/null
codesign --force --sign - "$app"
codesign --verify "$app"
echo "package_mac: $app ($(du -sk "$app" | cut -f1) KB${icon_key:+, with its icon})"
