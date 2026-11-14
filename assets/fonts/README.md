# Chinese font assets

The application embeds a Chinese-only subset of Source Han Serif TC VF. It
contains simplified, Taiwan traditional and Hong Kong traditional locale
features in one variable font. `src/fonts.rs` is the single registration point
for the font files, family stack and language.

## Files

- `ZhijiChineseSerif-VF.otf` is a renamed subset of Source Han Serif TC VF
  2.003. The subset keeps CJK Unified Ideographs, Extension A, compatibility
  ideographs, CJK punctuation, ASCII, Latin-1, general punctuation and the
  `ZHS`, `ZHT`, `ZHH` language systems with their `locl` variants. It is
  variable across the original `wght` axis.
- `ZhijiHanSupplement.ttf` is a renamed subset of Hanazono Mincho B (20170904)
  with four Extension B characters retained from the former location catalog:
  U+20BCF, U+20BEB, U+25464 and U+26B99. It is a small fallback for annotations
  that use those rare characters.

The application does not bundle dedicated fallback fonts for Korean, Arabic,
Thai, Bengali, Georgian or Ethiopic. Text outside the Chinese repertoire falls
back to an installed system font or renders as a missing glyph.

## Language

The application defaults to `zh-Hans`. Interface and chart copy have independent
Simplified/Traditional settings; the interface setting also selects the shared
window font language, so chart text uses the same regional glyph form as the
surrounding interface. `src/i18n.rs` converts each text scope with OpenCC
dictionaries. User-entered annotation text, file names and paths are not
converted.

## Regeneration

Generate the subset in a separate, empty output directory using HarfBuzz
`hb-subset`:

```sh
mise exec -- node tools/fonts/generate-subsets.ts tc /path/to/SourceHanSerifTC-VF.otf --output /path/to/generated/tc
```

The source must have SHA-256
`8b71377f002adbe22387a780b0a09b5e263e9404196df5411c0ff5d682dd2703`.
The generator refuses to overwrite existing files. Generated binaries can
differ from the checked-in file; review names, Unicode coverage, the
`ZHS`/`ZHT`/`ZHH` language systems, `locl` output, licenses and visual
regressions before replacing the asset.

The Han supplement can be regenerated separately from the Hanazono source:

```sh
mise exec -- node tools/fonts/generate-subsets.ts supplemental /path/to/HanaMinB.ttf --output /path/to/generated/supplemental
```

Its source SHA-256 is
`829543436cdf7b4c6730e2af5806fa406f8ddf0d4b656e0cd49c58d4db1c8d3c`.
The source is available from the
[Hanazono source package](https://sources.debian.org/src/fonts-hanazono/20170904-2.1/).

## Licensing

Source Han Serif is © 2017–2024 Adobe, with Reserved Font Name “Source”. The
derivative uses a different family name and remains under the SIL Open Font
License 1.1; see [`OFL.txt`](OFL.txt). Hanazono Mincho B is dual-licensed under
the Hanazono Font License and the SIL Open Font License; see
[`LICENSE-Hanazono.txt`](LICENSE-Hanazono.txt).

Review font changes with the desktop visual tests, including simplified and
traditional samples. Unicode coverage alone does not establish correct shaping
or rendering.
