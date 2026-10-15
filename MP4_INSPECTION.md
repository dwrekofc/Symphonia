# Read-only MP4 inspection facade

MPL-2.0, existing notices and licenses retained. This adds a public associated
IsoMp4Reader::inspect function over the existing AtomIterator/MoovAtom readers.
It exposes neutral movie/track/handler/clock/edit/sample-table scalar metadata.
It introduces no parser, DSP, reader-state or normal decoding behavior changes.
All files other than the new inspection module, its export and this document
remain byte-for-byte at c622fad4b3ca7eddd6b7fdb1ae256dde3a21fb22.

The facade intentionally reports unsupported edits and fragmented movie state;
reporting metadata does not declare that the decoder interprets every profile.
Callers must separately establish decoded timing agreement and supported edits.
Ten real AAC-LC fixtures (8/32/44.1/48 kHz, short packets, moov ordering,
nondefault movie clock and absent edits) are compared against independent packet
metadata. No original executable, private audio or project-specific policy is
included in this fork.
