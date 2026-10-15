# AAC-LC MP4 presentation timing repair

MPL-2.0; all original notices and LICENSE remain. This extends the existing
owned duration fork without changing its ID3/MPEG repairs or AAC DSP.

Sources (exact revisions):
- pdeljanov/Symphonia PR 574, 633623f46beaef0de2df0d1f31cbe208da7a4e7c:
  single normal-rate, nonempty edit mapping and seek clock conversion.
- Included PR 573, 0057638be01027f64de17c32cbc3b54bcdbb7eb5:
  packet duration subtracts the computed trims, with trim end capped.
- AAC-LC buffer trimming from PR 473, 00043533d7ed02d679d71cfb7efcb3b76e83ed38.
  No HE-AAC, SBR or other DSP implementation is imported.

Local boundary adaptations, found with independently generated real media:
- Preserve a complete AAC decoded block when stts shortens its presentation
  duration, and express the remainder through the existing packet trim API.
  Apply only when track and decoded sample clocks agree.
- Rescale single edit duration to nearest sample, ties upward. An edit of
  5 ms at 44.1 kHz retains 221 samples. The initial upstream floor lost one.
  Independently checked against FFmpeg mov.c get_edit_list_entry and actual
  PCM output; no FFmpeg parser/codec code is imported.

Tests: four 3-second AAC-LC profiles at 8/32/44.1/48 kHz, mono/stereo, moov
before/after data, several file extensions; six additional 5 ms, fractional
sample duration, non-default movie-clock and absent-edit fixtures. Corrected
outputs in both versions are exact slices of unchanged raw decoded PCM and
match independent FFmpeg frame counts. Disabling trimming preserves raw PCM.
Existing core, metadata and MPEG reader unit tests are also run independently.

Limits: a single normal-rate edit only. Empty, dwell, multiple/reordered edits,
nonmatching media/sample clocks, HE-AAC, unsupported profiles and general
container admission remain unproved/unsupported. AAC seek decoder preroll and
original/player equivalence are not established by full-file PCM comparisons.
No original media, application or private user data is included in this fork.

0.5 backport uses the existing formats::util::trim_packet; delay is bounded to
its u32 domain and enable_gapless controls both mapping and trimming. The
unchanged pinned decoder rejects every profile except AAC-LC, no SBR, 1024
frames, at most stereo. Its fixed decoded block size is reused, not inferred
from the final shortened stts sample. Core and MPEG sources remain exact.

The seven retained MP3 duration-repair fixtures pass in both versions with
exactly unchanged decoded PCM (14 complete comparisons).
