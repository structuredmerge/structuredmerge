use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use structuredmerge_core::*;

fn limits() -> ParseLimits {
    ParseLimits {
        max_batch_items: 16,
        max_input_bytes: 0,
        max_nodes: 0,
        max_diagnostics: 0,
        timeout_millis: None,
    }
}

fn query(profile: &str, operation: OperationKind, dialect: Option<&str>) -> CapabilityQuery {
    CapabilityQuery {
        profile_id: profile.into(),
        operation,
        dialect: dialect.map(str::to_owned),
        parser_selection: ParserSelection {
            backend_id: Some("manifest-test".into()),
            preference: vec![],
            required_capabilities: vec![],
        },
    }
}

struct Host {
    probes: AtomicUsize,
    retire: AtomicBool,
    fault: AtomicBool,
}

impl ParserHost for Host {
    fn descriptor(&self) -> Result<ParserProviderDescriptor, CoreError> {
        Ok(ParserProviderDescriptor {
            id: "manifest-test".into(),
            family: "native".into(),
            runtime: "test".into(),
            package: "test".into(),
            package_version: "1".into(),
            parser: "test".into(),
            parser_version: "1".into(),
            grammar: None,
            grammar_version: None,
            languages: vec!["json".into(), "json5".into(), "tsx".into(), "yaml".into()],
            dialects: vec![],
            contracts: vec![service::PARSE_RESULT_SCHEMA.into()],
            capabilities: vec!["comments".into(), "diagnostics".into(), "native_extensions".into()],
            probe_id: "manifest-test.probe".into(),
            priority: 0,
            metadata: Default::default(),
            extensions: vec![],
        })
    }
    fn probe_batch(&self, request: ProbeBatchRequest) -> Result<ProbeBatchResult, CoreError> {
        self.probes.fetch_add(1, Ordering::SeqCst);
        if self.retire.swap(false, Ordering::SeqCst) {
            unregister_parser_provider("manifest-test".into())?;
        }
        if self.fault.load(Ordering::SeqCst) {
            return Err(CoreError {
                code: "test.offline".into(),
                message: "private probe detail".into(),
            });
        }
        Ok(ProbeBatchResult {
            items: request
                .items
                .iter()
                .map(|_| ParserProbeResult { available: true, loadable: true })
                .collect(),
        })
    }
    fn parse_batch(&self, _: ParseBatchRequest) -> Result<ParseBatchResult, CoreError> {
        panic!("capability observations must never parse source")
    }
}

#[test]
fn manifest_separates_declarations_probes_authority_and_snapshot_lifetime() {
    let host = Arc::new(Host {
        probes: AtomicUsize::new(0),
        retire: AtomicBool::new(false),
        fault: AtomicBool::new(false),
    });
    register_parser_host(host.clone()).unwrap();
    let inventory = capability_manifest(vec![], limits()).unwrap();
    assert_eq!(inventory.profiles.profiles.len(), 8);
    assert_eq!(inventory.parsers.providers[0].id, "manifest-test");
    assert_eq!(host.probes.load(Ordering::SeqCst), 0);

    let unknown = query("unknown", OperationKind::Merge3, None);
    assert_eq!(
        capability_manifest(
            vec![query("kernel.json.nested.v1", OperationKind::Merge3, None), unknown],
            limits()
        )
        .unwrap_err()
        .code,
        "capability.unknown_profile"
    );
    assert_eq!(host.probes.load(Ordering::SeqCst), 0);
    let unsupported = capability_manifest(
        vec![
            query("kernel.yaml.native_mapping.v1", OperationKind::Merge2, None),
            query("kernel.git.json.v1", OperationKind::Analyze, None),
            query("kernel.python.native_declarations.v1", OperationKind::Merge3, Some("python")),
        ],
        limits(),
    )
    .unwrap();
    assert!(!unsupported.observations[0].operation_declared);
    assert!(!unsupported.observations[1].operation_declared);
    assert!(!unsupported.observations[2].dialect_declared);
    assert!(
        unsupported
            .observations
            .iter()
            .all(|o| o.parser_report.is_none() && o.parser_eligible.is_none())
    );
    assert_eq!(host.probes.load(Ordering::SeqCst), 0);

    host.retire.store(true, Ordering::SeqCst);
    let manifest = capability_manifest(
        vec![
            query("kernel.json.nested.v1", OperationKind::Analyze, Some("jsonc")),
            query("kernel.json.nested.v1", OperationKind::Merge3, Some("json")),
            query("kernel.typescript.owners.v1", OperationKind::Merge3, Some("tsx")),
        ],
        limits(),
    )
    .unwrap();
    assert_eq!(host.probes.load(Ordering::SeqCst), 3);
    assert!(parser_registry_inventory().unwrap().providers.is_empty());
    for observation in &manifest.observations {
        assert_eq!(observation.parser_eligible, Some(true));
        assert!(!observation.approved_as_default);
        let report = observation.parser_report.as_ref().unwrap();
        assert_eq!(report.generation, manifest.parsers.generation);
        assert_eq!(report.digest, manifest.parsers.descriptor_digest);
    }
    let request = manifest.observations[0].parser_request.as_ref().unwrap();
    assert_eq!(request.language, "json5");
    assert_eq!(request.dialect, None);
    assert!(
        request.options.comments
            && request.options.diagnostics
            && request.options.native_extensions
    );
    assert_eq!(
        manifest.observations[1].parser_request.as_ref().unwrap().options,
        ParseOptions::default()
    );
    let request = manifest.observations[2].parser_request.as_ref().unwrap();
    assert_eq!(request.language, "tsx");
    assert!(request.options.native_extensions);
    assert!(!request.options.comments);
    let encoded = serde_json::to_string(&manifest).unwrap();
    assert_eq!(serde_json::from_str::<CapabilityManifest>(&encoded).unwrap(), manifest);

    let absent = capability_manifest(
        vec![query("kernel.json.nested.v1", OperationKind::Merge3, None)],
        limits(),
    )
    .unwrap();
    assert_eq!(absent.observations[0].parser_eligible, Some(false));
    register_parser_host(host.clone()).unwrap();
    host.fault.store(true, Ordering::SeqCst);
    let fault = capability_manifest(
        vec![query("kernel.json.nested.v1", OperationKind::Merge3, None)],
        limits(),
    )
    .unwrap();
    assert_eq!(fault.observations[0].parser_eligible, Some(false));
    assert!(
        fault.observations[0].parser_report.as_ref().unwrap().candidates[0].probe_fault.is_some()
    );
    unregister_parser_provider("manifest-test".into()).unwrap();
}

#[test]
fn manifest_enforces_limits_and_execution_control_even_without_queries() {
    let mut zero = limits();
    zero.max_batch_items = 0;
    assert_eq!(
        capability_manifest(
            vec![query("kernel.json.nested.v1", OperationKind::Merge3, None)],
            zero
        )
        .unwrap_err()
        .code,
        "resource.limit"
    );
    let control = OperationControl::new();
    control.cancel();
    assert!(capability_manifest_controlled(vec![], limits(), &control).is_err());
    let mut expired = limits();
    expired.timeout_millis = Some(0);
    assert!(capability_manifest(vec![], expired).is_err());
}
