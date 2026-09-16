# Typed template reports

`structuredmerge-core` exports three typed operations backed directly by
`ast-template`, with no dependency on the prototype facade:

- `report_template_options(TemplateSessionOptions)` checks configuration and
  returns `TemplateSessionRequestReport`, without filesystem access.
- `report_template_profile(TemplateProfileRequest)` resolves named profiles using
  the owning library's existing defaults and override rules, without filesystem
  access. A report with mode `apply` is still only a report.
- `plan_template_directory(TemplateSessionOptions)` reads explicitly supplied
  local roots and returns `TemplateSessionPlanReport`. It rejects modes other
  than `plan`; it does not apply the plan or write destination files.

Paths cross the binding boundary as UTF-8 strings. Options, token configuration,
strategy overrides, profiles, diagnostics, plan entries, previews and summaries
are typed DTOs; there is no opaque JSON request/response operation. Runtime
adapters may project these objects back to historical report shapes.

Report semantics remain in ast-template/ast-merge. In particular, profile
resolution inherits non-default profile values when the override contains the
historical default; an empty replacements map inherits rather than clears.
`allowed_families` is retained in options/profile reports but does not constrain
the legacy directory-plan primitive. It is not an access-control boundary.

The directory planner is a local filesystem API, not an untrusted-input sandbox.
Callers must authorize roots. It currently inherits the owner's behavior for
missing roots (empty plans), path traversal/symlinks, and token processing; it
does not provide byte budgets, cancellation, transactionality or snapshot
isolation. No filesystem apply API or default-authority approval is added.

Python callers should construct `DirectorySessionMode` and `TemplateStrategy`
enum objects (for example `.PLAN` and `.RAW_COPY`). Native constructors currently
reject string alternatives despite the generator's broader stub annotations;
that generator mismatch remains open. `TemplateDestinationContext` is explicitly
reexported as its native class so public input constructors have one identity.

Verification includes exact serialization equivalence to owning options/profile
reports, shared Slice 362 directory-plan golden output, and unchanged fixture
bytes. Installed Ruby/Python tests construct nested DTOs and profile maps, check
diagnostics/inheritance, reject apply mode, and verify a Unicode/CRLF preview
without destination writes. These tests do not complete the Ruby ast-template
consumer migration, template apply parity, or broader release gates.
