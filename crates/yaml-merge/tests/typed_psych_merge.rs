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
use yaml_merge::typed::{MappingMergeError, merge_mapping_sources};

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
