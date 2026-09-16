//! Native-parser integration gate. Run explicitly with --ignored where Ruby +
//! Psych are installed. This process harness is not the generated binding gate.
use ast_merge::ThreeWayMergeOutcome;
use std::{
    collections::BTreeMap,
    io::Write,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
use tree_haver::{
    parsed::ParseOutput,
    service::*,
    source::{SourceEncoding, SourceRole, source_input},
};
use yaml_merge::typed::{MappingMergeError, diff_mapping_sources, merge_mapping_sources};

fn diff_requests(before: &str, after: &str) -> Vec<ParseRequest> {
    requests(["", before, after])
        .into_iter()
        .skip(1)
        .zip([SourceRole::Before, SourceRole::After])
        .map(|(mut request, role)| {
            request.source.descriptor.role = role;
            request
        })
        .collect()
}

struct Psych {
    descriptor: ParserProviderDescriptor,
    calls: AtomicUsize,
}

impl Psych {
    fn new() -> Self {
        Self {
            descriptor: ParserProviderDescriptor {
                id: "test.psych".into(),
                family: "native".into(),
                runtime: "ruby".into(),
                package: "psych".into(),
                package_version: "test-runtime".into(),
                parser: "psych".into(),
                parser_version: "test-runtime".into(),
                grammar: None,
                grammar_version: None,
                languages: vec!["yaml".into()],
                dialects: vec![],
                contracts: vec![PARSE_RESULT_SCHEMA.into()],
                capabilities: vec!["native_extensions".into(), "source_spans".into()],
                probe_id: "test.psych.require".into(),
                priority: 0,
                metadata: BTreeMap::new(),
                extensions: vec![],
            },
            calls: AtomicUsize::new(0),
        }
    }
}

fn ruby() -> String {
    std::env::var("STRUCTUREDMERGE_NATIVE_RUBY").unwrap_or_else(|_| "ruby".into())
}
fn fault(message: impl ToString) -> ProviderFault {
    ProviderFault { code: "test.psych.process".into(), message: message.to_string() }
}

impl ParserProvider for Psych {
    fn descriptor(&self) -> &ParserProviderDescriptor {
        &self.descriptor
    }
    fn probe(&self, _: &ParserProbeRequest) -> Result<ParserProbeResult, ProviderFault> {
        let status =
            Command::new(ruby()).args(["-rpsych", "-e", "exit 0"]).status().map_err(fault)?;
        Ok(ParserProbeResult { available: status.success(), loadable: status.success() })
    }
    fn parse_batch(
        &self,
        requests: Vec<ParseRequest>,
        _: &ExecutionContext,
    ) -> Result<Vec<ParseOutput>, ProviderFault> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut child = Command::new(ruby())
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/psych_facts.rb"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(fault)?;
        let bytes = serde_json::to_vec(&requests).map_err(fault)?;
        child
            .stdin
            .take()
            .ok_or_else(|| fault("missing stdin"))?
            .write_all(&bytes)
            .map_err(fault)?;
        let output = child.wait_with_output().map_err(fault)?;
        if !output.status.success() {
            return Err(fault(String::from_utf8_lossy(&output.stderr)));
        }
        serde_json::from_slice(&output.stdout).map_err(fault)
    }
}

fn requests(sources: [&str; 3]) -> Vec<ParseRequest> {
    [SourceRole::Base, SourceRole::Ours, SourceRole::Theirs]
        .into_iter()
        .zip(sources)
        .enumerate()
        .map(|(index, (role, source))| {
            let id = format!("source-{index}");
            ParseRequest {
                schema: PARSE_REQUEST_SCHEMA.into(),
                request_id: id.clone(),
                source: source_input(id, role, SourceEncoding::Utf8, source.as_bytes().to_vec())
                    .unwrap(),
                language: "yaml".into(),
                dialect: None,
                selection: ParserSelection {
                    backend_id: Some("test.psych".into()),
                    preference: vec![],
                    required_capabilities: vec!["native_extensions".into(), "source_spans".into()],
                },
                options: ParseOptions { native_extensions: true, ..ParseOptions::default() },
                metadata: BTreeMap::new(),
                extra: BTreeMap::new(),
            }
        })
        .collect()
}

fn setup() -> (Arc<Psych>, ParserRegistrySnapshot, ExecutionContext) {
    let provider = Arc::new(Psych::new());
    let registry = ParserRegistry::default();
    registry.register(provider.clone()).unwrap();
    (
        provider,
        registry.snapshot().unwrap(),
        ExecutionContext {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline: None,
            max_batch_items: 3,
            max_input_bytes: 10000,
            max_nodes: 1000,
            max_diagnostics: 20,
        },
    )
}

#[test]
#[ignore = "native Ruby/Psych integration gate"]
fn native_owner_analysis_retains_actual_ordered_key_value_node_references() {
    let (_, snapshot, context) = setup();
    let parsed = TreeHaverParseService::default()
        .parse_batch(
            requests(["# café\r\né: one\r\nbeta:\r\n  child: two\r\n"; 3]),
            &snapshot,
            &context,
        )
        .unwrap();
    let parsed = &parsed[0];
    let analysis = yaml_merge::typed::mapping_analysis(parsed).unwrap();
    assert_eq!(analysis.document, yaml_merge::typed::mapping_owners(parsed).unwrap());
    assert_eq!(analysis.owner_node_ids.len(), 2);
    for owner in &analysis.document.owners {
        let ids = &analysis.owner_node_ids[&owner.id];
        assert_eq!(ids.len(), 2);
        let key = parsed.document.node(&ids[0]).unwrap();
        let value = parsed.document.node(&ids[1]).unwrap();
        assert_eq!(key.kind, "scalar");
        assert_eq!(key.span.range.start_byte, owner.start_byte);
        assert_eq!(value.span.range.end_byte, owner.end_byte);
        assert_eq!(key.parent_id, value.parent_id);
    }
    analysis.validate(parsed).unwrap();
    // Parse-local node IDs are evidence, never the cross-revision match key.
    assert!(analysis.document.owners.iter().all(|owner| owner.id.starts_with('/')));
}

#[test]
#[ignore = "native Ruby/Psych integration gate"]
fn native_owner_analysis_rejects_stale_or_incomplete_node_provenance() {
    let (_, snapshot, context) = setup();
    let parsed = TreeHaverParseService::default()
        .parse_batch(requests(["a: one\nb: two\n"; 3]), &snapshot, &context)
        .unwrap();
    let parsed = &parsed[0];
    let original = yaml_merge::typed::mapping_analysis(parsed).unwrap();
    let mut corruptions = vec![];
    let mut changed = original.clone();
    changed.document.source.push(' ');
    corruptions.push(changed);
    let mut changed = original.clone();
    changed.owner_node_ids.remove("/a");
    corruptions.push(changed);
    for nodes in [
        vec![],
        vec!["missing".into()],
        vec![original.owner_node_ids["/a"][0].clone(); 2],
        original.owner_node_ids["/a"].iter().rev().cloned().collect(),
        original.owner_node_ids["/b"].clone(),
        vec![original.owner_node_ids["/a"][0].clone()],
    ] {
        let mut changed = original.clone();
        changed.owner_node_ids.insert("/a".into(), nodes);
        corruptions.push(changed);
    }
    for changed in corruptions {
        assert!(changed.validate(parsed).is_err(), "accepted invalid analysis: {changed:?}");
    }
}

#[test]
#[ignore = "native Ruby/Psych integration gate"]
fn merges_independent_native_mapping_changes_in_rust_preserving_exact_bytes() {
    let (provider, snapshot, context) = setup();
    let result = merge_mapping_sources(
        requests([
            "# header\r\né: 'one'  # stable\r\nbeta: two",
            "# header\r\né: 'ours'  # stable\r\nbeta: two",
            "# header\r\né: 'one'  # stable\r\nbeta: theirs",
        ]),
        &TreeHaverParseService::default(),
        &snapshot,
        &context,
    )
    .unwrap();
    assert_eq!(result.outcome, ThreeWayMergeOutcome::Clean);
    assert_eq!(result.output.as_deref(), Some("# header\r\né: 'ours'  # stable\r\nbeta: theirs"));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2); // inputs + rendered-output parse
}

#[test]
#[ignore = "native Ruby/Psych integration gate"]
fn conflicting_changes_are_decided_in_rust_without_fabricated_output() {
    let (_, snapshot, context) = setup();
    let result = merge_mapping_sources(
        requests(["a: one\n", "a: ours\n", "a: theirs\n"]),
        &TreeHaverParseService::default(),
        &snapshot,
        &context,
    )
    .unwrap();
    assert_eq!(result.outcome, ThreeWayMergeOutcome::Conflict);
    assert!(result.output.is_none());
    assert_eq!(result.conflicts.len(), 1);
}

#[test]
#[ignore = "native Ruby/Psych integration gate"]
fn malformed_native_syntax_and_unproven_profiles_fail_closed() {
    for base in [
        "a: [\n",
        "a: one\na: two\n",
        "a: &item one\nb: *item\n",
        "{a: one}\n",
        "true: one\n",
        "? [a, b]\n: value\n",
    ] {
        let (_, snapshot, context) = setup();
        assert!(
            matches!(
                merge_mapping_sources(
                    requests([base, base, base]),
                    &TreeHaverParseService::default(),
                    &snapshot,
                    &context
                ),
                Err(MappingMergeError::Unsupported(_)
                    | MappingMergeError::NativeParseRejected { .. })
            ),
            "source: {base:?}"
        );
    }
}

#[test]
#[ignore = "native Ruby/Psych integration gate"]
fn changed_unowned_comment_layout_is_not_silently_rewritten() {
    let (_, snapshot, context) = setup();
    let result = merge_mapping_sources(
        requests([
            "# base\na: one\nb: two\n",
            "# edited\na: ours\nb: two\n",
            "# base\na: one\nb: theirs\n",
        ]),
        &TreeHaverParseService::default(),
        &snapshot,
        &context,
    )
    .unwrap();
    assert_eq!(result.outcome, ThreeWayMergeOutcome::Error);
    assert!(result.output.is_none());
}

#[test]
#[ignore = "native Ruby/Psych integration gate"]
fn retains_native_syntax_diagnostics_and_obeys_explicit_selection() {
    let (provider, snapshot, context) = setup();
    let error = merge_mapping_sources(
        requests(["a: [\n", "a: 1\n", "a: 2\n"]),
        &TreeHaverParseService::default(),
        &snapshot,
        &context,
    )
    .unwrap_err();
    let MappingMergeError::NativeParseRejected { parses, sources } = error else {
        panic!("expected native syntax failure")
    };
    let parsed = parses.iter().find(|result| !result.document.output().ok).unwrap();
    assert_eq!(parsed.document.output().diagnostics[0].code.as_deref(), Some("psych.syntax"));
    assert_eq!(parsed.document.output().source.role, SourceRole::Base);
    assert_eq!(sources.len(), 3);
    assert_eq!(parsed.selection.selected_backend.as_deref(), Some("test.psych"));
    let mut inputs = requests(["a: one\n", "a: ours\n", "a: theirs\n"]);
    for request in &mut inputs {
        request.selection.backend_id = Some("absent".into());
    }
    assert!(matches!(
        merge_mapping_sources(inputs, &TreeHaverParseService::default(), &snapshot, &context),
        Err(MappingMergeError::Parse(ServiceError::Selection(_)))
    ));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

#[test]
#[ignore = "native Ruby/Psych integration gate"]
fn nested_values_are_whole_owners_and_roles_are_not_argument_positions() {
    let (_, snapshot, context) = setup();
    let mut inputs = requests([
        "parent:\n  nested: base\nother: base\n",
        "parent:\n  nested: ours\nother: base\n",
        "parent:\n  nested: base\nother: theirs\n",
    ]);
    inputs.reverse();
    let result =
        merge_mapping_sources(inputs, &TreeHaverParseService::default(), &snapshot, &context)
            .unwrap();
    assert_eq!(result.output.as_deref(), Some("parent:\n  nested: ours\nother: theirs\n"));
    assert_eq!(result.outcome, ThreeWayMergeOutcome::Clean);
}

#[test]
#[ignore = "native Ruby/Psych integration gate"]
fn bom_is_retained_outside_native_first_key_coordinates() {
    let (_, snapshot, context) = setup();
    let result = merge_mapping_sources(
        requests([
            "\u{feff}é: one\nnext: two\n",
            "\u{feff}é: ours\nnext: two\n",
            "\u{feff}é: one\nnext: theirs\n",
        ]),
        &TreeHaverParseService::default(),
        &snapshot,
        &context,
    )
    .unwrap();
    assert_eq!(result.outcome, ThreeWayMergeOutcome::Clean);
    assert_eq!(result.output.as_deref(), Some("\u{feff}é: ours\nnext: theirs\n"));
}

#[test]
#[ignore = "native Ruby/Psych integration gate"]
fn diffs_native_owners_in_rust_with_one_parse_batch_and_no_rendering() {
    use ast_merge::owner_diff::OwnerChangeKind;
    let (provider, snapshot, context) = setup();
    let mut requests =
        diff_requests("# header\r\na: one\r\nb: two", "# header\r\na: edited\r\nc: three");
    requests.reverse();
    let result =
        diff_mapping_sources(requests, &TreeHaverParseService::default(), &snapshot, &context)
            .unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    assert_eq!(result.input_parses.len(), 2);
    assert_eq!(result.input_parses[0].source.descriptor().role, SourceRole::Before);
    assert_eq!(result.input_parses[1].source.descriptor().role, SourceRole::After);
    assert_eq!(
        result.diff.changes.iter().map(|change| change.kind).collect::<Vec<_>>(),
        vec![OwnerChangeKind::Edited, OwnerChangeKind::Deleted, OwnerChangeKind::Added]
    );
    assert_eq!(result.diff.before_layout[0].range.end_byte, "# header\r\n".len());
    assert_eq!(result.diff.before_layout[0].sha256, result.diff.after_layout[0].sha256);
}

#[test]
#[ignore = "native Ruby/Psych integration gate"]
fn diff_native_parse_and_analysis_failures_retain_exact_roles() {
    let (_, snapshot, context) = setup();
    let result = diff_mapping_sources(
        diff_requests("a: [\n", "a: one\n"),
        &TreeHaverParseService::default(),
        &snapshot,
        &context,
    );
    let Err(MappingMergeError::NativeParseRejected { parses, sources }) = result else {
        panic!("expected native syntax failure")
    };
    assert_eq!(sources.len(), 2);
    assert_eq!(parses[0].source.descriptor().role, SourceRole::Before);
    assert!(!parses[0].document.output().ok);
    assert_eq!(parses[0].document.output().diagnostics[0].code.as_deref(), Some("psych.syntax"));
    let result = diff_mapping_sources(
        diff_requests("a: one\n", "a: one\na: two\n"),
        &TreeHaverParseService::default(),
        &snapshot,
        &context,
    );
    let Err(MappingMergeError::AnalysisRejected { failures, parses, sources }) = result else {
        panic!("expected ownership rejection")
    };
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].source_role, SourceRole::After);
    assert_eq!(parses.len(), 2);
    assert_eq!(sources.len(), 2);
}

#[test]
#[ignore = "native Ruby/Psych integration gate"]
fn diff_rejects_bad_roles_corrupt_sources_and_limits_before_parsing() {
    let (provider, snapshot, mut context) = setup();
    let service = TreeHaverParseService::default();
    assert!(matches!(
        diff_mapping_sources(requests(["a: 1", "a: 2", "a: 3"]), &service, &snapshot, &context),
        Err(MappingMergeError::InvalidInputs)
    ));
    let mut corrupt = diff_requests("a: one", "a: two");
    corrupt[1].source.bytes[0] = b'z';
    assert!(matches!(
        diff_mapping_sources(corrupt, &service, &snapshot, &context),
        Err(MappingMergeError::Parse(ServiceError::Source(_)))
    ));
    context.max_batch_items = 1;
    assert!(matches!(
        diff_mapping_sources(diff_requests("a: one", "a: two"), &service, &snapshot, &context),
        Err(MappingMergeError::Parse(ServiceError::LimitExceeded))
    ));
    context.cancelled.store(true, Ordering::SeqCst);
    assert!(matches!(
        diff_mapping_sources(diff_requests("a: one", "a: two"), &service, &snapshot, &context),
        Err(MappingMergeError::Parse(ServiceError::Cancelled))
    ));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
}

#[test]
#[ignore = "native Ruby/Psych integration gate"]
fn diff_never_substitutes_an_explicit_missing_parser() {
    let (provider, snapshot, context) = setup();
    let mut requests = diff_requests("a: one", "a: two");
    for request in &mut requests {
        request.selection.backend_id = Some("missing".into());
    }
    let error =
        diff_mapping_sources(requests, &TreeHaverParseService::default(), &snapshot, &context)
            .unwrap_err();
    let MappingMergeError::InputParseFailed { error: ServiceError::Selection(report), sources } =
        error
    else {
        panic!("expected selection failure")
    };
    assert!(report.selected_backend.is_none());
    assert_eq!(sources.len(), 2);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
}

struct CancellingPsych {
    inner: Psych,
    fail: bool,
}

impl ParserProvider for CancellingPsych {
    fn descriptor(&self) -> &ParserProviderDescriptor {
        self.inner.descriptor()
    }
    fn probe(&self, request: &ParserProbeRequest) -> Result<ParserProbeResult, ProviderFault> {
        self.inner.probe(request)
    }
    fn parse_batch(
        &self,
        requests: Vec<ParseRequest>,
        context: &ExecutionContext,
    ) -> Result<Vec<ParseOutput>, ProviderFault> {
        let parsed = self.inner.parse_batch(requests, context);
        context.cancelled.store(true, Ordering::SeqCst);
        if self.fail { Err(fault("late native error")) } else { parsed }
    }
}

#[test]
#[ignore = "native Ruby/Psych integration gate"]
fn diff_discards_late_native_results_and_errors_after_cancellation() {
    for fail in [false, true] {
        let provider = Arc::new(CancellingPsych { inner: Psych::new(), fail });
        let registry = ParserRegistry::default();
        registry.register(provider.clone()).unwrap();
        let (_, _, context) = setup();
        let result = diff_mapping_sources(
            diff_requests("a: one", "a: two"),
            &TreeHaverParseService::default(),
            &registry.snapshot().unwrap(),
            &context,
        );
        assert!(matches!(result, Err(MappingMergeError::Parse(ServiceError::Cancelled))));
        assert_eq!(provider.inner.calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
#[ignore = "native Ruby/Psych integration gate"]
fn diff_checks_family_analysis_against_validated_sources() {
    fn stale(_: &ParsedResult) -> Result<ast_merge::SourcePreservingOwnerDocument, String> {
        Ok(ast_merge::SourcePreservingOwnerDocument { source: "stale".into(), owners: vec![] })
    }
    let (_, snapshot, context) = setup();
    let result = ast_merge::typed_diff::diff_native_sources_with_evidence(
        "yaml",
        diff_requests("a: one", "a: two"),
        &TreeHaverParseService::default(),
        &snapshot,
        &context,
        stale,
    );
    let Err(MappingMergeError::AnalysisRejected { failures, parses, sources }) = result else {
        panic!("expected stale analysis rejection")
    };
    assert_eq!(
        failures.iter().map(|failure| failure.source_role).collect::<Vec<_>>(),
        vec![SourceRole::Before, SourceRole::After]
    );
    assert_eq!(parses.len(), 2);
    assert_eq!(sources.len(), 2);
}

#[test]
#[ignore = "native Ruby/Psych integration gate"]
fn native_execution_retains_the_actual_owner_classification_decisions() {
    use ast_merge::OwnerDecisionKind as D;
    for (sources, expected) in [
        (
            ["anchor: kept\na: one\n", "anchor: kept\na: ours\n", "anchor: kept\na: theirs\n"],
            D::ConflictEditEdit,
        ),
        (
            ["anchor: kept\na: one\n", "anchor: kept\n", "anchor: kept\na: theirs\n"],
            D::ConflictDeleteModify,
        ),
        (
            ["anchor: kept\n", "anchor: kept\na: ours\n", "anchor: kept\na: theirs\n"],
            D::ConflictAddAdd,
        ),
    ] {
        let (_, snapshot, context) = setup();
        let result = ast_merge::typed_merge::merge_native_sources_with_evidence(
            "yaml",
            requests(sources),
            &TreeHaverParseService::default(),
            &snapshot,
            &context,
            yaml_merge::typed::mapping_owners,
        )
        .unwrap();
        let evidence = result.rendered.classification.unwrap();
        assert!(evidence.whole_source_selection.is_none());
        let decision =
            evidence.decisions.iter().find(|decision| decision.conflict_id.is_some()).unwrap();
        assert_eq!(decision.kind, expected);
        assert_eq!(
            decision.conflict_id.as_ref().unwrap(),
            &result.rendered.result.conflicts[0].conflict_id
        );
        assert_eq!(decision.alternatives, result.rendered.result.conflicts[0].alternatives);
        assert_eq!(result.input_parses.len(), 3);
        assert!(result.output_parse.is_none());
        assert!(result.rendered.result.output.is_none());
    }
}

#[test]
#[ignore = "native Ruby/Psych integration gate"]
fn merge_rejects_family_analysis_that_changes_validated_input_bytes() {
    fn stale(_: &ParsedResult) -> Result<ast_merge::SourcePreservingOwnerDocument, String> {
        Ok(ast_merge::SourcePreservingOwnerDocument { source: "stale".into(), owners: vec![] })
    }
    let (_, snapshot, context) = setup();
    let result = ast_merge::typed_merge::merge_native_sources_with_evidence(
        "yaml",
        requests(["a: base", "a: ours", "a: theirs"]),
        &TreeHaverParseService::default(),
        &snapshot,
        &context,
        stale,
    );
    let Err(MappingMergeError::AnalysisRejected { failures, parses, sources }) = result else {
        panic!("expected stale analysis rejection before classification")
    };
    assert_eq!(
        failures.iter().map(|failure| failure.source_role).collect::<Vec<_>>(),
        vec![SourceRole::Base, SourceRole::Ours, SourceRole::Theirs]
    );
    assert_eq!(parses.len(), 3);
    assert_eq!(sources.len(), 3);
}
