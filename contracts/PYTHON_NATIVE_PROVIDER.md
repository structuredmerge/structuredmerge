# Independently installed LibCST provider

The Python native-layer repository (`structuredmerge/structuredmerge-python`,
local sibling `../python`) now packages `structuredmerge-libcst` 0.1.0 separately
from this generated binding. It pins core 0.2.0 and LibCST 1.9.0; those pins are
tested pre-publication compatibility, not evidence of released dependencies.
The provider contains native syntax projection only. Rust continues to own
analysis, diff, matching, conflicts and rendering for the explicit
`kernel.python.native_declarations.v1` profile.

The adapter is stateless and explicitly registered. Descriptor package identity
names the adapter while parser identity names LibCST. It accepts original UTF-8
with optional BOM and LF/CRLF; bare CR and other encodings fail closed. It does
not advertise comment/token streams or a complete normalized Python CST.
Native extensions are opt-in. Syntax diagnostics do not copy private source.

`workspace-scripts/check_core_python_artifact.py CORE_WHEEL --provider-wheel
PROVIDER_WHEEL` installs both artifacts without sibling paths. Its test helper
only adds counters and received-batch observations to the installed host; the
retained conformance-only projector is not used by those callbacks. The default
mode still exercises that projector for independent regression evidence.
Reports distinguish these modes and record the provider artifact/digest.

On Linux CPython 3.14.2, all 55 boundary, 113 generated e2e and 114 test-app tests
pass with the independent provider. See `tmp/libcst-installed-provider.log` and
`tmp/core-python-artifact-k0rz7qas/report.json`. Provider wheel SHA-256 is
`bfdeb8a42edad44a02cedfa4348050fb40087eca3537feb89a0a51952396957c`.
The native-layer's separate sdist-to-wheel gate passes nine installed tests and
six archive/cleanup tests. Both package licenses match the authoritative kernel
license texts. Temporary environments were cleaned; no binding rebuild occurred.

This closes the copied-provider dependency for this scoped Python path, not the
full native-layer or release gates. Hosted CI, minimum-runtime/platform evidence,
released-package installation, facade/CLI implementation, other native providers
and publication remain open. Alef fixes remain local-only, as instructed.
