# Empty ID3 compatibility patch for the replacement player

Base: Symphonia 0.5.5 published revision
6d533f26150953a882a6a111ebd13f0abf7129d5. License remains MPL-2.0.

The existing metadata reader tries to read a frame from a zero-sized body and
returns EOF before the next metadata/audio element can be probed. Symphonia
0.6.1 (ee35874b571a35a9a6e15d3bc9a3aaf8f11fbeee) accepts this case through
its broader structural-frame error handling. This patch tolerates only an
exactly zero declared body after any required extended header has been read.
All nonempty bodies, truncated/unsupported headers and invalid extended-header
errors keep the 0.5.5 behavior. No decoder, codec, DSP or core source changes.

An empty tag is a compatibility input, not an ID3 standards-conformance claim:
https://id3.org/id3v2.3.0 requires at least one frame. The entire input file
remains unchanged. New tests cover versions 2.2/2.3/2.4, unsynchronisation,
following-byte preservation, malformed/extended/header errors and a normal title.

Consumers patch symphonia-metadata and the unmodified symphonia-core 0.5.5 to
the same exact Git revision. This preserves one shared core type identity for
all 0.5 codecs. The 0.6 branch is not replaced. The metadata/core tests can run
from an exact two-member copied workspace when optional upstream player/check
workspace tool dependencies are unavailable offline; source/manifests stay exact.
