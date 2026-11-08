#!/usr/bin/env bash
# Release build of a Quadra app for Windows: MSVC -prod, GUI subsystem, and a resource file
# with the manifest (per-monitor DPI v2, UTF-8 code page, Common Controls v6, Windows 10+),
# version info and the app icon.
#
#   tools/package.sh <app dir> <exe name> "<product name>" <version a.b.c> [icon.ico]
#   tools/package.sh apps/notes quadra-notes "Quadra Notes" 1.0.0
#
# Output: dist/<exe name>/<exe name>.exe. Intermediate files go to .tmp/package/.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR=$1 EXE=$2 PRODUCT=$3 VERSION=$4 ICON=${5:-}
WORK="$ROOT/.tmp/package/$EXE"
OUT="$ROOT/dist/$EXE"
mkdir -p "$WORK" "$OUT"

KITS="/c/Program Files (x86)/Windows Kits/10/bin"
RC="$(ls -d "$KITS"/10.*/x64/rc.exe 2>/dev/null | sort -V | tail -1)"
[ -n "$RC" ] || { echo "package.sh: rc.exe not found under $KITS" >&2; exit 1; }

if [ -z "$ICON" ]; then
	ICON="$WORK/app.ico"
	python "$ROOT/tools/make_ico.py" "$ICON"
fi
[ "$ICON" = "$WORK/app.ico" ] || cp "$ICON" "$WORK/app.ico"

IFS=. read -r MAJ MIN PAT <<<"$VERSION"
cat >"$WORK/app.manifest" <<EOF
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity type="win32" name="quadra.$EXE" version="$MAJ.$MIN.$PAT.0"/>
  <dependency><dependentAssembly>
    <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0"
      processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
  </dependentAssembly></dependency>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1"><application>
    <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
  </application></compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3"><windowsSettings>
    <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
    <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    <activeCodePage xmlns="http://schemas.microsoft.com/SMI/2019/WindowsSettings">UTF-8</activeCodePage>
  </windowsSettings></application>
</assembly>
EOF

cat >"$WORK/app.rc" <<EOF
#pragma code_page(65001)
1 24 "app.manifest"
1 ICON "app.ico"
1 VERSIONINFO
FILEVERSION $MAJ,$MIN,$PAT,0
PRODUCTVERSION $MAJ,$MIN,$PAT,0
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "FileDescription", "$PRODUCT"
      VALUE "FileVersion", "$VERSION"
      VALUE "ProductName", "$PRODUCT"
      VALUE "ProductVersion", "$VERSION"
      VALUE "OriginalFilename", "$EXE.exe"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
EOF

(cd "$WORK" && "$RC" //nologo //fo app.res app.rc)
"$ROOT/tools/v.sh" -cc msvc -prod -subsystem windows -cflags "$(cygpath -w "$WORK/app.res")" \
	-o "$OUT/$EXE.exe" "$APP_DIR"
rm -f "$OUT/$EXE.exp" "$OUT/$EXE.lib" # export stubs nobody links against
echo "packaged $OUT/$EXE.exe"
