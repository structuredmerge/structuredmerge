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
The native-layer's separate sdist-to-wheel gate now passes three core-only
facade tests, thirteen facade/provider tests and eleven archive/cleanup tests.
Both package licenses match the authoritative kernel
license texts. Temporary environments were cleaned; no binding rebuild occurred.

On Linux CPython 3.10.19, the same core and provider wheel digests also pass all
55 boundary, 113 generated e2e and 114 test-app tests. Evidence:
`tmp/python310-installed-provider.log` and
`tmp/core-python-artifact-xptfulby/report.json`. The Python native layer separately
builds both sdists/wheels and runs its core-only and LibCST-extra installed gates
on that minimum runtime; see its `VERIFICATION.md`. This proves these two local
CPython/runtime combinations, not hosted CI or the full supported-platform matrix.

This closes the copied-provider dependency for this scoped Python path, not the
full native-layer or release gates. Hosted CI, broader platform/runtime evidence,
released-package installation, CLI implementation, other native providers
and publication remain open. Alef fixes remain local-only, as instructed.

## Registry-guard integration refresh (2026-09-18)

The independent facade exposes the generated selection-report and guarded
workflow calls as direct aliases. Source-built installed native-layer artifacts
pass three core-only and sixteen native-enabled tests on Linux CPython 3.10.19
and 3.14.2. New cases exercise all four operations with Rust ownership, stale
parser retirement/re-registration with identical descriptor bytes, explicit
re-observation and cancellation before native callbacks. These do not grant
default authority, authenticated preflight or a loaded-asset lease.

The current core wheel SHA-256 is
`faccc73f37bad6b6f6b4c222421697dafc23590383444454069bc8aa74ceef53`.
Its separate full installed-provider gate passes 60 boundary, 113 generated e2e
and 114 app tests on CPython 3.14.2, using provider wheel SHA-256
`0959da7221df74e27a8b2118bef7dadf9e8591d1359fc2d0e9eb3e17b09babe7`.
Report: `tmp/core-python-artifact-t2ofaeo6/report.json`; log:
`tmp/guarded-independent-provider.log`. The native-layer `VERIFICATION.md`
records both facade artifact sets. The full current kernel suite was not rerun
on 3.10 here. Temporary environments were removed; no compiler build, package
publication or hosted-CI success is claimed.

## Fresh-cache verification

The Python cached-only JSON parser example, like its Ruby counterpart, previously
depended on another test preparing the grammar. Running it alone against an empty
cache reproduced `selection.no_parser`. Its setup now explicitly parses through
a separate acquisition-enabled provider and unregisters that provider before
testing cached-only registration. Registration alone is lazy. The unchanged
installed core wheel fails the original isolated example and passes the corrected
one; evidence is in `tmp/python-cold-cache-report.json`.

The complete independent-provider gate then passed 60 boundary / 113 generated /
114 app tests on CPython 3.14.2 with a fresh grammar cache and isolated dependency
installation. Artifact hashes are unchanged from the registry-guard refresh
above. Report: `tmp/core-python-artifact-3ox1nrdf/report.json`; log:
`tmp/python-provider-cold-cache.log`. Explicit acquisition-enabled tests may use
the network: this is not an offline gate. Disposable environments, consumers and
grammar caches were removed; no compiler build, publication, hosted CI or broader
runtime/platform result is implied.
