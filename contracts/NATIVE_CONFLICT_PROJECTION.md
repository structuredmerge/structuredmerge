# Executed owner decisions to portable conflicts

The shared Rust owner merger now retains `OwnerMergeClassification` alongside
its result and byte segments. It records all three whole-source comparisons,
any whole-source selection, and ordered per-owner decisions and alternatives.
Decision IDs use deterministic owner traversal order (base, then unseen ours,
then unseen theirs), not hash iteration or runtime object identity.

The same classifier branch selects/deletes an owner or creates its conflict
and records the typed reason. Edit/edit, delete/modify and add/add are distinct
facts even though historical conflict consumers still receive their existing
generic category. No second semantic classifier was added in a host adapter.
Input-validation failures have no classification record. A later rendering
failure retains classification facts but not successful output byte segments.

`project_native_merge_conflicts` projects these executed decisions into Slice
1028 diagnostics/conflicts and a decision evidence catalog. It verifies input
source correlation, source comparison claims, decision/conflict association,
alternative order and exact source ranges, and computes digests and diagnostic
points from verified bytes. Portable category/code selection uses the typed
decision, never the old message or category. The selected provider identity
must be supplied by the trusted executor and agree with an explicit request.

Projection is deliberately limited to an actual conflicted merge3 execution.
It does not manufacture clean results, authorize resolutions, render markers,
or infer absent alternatives from text. Localization honestly reports the
known owner region rather than claiming an isolated conflicting token. Original
input parse evidence remains on `NativeMergeExecution`; the common dispatcher
must carry that evidence into its eventual complete result projection.

The native merge pipeline also rejects a family analyzer that changes the
validated input bytes or supplies invalid owner ranges before classification.
This matches the existing native diff protection.

## Verification and open integration

Core tests run the actual Rust owner classifier on explicit owner facts, then
project and validate its conflicts through the existing common operation
result. They cover all three conflict kinds, both delete/modify directions,
repeatability, source/provider mismatch, missing classification and whole-source
base comparisons. These are kernel tests, not a native-parser binding gate.
The existing real Ruby/Psych process suite separately proves classification
evidence for all three conflict kinds and stale-analysis rejection.

Common operation dispatch and installed binding integration remain open, as do
complete clean-result, parse/analysis-failure and render-failure projection.
This projector does not convert an arbitrary legacy conflict into invented
evidence. End-to-end native-runtime portable-result conformance, complete
policy/registry negotiation and golden-master authority gates are not claimed.
