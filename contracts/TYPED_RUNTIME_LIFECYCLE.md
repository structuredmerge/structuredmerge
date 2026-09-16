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

Ruby's generated dispatcher exits asynchronously after the last sender drops.
The release check permits bounded thread scheduling and full collections before
checking the weak reference. This is not a shutdown deadline or a latency claim.
Python checks release after collection while attached to its running interpreter.

Run `workspace-scripts/check_core_ruby_artifact.rb` through the Ruby package's
bundle and `workspace-scripts/check_core_python_artifact.py WHEEL` to execute these
checks outside the checkout against installed packages. The existing native
syntax/error, provider exception, typed transport, and preservation tests remain
part of those same suites.

Still unproven: concurrent worker execution, arbitrary foreign-thread entry,
in-flight replacement races, interpreter/VM shutdown, subinterpreters, non-MRI
Ruby, cancellation and late results across the generated boundary, and the full
supported runtime/platform matrix. Reentrant removal on a callback thread is not
a substitute for those independent requirements. No lifecycle claim here
authorizes package publication or default-provider approval.
