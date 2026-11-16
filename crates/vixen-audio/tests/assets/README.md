These are generated 440 Hz tones, not user media. `tone.mp2` verifies the
advertised MPEG Layer II path and that MP3-specific delay trimming is not applied.
The other files exercise advertised formats rejected by the primary decoder.

Generated with local FFmpeg 9.0.2:

```sh
ffmpeg -nostdin -v error -y -f lavfi \
  -i sine=frequency=440:sample_rate=44100:duration=0.25 -ac 2 \
  -c:a mp2 -f mp2 tone.mp2
```

The same source options generate the fallback cases with these output arguments:

| File | Output arguments |
|---|---|
| `tone.wma` | `-c:a wmav2 -f asf tone.wma` |
| `tone.avi` | `-c:a pcm_s16le -f avi tone.avi` |
| `tone.m2ts` | `-c:a aac -f mpegts tone.m2ts` |
| `tone.ac3` | `-c:a ac3 -f ac3 tone.ac3` |
| `tone.ec3` | `-c:a eac3 -f eac3 tone.ec3` |
| `tone.adpcm.wav` | `-c:a adpcm_ima_wav -f wav tone.adpcm.wav` |

`tone.mono.wma` uses the same command with `sample_rate=22050`, `-ac 1`,
and `-c:a wmav2 -f asf`, exercising preservation of a different source rate and
channel count.

Run `scripts/build_media_tools.sh` before the fallback integration test. The
pinned source archive is fetched explicitly once with `--fetch`; ordinary builds
and tests do not use the network.
