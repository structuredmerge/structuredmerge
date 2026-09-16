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

## Cooperative cancellation

Create an opaque token with `create_operation_control()`, then pass it to
`parse_sources_controlled`, `merge_yaml_mapping_controlled`, or
`merge_python_declarations_controlled`. Call `cancel()` from a runtime thread;
`is_cancelled()` reports its one-way state. Repeated cancellation is harmless.
A cancelled token stays cancelled: create a new token for an independent run.
Sharing a token intentionally cancels every operation using it. Existing entry
points create a fresh, inaccessible token and retain their previous signatures.

The control wraps a shared Rust atomic flag, not serialized request data, a
process-wide ID registry, or a host callback. The explicit factory preserves its
identity across generated facade calls; it is not a defaultable options DTO.
The generated wrappers retain it through shared ownership without holding a
mutex over the operation. Installed tests cancel on one runtime thread while
another is paused inside input parsing or output verification, then release the
callback and require `execution.cancelled` without a successful result.

Python explicitly opts `OperationControl` into Alef's `send_sync_types`. The
generated frozen pyclass requires Rust `Send + Sync`; all other opaque handles
remain thread-confined by default. This requires the local Alef correction and
does not establish free-threaded Python or subinterpreter support. Without the
opt-in, the installed cross-thread test correctly fails on PyO3's unsendable
guard; do not weaken that test or bypass the guard.

Like deadlines, cancellation is observed at Rust checkpoints, not by forcibly
interrupting native code. A callback fault can be classified before the next
checkpoint. Cancellation does not shut down the registry or unregister a parser.

Run `workspace-scripts/check_core_ruby_artifact.rb` through the Ruby package's
bundle and `workspace-scripts/check_core_python_artifact.py WHEEL` to execute these
checks outside the checkout against installed packages. The existing native
syntax/error, provider exception, typed transport, and preservation tests remain
part of those same suites.

Still unproven: arbitrary foreign-thread entry, broad concurrent stress and
uncontrolled replacement races, interpreter/VM shutdown, subinterpreters, non-MRI
Ruby, cancellation under arbitrary foreign-thread execution and shutdown, and the full
supported runtime/platform matrix. Reentrant removal on a callback thread is not
a substitute for those independent requirements. No lifecycle claim here
authorizes package publication or default-provider approval.
