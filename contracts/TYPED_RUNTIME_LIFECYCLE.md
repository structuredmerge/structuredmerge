# Typed parser callback lifecycle evidence

The installed-artifact suites exercise the generated `structuredmerge-core`
Ruby and Python boundaries. These checks establish only the behaviors below,
not general runtime/threading safety.

- Twelve register/GC/merge/unregister cycles in each runtime use weak references
  to establish that the registry retains a callback after the caller drops its
  strong reference and releases it after removal. Ruby also requests compaction.
  Each cycle executes a real native-parser merge, then checks explicit parser
  selection fails after removal.
- A provider unregisters itself inside `parse_batch`, then collects garbage and
  completes its batch. The in-flight parse succeeds using its retained snapshot;
  a subsequent parse fails selection. Registering a new provider with the same
  ID routes subsequent calls to the new instance, not the removed one.
- Two runtime-created worker threads enter native callbacks and wait behind a
  bounded barrier. The test requires both callbacks to arrive before either is
  released, so sequential execution cannot satisfy it. While both calls are
  paused, the controlling runtime thread unregisters the provider and registers
  a new instance with the same ID. Pending batches finish on the original
  instance and retain their distinct source checksums; the next call reaches
  only the replacement. This tests a controlled unregister/register transition,
  not an atomic replacement API.

Ruby's generated dispatcher exits asynchronously after the last sender drops.
The release check permits bounded thread scheduling and full collections before
checking the weak reference. This is not a shutdown deadline or a latency claim.
Python checks release after collection while attached to its running interpreter.

## Cooperative deadlines

`ParseLimits.timeout_millis` is an optional per-operation monotonic budget shared
by parser selection, input parsing, Rust merge processing, and output verification.
Omission/`None`/`nil` means no deadline; zero expires before parser dispatch.
Existing source/node/diagnostic limits still apply independently. An unrepresentable
clock deadline is rejected as `request.invalid`, never treated as unlimited.

Installed tests cover zero-budget parse/merge calls, successful native input
results returned after expiry, and successful output-verification parses returned
after expiry. Late successful results are discarded with
`execution.deadline_exceeded`, rather than exposing a clean result. A callback
failure may retain its provider-failure classification before the next deadline
checkpoint; deadline expiry is not a universal error-precedence override.

The budget is cooperative, not preemptive. A blocked native callback must return
before Rust can observe expiry; this API does not guarantee a wall-clock return
bound, interrupt native code, or expose an explicit cancellation handle. No
deadline is reset between input parsing and output verification.

Run `workspace-scripts/check_core_ruby_artifact.rb` through the Ruby package's
bundle and `workspace-scripts/check_core_python_artifact.py WHEEL` to execute these
checks outside the checkout against installed packages. The existing native
syntax/error, provider exception, typed transport, and preservation tests remain
part of those same suites.

Still unproven: arbitrary foreign-thread entry, broad concurrent stress and
uncontrolled replacement races, interpreter/VM shutdown, subinterpreters, non-MRI
Ruby, explicit cancellation and its late results across the generated boundary, and the full
supported runtime/platform matrix. Reentrant removal on a callback thread is not
a substitute for those independent requirements. No lifecycle claim here
authorizes package publication or default-provider approval.
