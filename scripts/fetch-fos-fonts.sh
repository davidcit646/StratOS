#!/usr/bin/env bash
# Download a large curated set of free/open-source fonts into fonts/stratos/
# for inclusion in the phase7 rootfs (see build-all-and-run.sh).
#
# Licenses are upstream (SIL OFL 1.1, Apache 2.0, UFL, Bitstream Vera / DejaVu, etc.).
# Run from repo root or any cwd; requires curl, tar, bzip2, unzip.
#
# Usage: ./scripts/fetch-fos-fonts.sh [--force]
#   --force  re-download even if the output file already exists

set -u
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$REPO_ROOT/fonts/stratos"
FORCE=0
if [ "${1:-}" = "--force" ]; then
    FORCE=1
fi

mkdir -p "$DEST"
if [ "$FORCE" -eq 1 ]; then
    rm -f "$DEST/.dejavu-2.37.extracted"
fi
ERRORS=0

dl() {
    local url="$1" out="$2"
    local path="$DEST/$out"
    if [ "$FORCE" -eq 0 ] && [ -f "$path" ] && [ -s "$path" ]; then
        echo "  skip (exists): $out"
        return 0
    fi
    echo "  get $out"
    if ! curl -fsSL --connect-timeout 45 --retry 2 --retry-delay 2 -o "$path.part" "$url"; then
        echo "  ** failed: $out" >&2
        rm -f "$path.part"
        ERRORS=$((ERRORS + 1))
        return 1
    fi
    mv -f "$path.part" "$path"
}

echo "[fetch-fos-fonts] -> $DEST"

# --- Single-file downloads (tab-separated URL then basename) ---
while IFS=$'\t' read -r url name; do
    [ -z "${url:-}" ] && continue
    case "$url" in \#*) continue ;; esac
    dl "$url" "$name"
done <<'EOF'
https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/NotoSans/NotoSans-Regular.ttf	NotoSans-Regular.ttf
https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/NotoSans/NotoSans-Bold.ttf	NotoSans-Bold.ttf
https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/NotoSans/NotoSans-Italic.ttf	NotoSans-Italic.ttf
https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/NotoSans/NotoSans-BoldItalic.ttf	NotoSans-BoldItalic.ttf
https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/NotoSerif/NotoSerif-Regular.ttf	NotoSerif-Regular.ttf
https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/NotoSerif/NotoSerif-Bold.ttf	NotoSerif-Bold.ttf
https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/NotoSerif/NotoSerif-Italic.ttf	NotoSerif-Italic.ttf
https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/NotoSansMono/NotoSansMono-Regular.ttf	NotoSansMono-Regular.ttf
https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/NotoSansMono/NotoSansMono-Bold.ttf	NotoSansMono-Bold.ttf
https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/NotoSansMono/NotoSansMono-Medium.ttf	NotoSansMono-Medium.ttf
https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/NotoSansDisplay/NotoSansDisplay-Regular.ttf	NotoSansDisplay-Regular.ttf
https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/Arimo/Arimo-Regular.ttf	Arimo-Regular.ttf
https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/Arimo/Arimo-Bold.ttf	Arimo-Bold.ttf
https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/Tinos/Tinos-Regular.ttf	Tinos-Regular.ttf
https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/Tinos/Tinos-Bold.ttf	Tinos-Bold.ttf
https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/Cousine/Cousine-Regular.ttf	Cousine-Regular.ttf
https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/Cousine/Cousine-Bold.ttf	Cousine-Bold.ttf
https://raw.githubusercontent.com/googlefonts/roboto/main/src/hinted/Roboto-Regular.ttf	Roboto-Regular.ttf
https://raw.githubusercontent.com/googlefonts/roboto/main/src/hinted/Roboto-Bold.ttf	Roboto-Bold.ttf
https://raw.githubusercontent.com/googlefonts/opensans/main/fonts/ttf/OpenSans-Regular.ttf	OpenSans-Regular.ttf
https://raw.githubusercontent.com/googlefonts/opensans/main/fonts/ttf/OpenSans-Bold.ttf	OpenSans-Bold.ttf
https://raw.githubusercontent.com/googlefonts/fira/main/ttf/FiraSans-Regular.ttf	FiraSans-Regular.ttf
https://raw.githubusercontent.com/googlefonts/fira/main/ttf/FiraSans-Bold.ttf	FiraSans-Bold.ttf
https://raw.githubusercontent.com/googlefonts/fira/main/ttf/FiraMono-Regular.ttf	FiraMono-Regular.ttf
https://raw.githubusercontent.com/googlefonts/fira/main/ttf/FiraMono-Bold.ttf	FiraMono-Bold.ttf
https://raw.githubusercontent.com/googlefonts/Inconsolata/main/fonts/ttf/Inconsolata-Regular.ttf	Inconsolata-Regular.ttf
https://raw.githubusercontent.com/googlefonts/Inconsolata/main/fonts/ttf/Inconsolata-Bold.ttf	Inconsolata-Bold.ttf
https://raw.githubusercontent.com/adobe-fonts/source-sans/release/TTF/SourceSans3-Regular.ttf	SourceSans3-Regular.ttf
https://raw.githubusercontent.com/adobe-fonts/source-sans/release/TTF/SourceSans3-Bold.ttf	SourceSans3-Bold.ttf
https://raw.githubusercontent.com/adobe-fonts/source-sans/release/TTF/SourceSans3-It.ttf	SourceSans3-It.ttf
https://raw.githubusercontent.com/adobe-fonts/source-serif/release/TTF/SourceSerif4-Regular.ttf	SourceSerif4-Regular.ttf
https://raw.githubusercontent.com/adobe-fonts/source-serif/release/TTF/SourceSerif4-Bold.ttf	SourceSerif4-Bold.ttf
https://raw.githubusercontent.com/adobe-fonts/source-code-pro/release/TTF/SourceCodePro-Regular.ttf	SourceCodePro-Regular.ttf
https://raw.githubusercontent.com/adobe-fonts/source-code-pro/release/TTF/SourceCodePro-Bold.ttf	SourceCodePro-Bold.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/lato/Lato-Regular.ttf	Lato-Regular.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/lato/Lato-Bold.ttf	Lato-Bold.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/montserrat/Montserrat%5Bwght%5D.ttf	Montserrat-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/oswald/Oswald%5Bwght%5D.ttf	Oswald-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/raleway/Raleway%5Bwght%5D.ttf	Raleway-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/nunito/Nunito%5Bwght%5D.ttf	Nunito-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/merriweather/Merriweather%5Bopsz,wdth,wght%5D.ttf	Merriweather-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/playfairdisplay/PlayfairDisplay%5Bwght%5D.ttf	PlayfairDisplay-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/worksans/WorkSans%5Bwght%5D.ttf	WorkSans-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/dmsans/DMSans%5Bopsz,wght%5D.ttf	DMSans-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/karla/Karla%5Bwght%5D.ttf	Karla-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/rubik/Rubik%5Bwght%5D.ttf	Rubik-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/bitter/Bitter%5Bwght%5D.ttf	Bitter-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/ebgaramond/EBGaramond%5Bwght%5D.ttf	EBGaramond-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/crimsonpro/CrimsonPro%5Bwght%5D.ttf	CrimsonPro-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/quicksand/Quicksand%5Bwght%5D.ttf	Quicksand-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/manrope/Manrope%5Bwght%5D.ttf	Manrope-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/archivo/Archivo%5Bwdth,wght%5D.ttf	Archivo-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/lora/Lora%5Bwght%5D.ttf	Lora-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/firacode/FiraCode%5Bwght%5D.ttf	FiraCode-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/spacemono/SpaceMono-Regular.ttf	SpaceMono-Regular.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/spacemono/SpaceMono-Bold.ttf	SpaceMono-Bold.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/anonymouspro/AnonymousPro-Regular.ttf	AnonymousPro-Regular.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/anonymouspro/AnonymousPro-Bold.ttf	AnonymousPro-Bold.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/courierprime/CourierPrime-Regular.ttf	CourierPrime-Regular.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/courierprime/CourierPrime-Bold.ttf	CourierPrime-Bold.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/slabo27px/Slabo27px-Regular.ttf	Slabo27px-Regular.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/bebasneue/BebasNeue-Regular.ttf	BebasNeue-Regular.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/ptserif/PT_Serif-Web-Regular.ttf	PTSerif-Web-Regular.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/ptserif/PT_Serif-Web-Bold.ttf	PTSerif-Web-Bold.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/ptsans/PT_Sans-Web-Regular.ttf	PTSans-Web-Regular.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/ptsans/PT_Sans-Web-Bold.ttf	PTSans-Web-Bold.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/cantarell/Cantarell-Regular.ttf	Cantarell-Regular.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/cantarell/Cantarell-Bold.ttf	Cantarell-Bold.ttf
https://raw.githubusercontent.com/google/fonts/main/ufl/ubuntu/Ubuntu-Regular.ttf	Ubuntu-Regular.ttf
https://raw.githubusercontent.com/google/fonts/main/ufl/ubuntu/Ubuntu-Bold.ttf	Ubuntu-Bold.ttf
https://raw.githubusercontent.com/google/fonts/main/ufl/ubuntumono/UbuntuMono-Regular.ttf	UbuntuMono-Regular.ttf
https://raw.githubusercontent.com/google/fonts/main/ufl/ubuntumono/UbuntuMono-Bold.ttf	UbuntuMono-Bold.ttf
https://raw.githubusercontent.com/google/fonts/main/ufl/ubuntusans/UbuntuSans%5Bwdth,wght%5D.ttf	UbuntuSans-VF.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/poppins/Poppins-Regular.ttf	Poppins-Regular.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/poppins/Poppins-Bold.ttf	Poppins-Bold.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/poppins/Poppins-Medium.ttf	Poppins-Medium.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/spectral/Spectral-Regular.ttf	Spectral-Regular.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/spectral/Spectral-Bold.ttf	Spectral-Bold.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/zillaslab/ZillaSlab-Regular.ttf	ZillaSlab-Regular.ttf
https://raw.githubusercontent.com/google/fonts/main/ofl/zillaslab/ZillaSlab-Bold.ttf	ZillaSlab-Bold.ttf
EOF

# --- Archives: DejaVu (Bitstream Vera / DejaVu license) ---
fetch_dejavu() {
    local mark="$DEST/.dejavu-2.37.extracted"
    if [ "$FORCE" -eq 0 ] && [ -f "$mark" ]; then
        echo "  skip DejaVu bundle (marker present; use --force to refresh)"
        return 0
    fi
    local tmp
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' RETURN
    echo "  get DejaVu 2.37 tarball"
    if ! curl -fsSL --connect-timeout 45 --retry 2 \
        -o "$tmp/dv.tar.bz2" \
        "https://github.com/dejavu-fonts/dejavu-fonts/releases/download/version_2_37/dejavu-fonts-ttf-2.37.tar.bz2"; then
        echo "  ** failed DejaVu download" >&2
        ERRORS=$((ERRORS + 1))
        return 1
    fi
    tar -xjf "$tmp/dv.tar.bz2" -C "$tmp"
    for f in DejaVuSans.ttf DejaVuSans-Bold.ttf DejaVuSansMono.ttf DejaVuSansMono-Bold.ttf DejaVuSerif.ttf DejaVuSerif-Bold.ttf; do
        cp -f "$tmp/dejavu-fonts-ttf-2.37/ttf/$f" "$DEST/$f"
    done
    date -u +"%Y-%m-%dT%H:%M:%SZ" >"$mark"
}
fetch_dejavu

# --- Inter (OFL) variable fonts from official release zip ---
fetch_inter() {
    local want="$DEST/InterVariable.ttf"
    if [ "$FORCE" -eq 0 ] && [ -f "$want" ] && [ -s "$want" ]; then
        echo "  skip Inter (exists)"
        return 0
    fi
    local tmp
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' RETURN
    echo "  get Inter 4.1 release zip"
    if ! curl -fsSL --connect-timeout 60 --retry 2 \
        -o "$tmp/inter.zip" \
        "https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip"; then
        echo "  ** failed Inter download" >&2
        ERRORS=$((ERRORS + 1))
        return 1
    fi
    unzip -qo "$tmp/inter.zip" -d "$tmp/inter"
    cp -f "$tmp/inter/InterVariable.ttf" "$DEST/InterVariable.ttf"
    cp -f "$tmp/inter/InterVariable-Italic.ttf" "$DEST/InterVariable-Italic.ttf" 2>/dev/null || true
    cp -f "$tmp/inter/LICENSE.txt" "$DEST/Inter-LICENSE.txt" 2>/dev/null || true
}
fetch_inter

# --- JetBrains Mono (OFL / Apache dual; OFL in package) ---
fetch_jetbrains_mono() {
    local mark="$DEST/JetBrainsMono-Regular.ttf"
    if [ "$FORCE" -eq 0 ] && [ -f "$mark" ]; then
        echo "  skip JetBrains Mono (exists)"
        return 0
    fi
    local tmp
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' RETURN
    echo "  get JetBrainsMono 2.304 zip"
    if ! curl -fsSL --connect-timeout 60 --retry 2 \
        -o "$tmp/jb.zip" \
        "https://github.com/JetBrains/JetBrainsMono/releases/download/v2.304/JetBrainsMono-2.304.zip"; then
        echo "  ** failed JetBrains Mono download" >&2
        ERRORS=$((ERRORS + 1))
        return 1
    fi
    unzip -qo "$tmp/jb.zip" -d "$tmp/jb"
    cp -f "$tmp/jb/fonts/ttf/JetBrainsMono-Regular.ttf" "$DEST/JetBrainsMono-Regular.ttf"
    cp -f "$tmp/jb/fonts/ttf/JetBrainsMono-Bold.ttf" "$DEST/JetBrainsMono-Bold.ttf"
    cp -f "$tmp/jb/fonts/ttf/JetBrainsMono-Italic.ttf" "$DEST/JetBrainsMono-Italic.ttf" 2>/dev/null || true
    if [ -f "$tmp/jb/fonts/variable/JetBrainsMono[wght].ttf" ]; then
        cp -f "$tmp/jb/fonts/variable/JetBrainsMono[wght].ttf" "$DEST/JetBrainsMono-VF.ttf"
    fi
    cp -f "$tmp/jb/OFL.txt" "$DEST/JetBrainsMono-OFL.txt" 2>/dev/null || true
}
fetch_jetbrains_mono

# --- NOTICE for compliance / debugging ---
{
    echo "StratOS bundled open fonts (generated $(date -u +"%Y-%m-%dT%H:%M:%SZ"))"
    echo "Sources: googlefonts/noto-fonts, google/fonts (OFL/UFL), Adobe Source families,"
    echo "  DejaVu, rsms/inter, JetBrains/JetBrainsMono, plus upstream Google Fonts OFL tree."
    echo "Each family ships its own license (OFL.txt, LICENSE.txt, Apache-2.0, UFL, etc.) upstream."
    echo "Do not commit *.ttf here; run scripts/fetch-fos-fonts.sh before build-all-and-run.sh."
} >"$DEST/NOTICE.txt"

n=$(find "$DEST" -maxdepth 1 \( -name '*.ttf' -o -name '*.otf' -o -name '*.ttc' \) | wc -l)
echo "[fetch-fos-fonts] done: $n font files under fonts/stratos/ (errors=$ERRORS)"
exit $ERRORS
