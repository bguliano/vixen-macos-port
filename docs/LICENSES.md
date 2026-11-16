# Licences

This port is derived from [Vixen](https://github.com/VixenLights/Vixen). Keep these notices with any copy of it.

| Part | Licence | Where |
|---|---|---|
| Vixen itself, which everything here ports | Vixen's BSD-style licence: keep the copyright notice in source and binary forms | `upstream/Vixen/License.txt`, bundled in the app as `Resources/License.txt` |
| Beats and Bars analysis (`crates/vixen-analysis`), ported from QM Vamp Plugins and QM DSP | GPL-2.0-or-later | licence files in that crate |
| KissFFT, ported into `crates/vixen-analysis` | BSD-3-Clause | `kissfft-COPYING` in that crate |
| CMU Pronouncing Dictionary 0.07 (CMUdict), compiled into the lip sync text conversion | BSD-style: keep the copyright notice in source and binary forms | the notice at the top of `crates/vixen-session/assets/standard_dictionary`, copied from upstream's `LipSyncApp/Resources/standard_dictionary` |
| LiquidFun and Box2D, compiled into the Liquid effect | zlib-style | `crates/fx-pixel-physics/vendor/Box2D` |
| rust_decimal 1.43.0, exact 96-bit arithmetic for Curve f(x) compatibility | MIT | `app/Resources/Licenses/rust_decimal.txt`; [primary crate documentation](https://docs.rs/rust_decimal/1.43.0/rust_decimal/) |
| unicode-normalization 0.1.26 and tinyvec 1.13.3, canonical Unicode filename matching for sequence packages | unicode-normalization: (MIT OR Apache-2.0) AND Unicode-3.0; tinyvec: Zlib OR Apache-2.0 OR MIT | `app/Resources/Licenses/unicode-normalization.txt`; exact registry versions and unmodified upstream notices included |
| Nominal aliased stroke and Bezier arithmetic adapted from [emf-converter](https://github.com/ChristopherVR/emf-converter) | Apache-2.0 | `app/Resources/Licenses/emf-converter.txt` and `emf-converter-notice.txt`; Rust adaptation in `crates/vixen-effects/src/gfx_nominal.rs` |
| FFmpeg 9.0.2 and ffprobe, separate bundled media fallback executables | LGPL-2.1-or-later build; GPL/nonfree/version3 code and external codec libraries disabled | `app/Resources/Licenses/ffmpeg.txt`; exact source URL/hash and configure arguments in `Contents/Resources/MediaTools`, reproducible with `scripts/build_media_tools.sh` |

**The GPL part matters if this app is ever given to anyone else.** Because `vixen-analysis` is linked into the app, a copy someone else receives would have to come under the GPL as a whole, with its source. Upstream's Windows release is in the same position, since it embeds the same QM code. Keeping it for personal use has no such condition. Removing the Beats and Bars feature would remove the condition.

Upstream's graphical patching uses NShape, which is also GPL. That code was not ported: the Display Setup window draws its own.
