#!/bin/bash
# Build the pinned, isolated media fallback. No system installation and no
# network unless --fetch is explicitly supplied; ordinary app builds stay offline.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION=9.0.2
SHA256=8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e
URL="https://ffmpeg.org/releases/ffmpeg-$VERSION.tar.xz"
WORK="$ROOT/target/media-tools"
ARCHIVE="$WORK/source/ffmpeg-$VERSION.tar.xz"
SOURCE="$WORK/source/ffmpeg-$VERSION"
BUILD="$WORK/build-arm64-26.0"
mkdir -p "$WORK/source" "$WORK/bin" "$WORK/provenance"
# Isolated app builds share this pinned runtime. Keep configure/build/staging
# serial even if debug and release bundles are built concurrently.
if [ "${VIXEN_MEDIA_BUILD_LOCKED:-}" != "$WORK" ]; then
    exec python3 - "$WORK/build.lock" "$ROOT/scripts/build_media_tools.sh" "$WORK" "$@" <<'PY'
import fcntl, os, subprocess, sys
with open(sys.argv[1], 'a') as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    env = os.environ.copy()
    env['VIXEN_MEDIA_BUILD_LOCKED'] = sys.argv[3]
    raise SystemExit(subprocess.run(['/bin/bash', sys.argv[2], *sys.argv[4:]], env=env).returncode)
PY
fi
if [ "${1:-}" = "--fetch" ] && [ ! -f "$ARCHIVE" ]; then
    /usr/bin/curl --fail --location --proto '=https' --tlsv1.2 "$URL" -o "$ARCHIVE.part"
    ACTUAL="$(/usr/bin/shasum -a 256 "$ARCHIVE.part" | awk '{print $1}')"
    [ "$ACTUAL" = "$SHA256" ] || { echo "FFmpeg archive hash mismatch" >&2; exit 1; }
    mv "$ARCHIVE.part" "$ARCHIVE"
fi
[ -f "$ARCHIVE" ] || { echo "Missing pinned FFmpeg source; run scripts/build_media_tools.sh --fetch once." >&2; exit 1; }
ACTUAL="$(/usr/bin/shasum -a 256 "$ARCHIVE" | awk '{print $1}')"
[ "$ACTUAL" = "$SHA256" ] || { echo "FFmpeg archive hash mismatch" >&2; exit 1; }
if [ ! -f "$SOURCE/configure" ]; then
    /usr/bin/tar -xf "$ARCHIVE" -C "$WORK/source"
fi
SDK="$(/usr/bin/xcrun --sdk macosx --show-sdk-path)"
CONFIG=(
    --prefix=/opt/vixen-media-tools --cc=/usr/bin/clang --arch=arm64 --target-os=darwin
    --sysroot="$SDK" --extra-cflags=-mmacosx-version-min=26.0
    --extra-ldflags=-mmacosx-version-min=26.0
    --disable-shared --enable-static --disable-autodetect --disable-network
    --disable-gpl --disable-nonfree --disable-version3 --disable-doc --disable-debug
    --enable-small --disable-everything --disable-programs --enable-ffmpeg --enable-ffprobe
    --enable-avcodec --enable-avformat --enable-avfilter --enable-swresample --enable-swscale
    --disable-avdevice --enable-pthreads --enable-decoders --enable-parsers
    --enable-videotoolbox --enable-hwaccels --enable-zlib --enable-bzlib
    --enable-protocol=file,pipe
    --enable-demuxer=avi,asf,aac,ac3,eac3,aiff,flac,mov,mp3,mpegts,ogg,wav
    --enable-muxer=rawvideo,pcm_f32le --enable-encoder=rawvideo,pcm_f32le
    --enable-filter=scale,format,fps,aresample,aformat,anull,null,trim,atrim,setpts,asetpts,transpose,hflip,vflip,rotate
)
KEY="$( { printf '%s\n' "$SHA256" "${CONFIG[@]}"; /usr/bin/clang --version; } | /usr/bin/shasum -a 256 | awk '{print $1}')"
if [ ! -f "$BUILD/config.key" ] || [ "$(cat "$BUILD/config.key")" != "$KEY" ]; then
    rm -rf "$BUILD"
    mkdir -p "$BUILD"
    # FFmpeg supports a relative ./src tree, which also works when this checkout
    # has spaces in its path. Do not patch the verified upstream source.
    ln -s "$SOURCE" "$BUILD/src"
    ( cd "$BUILD" && ./src/configure "${CONFIG[@]}" )
    printf '%s\n' "$KEY" > "$BUILD/config.key"
    printf '%s\n' "${CONFIG[@]}" > "$WORK/provenance/configure-arguments.txt"
fi
JOBS="${VIXEN_MEDIA_JOBS:-$(sysctl -n hw.ncpu)}"
( cd "$BUILD" && /usr/bin/make -j "$JOBS" ffmpeg ffprobe )
for MEDIA_TOOL in ffmpeg ffprobe; do
    # Do not overwrite an inode another decoder process may still be executing.
    cp "$BUILD/$MEDIA_TOOL" "$WORK/bin/.$MEDIA_TOOL.part"
    /usr/bin/codesign --force --sign - --identifier "org.vixen.media.$MEDIA_TOOL" \
        --timestamp=none "$WORK/bin/.$MEDIA_TOOL.part" >/dev/null 2>&1
    mv -f "$WORK/bin/.$MEDIA_TOOL.part" "$WORK/bin/$MEDIA_TOOL"
done
cp "$SOURCE/COPYING.LGPLv2.1" "$SOURCE/LICENSE.md" "$WORK/provenance/"
printf '%s\n%s\n' "$URL" "$SHA256" > "$WORK/provenance/source.txt"
"$WORK/bin/ffmpeg" -version > "$WORK/provenance/ffmpeg-version.txt" 2>&1
"$WORK/bin/ffmpeg" -L > "$WORK/provenance/ffmpeg-license.txt" 2>&1
python3 - "$WORK" "$VERSION" "$URL" "$SHA256" <<'PY'
import hashlib, json, pathlib, subprocess, sys
root = pathlib.Path(sys.argv[1])
files = []
for name in ['ffmpeg', 'ffprobe']:
    path = root / 'bin' / name
    # Relative names keep this checkout's absolute path out of the shipped manifest.
    dependencies = subprocess.check_output(['/usr/bin/otool', '-L', name], text=True, cwd=root / 'bin')
    for line in dependencies.splitlines()[1:]:
        library = line.strip().split(' (compatibility version')[0]
        if not library.startswith(('/usr/lib/', '/System/Library/')):
            raise SystemExit(f'Non-system media dependency: {library}')
    files.append({'name': name, 'bytes': path.stat().st_size,
                  'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                  'dependencies': dependencies,
                  'loadCommands': subprocess.check_output(['/usr/bin/otool', '-l', name], text=True, cwd=root / 'bin')})
manifest = {'version': sys.argv[2], 'source': sys.argv[3], 'sourceSha256': sys.argv[4],
            'minimumMacOS': '26.0', 'architecture': 'arm64', 'files': files}
(root / 'provenance/manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
PY
echo "Built private media tools in $WORK/bin"
