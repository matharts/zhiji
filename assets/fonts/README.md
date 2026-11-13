# Prototype font subset

License text files use LF line endings without trailing whitespace. Their
wording and copyright notices are unchanged; font binaries are unchanged.

The `Zhiji Prototype Serif` files are renamed, subsetted derivatives of Source
Han Serif CN. Regular, SemiBold and Bold come from the WOFF2 files in the
reference simulator and cover characters present in the current Rust source,
including nested UI modules and their menu labels.
Regeneration also retains the glyphs already present in the regular face so
existing text does not lose coverage when UI strings are removed.
Heavy is derived from the variable font at weight 900 and contains only A–D,
for the birth-transformation marks. These are not general-purpose fonts;
check glyph coverage when adding new text.

Generate replacement fonts in a separate, empty output directory using Node.js,
HarfBuzz `hb-subset` (with variable-font support) and `woff2_decompress`:

```sh
mise exec -- node tools/fonts.mjs prototype /path/to/ziwei-simulator/public/fonts /path/to/SourceHanSerifCN-VF.otf --output /path/to/generated/prototype
mise exec -- node tools/fonts.mjs location /path/to/SourceHanSerifCN-VF.otf --output /path/to/generated/location
mise exec -- node tools/fonts.mjs supplemental /path/to/NotoSerifKR.ttf /path/to/HanaMinB.ttf --output /path/to/generated/supplemental
```

The variable source and supplemental sources are checked against the SHA-256
values below. The prototype subsets retain existing regular-face coverage and
include characters in application and workspace Rust source. The tool refuses
to overwrite existing output files and accepts `HB_SUBSET` and
`WOFF2_DECOMPRESS` environment variables for custom binary paths. It was checked
with HarfBuzz 8.3.0. Generated binaries can differ from the checked-in files;
review names, glyph coverage, licenses, and visual regressions before replacing
assets and updating the manifest. Building and running the app uses the
checked-in fonts.

Original font copyright: © 2017–2024 Adobe (http://www.adobe.com/), with
Reserved Font Name “Source”. The modified font files use a different family
name. Both derivatives remain under the SIL Open Font License 1.1; see
[`OFL.txt`](OFL.txt).

`ZhijiLocationSerif-VF.otf` is a renamed, full-coverage Source Han Serif CN
2.003 variable font used as a fallback for user input and editable annotations.
The original prototype faces remain first in the stack, preserving existing
chart and interface glyphs. Its source file has SHA-256
`0926369670e4b18b63ae2642ac5b57c2e03343c2f3083c4ec98e8c0117edea4f`.
Its license is the same OFL linked above.

`NotoSerif-VF.ttf` provides Latin extended, Greek, Cyrillic and combining marks
for multilingual annotations. It is the unmodified Noto
Serif 2.015 variable font from
[`google/fonts`](https://github.com/google/fonts/tree/main/ofl/notoserif),
SHA-256 `4d8e6761424656867019081a1a01336f3cb086982682698714054fc33f782713`.
Copyright 2022 The Noto Project Authors; licensed under the SIL Open Font
License 1.1, see [`OFL-Noto.txt`](OFL-Noto.txt).

The remaining fallback faces cover multilingual user-entered and annotated
text. These unmodified variable fonts come from the corresponding directories
of [`google/fonts`](https://github.com/google/fonts/tree/main/ofl):

| File | Source directory | SHA-256 | License |
| --- | --- | --- | --- |
| `NotoNaskhArabic-VF.ttf` | `notonaskharabic` | `67b5a525a661b607971fbd3f96a81b89d3a768e74534fca84f18ac97e6fab72f` | [`OFL-Noto-Arabic.txt`](OFL-Noto-Arabic.txt) |
| `NotoSerifThai-VF.ttf` | `notoserifthai` | `34a7ad11647c845303aabdde639059806c56b84719e5d2ceb28eb038711bdf53` | [`OFL-Noto-Thai.txt`](OFL-Noto-Thai.txt) |
| `NotoSerifBengali-VF.ttf` | `notoserifbengali` | `083180a7b47cf3b101ed55a8724aa1b38869daa9baf23b7cf198f9d225365aba` | [`OFL-Noto-Bengali.txt`](OFL-Noto-Bengali.txt) |
| `NotoSerifGeorgian-VF.ttf` | `notoserifgeorgian` | `a13a90c44a612ab80f79032d3edbbc7f6757f0c6b1613dba1a92f38077cccc57` | [`OFL-Noto-Georgian.txt`](OFL-Noto-Georgian.txt) |
| `NotoSerifEthiopic-VF.ttf` | `notoserifethiopic` | `49d5daa6c3477a427f5d4f8a5ae9ea7f0a04ea7f865fc21f7e08fc4aed70d92a` | [`OFL-Noto-Ethiopic.txt`](OFL-Noto-Ethiopic.txt) |

`ZhijiKoreanSerif-VF.ttf` is a renamed, Hangul-only subset of Noto Serif KR;
its source SHA-256 is `11f8d5de6f1b79195efba3828aaa2ec95c1178f5ae976fb23c8d53250a9938f3`
and its license is [`OFL-Noto-Korean.txt`](OFL-Noto-Korean.txt).
`ZhijiHanSupplement.ttf` is a renamed subset of Hanazono Mincho B (20170904)
with four Extension B characters selected from the former location catalog
but absent from Source Han Serif. The source SHA-256 is
`829543436cdf7b4c6730e2af5806fa406f8ddf0d4b656e0cd49c58d4db1c8d3c`;
see [`LICENSE-Hanazono.txt`](LICENSE-Hanazono.txt). The sources can be obtained
from the Google Fonts `notoserifkr` directory and the
[Hanazono source package](https://sources.debian.org/src/fonts-hanazono/20170904-2.1/).
Use the `supplemental` command above to generate both subsets.

The location catalog and its coverage checker were removed with date entry.
The font stack remains available for existing and new annotations. Validate
font changes with the desktop visual tests, including multilingual text;
Unicode coverage alone does not establish correct shaping or rendering.
Characters outside the embedded fonts still depend on system-font fallback.
