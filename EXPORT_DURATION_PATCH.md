# Preserve complete untagged MPEG audio

Base: published Symphonia 0.6.1 ee35874b571a35a9a6e15d3bc9a3aaf8f11fbeee.
MPL-2.0. Only demuxer provenance for estimated frame counts changes.

The MPEG reader estimates untagged duration from initial frame sizes and
incorrectly uses that estimate to signal exact tail trimming. A private flag
keeps the estimate for coarse seeking/duration but removes it as a trim limit.
Declared Xing/Info/VBRI counts, encoder delay/padding, packet builder/core and
all codec/DSP code remain unchanged. Consumers patch this MPEG package and the
unmodified 0.6.1 core to the same Git identity to preserve one shared type.

Tests cover MPEG1/2/2.5 underestimates, exact/overestimates, declared limits,
encoder delay/padding, absent counts, short and truncated streams. They measure
actual codec output: the existing 0.6.1 packet builder's duration field can
retain full block duration even when trim fields request discarding samples.
That separate packet-duration discrepancy is not changed by this repair.
