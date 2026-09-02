use std::{
    collections::BTreeMap,
    env, fs,
    io::{BufRead, Write},
    time::Instant,
};

use ast_merge::{DiagnosticCategory, ThreeWayMergeOutcome};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use bash_merge::{BashDialect, merge_bash_three_way};
use go_merge::{GoDialect, merge_go_three_way};
use json_merge::{
    JsonDialect, json_semantically_equivalent, merge_json_source_preserving, merge_json_three_way,
};
use markdown_merge::{MarkdownDialect, merge_markdown_source_preserving};
use rbs_merge::{RbsDialect, merge_rbs};
use ruby_merge::{RubyDialect, merge_ruby};
use rust_merge::{RustDialect, merge_rust_three_way};
use serde::Deserialize;
use toml_merge::{TomlDialect, merge_toml};
use typescript_merge::{TypeScriptDialect, merge_typescript_three_way};
use yaml_merge::{YamlDialect, merge_yaml};

const REQUEST_SCHEMA: &str = "structuredmerge.benchmark.adapter-request/v1";
const RESPONSE_SCHEMA: &str = "structuredmerge.benchmark.adapter-response/v1";

#[derive(Deserialize)]
struct Request {
    schema_version: String,
    request_id: String,
    operation: String,
    selector: Selector,
    sources: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct Selector {
    family: String,
    dialect: String,
}

#[derive(Clone, Copy)]
enum BenchmarkDialect {
    Bash(BashDialect),
    Go(GoDialect),
    Json(JsonDialect),
    Markdown(MarkdownDialect),
    Rbs(RbsDialect),
    Ruby(RubyDialect),
    Rust(RustDialect),
    Toml(TomlDialect),
    TypeScript(TypeScriptDialect),
    Yaml(YamlDialect),
}

pub fn serve(input: &mut dyn BufRead, output: &mut dyn Write) -> Result<(), String> {
    for line in input.lines() {
        let line = line.map_err(|error| format!("read benchmark request: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let response = execute_line(&line);
        serde_json::to_writer(&mut *output, &response)
            .map_err(|error| format!("serialize benchmark response: {error}"))?;
        output.write_all(b"\n").map_err(|error| format!("write benchmark response: {error}"))?;
        output.flush().map_err(|error| format!("flush benchmark response: {error}"))?;
    }
    Ok(())
}

pub fn run_merge2_files(args: &[String], output: &mut dyn Write, error: &mut dyn Write) -> i32 {
    if args.len() != 3 {
        let _ = writeln!(error, "smorg-rs: configuration_error: expected incoming current path");
        return 2;
    }
    let result = (|| {
        let incoming = read_source(&args[0], "incoming")?;
        let current = read_source(&args[1], "current")?;
        let dialect = selected_dialect(&args[2])?;
        Ok::<_, String>(match dialect {
            BenchmarkDialect::Bash(_) => {
                return Err("Bash merge2 is not source-preserving yet".to_string());
            }
            BenchmarkDialect::Go(_) => {
                return Err("Go merge2 is not source-preserving yet".to_string());
            }
            BenchmarkDialect::Json(dialect) => {
                merge_json_source_preserving(&incoming, &current, dialect)
            }
            BenchmarkDialect::Markdown(dialect) => {
                merge_markdown_source_preserving(&current, &incoming, dialect)
            }
            BenchmarkDialect::Rbs(dialect) => merge_rbs(&incoming, &current, dialect),
            BenchmarkDialect::Ruby(dialect) => merge_ruby(&incoming, &current, dialect),
            BenchmarkDialect::Rust(_) => {
                return Err("Rust merge2 is not source-preserving yet".to_string());
            }
            BenchmarkDialect::Toml(dialect) => merge_toml(&incoming, &current, dialect, None),
            BenchmarkDialect::TypeScript(_) => {
                return Err("TypeScript merge2 is not source-preserving yet".to_string());
            }
            BenchmarkDialect::Yaml(dialect) => merge_yaml(&incoming, &current, dialect),
        })
    })();
    match result {
        Ok(result) if result.ok => {
            let payload = serde_json::json!({"output": result.output.unwrap_or_default()});
            if let Err(cause) = serde_json::to_writer(output, &payload) {
                let _ = writeln!(error, "smorg-rs: process: serialize merge2 response: {cause}");
                return 3;
            }
            0
        }
        Ok(result) => {
            write_diagnostics(error, &result.diagnostics);
            2
        }
        Err(message) => {
            let _ = writeln!(error, "smorg-rs: process: {message}");
            2
        }
    }
}

pub fn run_diff_files(args: &[String], output: &mut dyn Write, error: &mut dyn Write) -> i32 {
    if args.len() != 3 {
        let _ = writeln!(error, "smorg-rs: configuration_error: expected before after path");
        return 2;
    }
    let result = (|| {
        let before = read_source(&args[0], "before")?;
        let after = read_source(&args[1], "after")?;
        let dialect = selected_dialect(&args[2])?;
        let BenchmarkDialect::Json(dialect) = dialect else {
            return Err(format!(
                "unsupported benchmark diff family: {}",
                benchmark_family(dialect)
            ));
        };
        json_semantically_equivalent(&before, &after, dialect)
    })();
    match result {
        Ok(equivalent) => {
            let changes = if equivalent {
                Vec::new()
            } else {
                vec![serde_json::json!({"path": "", "kind": "modify"})]
            };
            if let Err(cause) =
                serde_json::to_writer(output, &serde_json::json!({"changes": changes}))
            {
                let _ = writeln!(error, "smorg-rs: process: serialize diff response: {cause}");
                return 3;
            }
            0
        }
        Err(message) => {
            let _ = writeln!(error, "smorg-rs: parse_error: {message}");
            2
        }
    }
}

pub fn run_merge3_files(args: &[String], error: &mut dyn Write) -> i32 {
    if args.len() < 4 {
        let _ = writeln!(error, "smorg-rs: configuration_error: expected base ours theirs path");
        return 2;
    }
    let result = (|| {
        let base = read_source(&args[0], "base")?;
        let ours = read_source(&args[1], "ours")?;
        let theirs = read_source(&args[2], "theirs")?;
        let dialect = selected_dialect(&args[3])?;
        let result = match dialect {
            BenchmarkDialect::Bash(dialect) => merge_bash_three_way(&base, &ours, &theirs, dialect),
            BenchmarkDialect::Go(dialect) => merge_go_three_way(&base, &ours, &theirs, dialect),
            BenchmarkDialect::Json(dialect) => merge_json_three_way(&base, &ours, &theirs, dialect),
            BenchmarkDialect::Rust(dialect) => merge_rust_three_way(&base, &ours, &theirs, dialect),
            BenchmarkDialect::TypeScript(dialect) => {
                merge_typescript_three_way(&base, &ours, &theirs, dialect)
            }
            _ => {
                return Err(format!(
                    "unsupported benchmark merge3 family: {}",
                    benchmark_family(dialect)
                ));
            }
        };
        Ok::<_, String>(result)
    })();
    match result {
        Ok(result) if result.outcome == ThreeWayMergeOutcome::Clean => {
            match fs::write(&args[1], result.output.unwrap_or_default()) {
                Ok(()) => 0,
                Err(cause) => {
                    let _ = writeln!(error, "smorg-rs: process: write ours output: {cause}");
                    3
                }
            }
        }
        Ok(result) if result.outcome == ThreeWayMergeOutcome::Conflict => {
            write_diagnostics(error, &result.diagnostics);
            1
        }
        Ok(result) => {
            write_diagnostics(error, &result.diagnostics);
            2
        }
        Err(message) => {
            let _ = writeln!(error, "smorg-rs: process: {message}");
            2
        }
    }
}

fn read_source(path: &str, role: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("read {role} source {path:?}: {error}"))
}

fn selected_dialect(path: &str) -> Result<BenchmarkDialect, String> {
    if let Ok(dialect) = env::var("AST_MERGE_DIALECT") {
        return parse_benchmark_dialect(&dialect);
    }
    let extension = path.rsplit_once('.').map(|(_, extension)| extension).unwrap_or("json");
    parse_benchmark_dialect(extension)
}

fn benchmark_family(dialect: BenchmarkDialect) -> &'static str {
    match dialect {
        BenchmarkDialect::Bash(_) => "bash",
        BenchmarkDialect::Go(_) => "go",
        BenchmarkDialect::Json(_) => "json",
        BenchmarkDialect::Markdown(_) => "markdown",
        BenchmarkDialect::Rbs(_) => "rbs",
        BenchmarkDialect::Ruby(_) => "ruby",
        BenchmarkDialect::Rust(_) => "rust",
        BenchmarkDialect::Toml(_) => "toml",
        BenchmarkDialect::TypeScript(_) => "typescript",
        BenchmarkDialect::Yaml(_) => "yaml",
    }
}

fn write_diagnostics(error: &mut dyn Write, diagnostics: &[ast_merge::Diagnostic]) {
    for diagnostic in diagnostics {
        let category = diagnostic_category_name(diagnostic.category);
        let _ = writeln!(error, "smorg-rs: {category}: {}", diagnostic.message);
    }
}

fn diagnostic_category_name(category: DiagnosticCategory) -> &'static str {
    match category {
        DiagnosticCategory::ParseError => "parse_error",
        DiagnosticCategory::DestinationParseError => "destination_parse_error",
        DiagnosticCategory::UnsupportedFeature => "unsupported_feature",
        DiagnosticCategory::FallbackApplied => "fallback_applied",
        DiagnosticCategory::Ambiguity => "ambiguity",
        DiagnosticCategory::MergeConflict => "merge_conflict",
        DiagnosticCategory::KindMismatch => "kind_mismatch",
        DiagnosticCategory::UnsupportedVersion => "unsupported_version",
        DiagnosticCategory::AssumedDefault => "assumed_default",
        DiagnosticCategory::ConfigurationError => "configuration_error",
        DiagnosticCategory::ReplayRejected => "replay_rejected",
    }
}

fn execute_line(line: &str) -> serde_json::Value {
    let started = Instant::now();
    let request = match serde_json::from_str::<Request>(line) {
        Ok(request) => request,
        Err(error) => return error_response(None, started, format!("invalid request: {error}")),
    };
    let request_id = request.request_id.clone();
    match execute(request) {
        Ok((status, output, result)) => serde_json::json!({
            "schema_version": RESPONSE_SCHEMA,
            "request_id": request_id,
            "process_id": std::process::id(),
            "status": status,
            "duration_ns": elapsed_nanoseconds(started),
            "output_base64": STANDARD.encode(output.as_bytes()),
            "result": result,
            "stderr": ""
        }),
        Err(message) => error_response(Some(request_id), started, message),
    }
}

fn execute(request: Request) -> Result<(i32, String, serde_json::Value), String> {
    if request.schema_version != REQUEST_SCHEMA {
        return Err(format!("unsupported request schema: {}", request.schema_version));
    }
    match request.selector.family.as_str() {
        "bash" => execute_bash(request),
        "go" => execute_go(request),
        "json" => execute_json(request),
        "markdown" => execute_markdown(request),
        "rbs" => execute_rbs(request),
        "ruby" => execute_ruby(request),
        "rust" => execute_rust(request),
        "toml" => execute_toml(request),
        "typescript" => execute_typescript(request),
        "yaml" => execute_yaml(request),
        family => Err(format!("unsupported benchmark family: {family}")),
    }
}

fn execute_bash(request: Request) -> Result<(i32, String, serde_json::Value), String> {
    if request.selector.dialect != "bash" {
        return Err(format!("unsupported Bash dialect: {}", request.selector.dialect));
    }
    if request.operation != "merge3" {
        return Err(format!("unsupported Bash benchmark operation: {}", request.operation));
    }

    let base = source(&request.sources, "base")?;
    let ours = source(&request.sources, "ours")?;
    let theirs = source(&request.sources, "theirs")?;
    let result = merge_bash_three_way(&base, &ours, &theirs, BashDialect::Bash);
    let status = match result.outcome {
        ThreeWayMergeOutcome::Clean => 0,
        ThreeWayMergeOutcome::Conflict => 1,
        ThreeWayMergeOutcome::Error => 2,
    };
    let output = result.output.clone().unwrap_or_default();
    let result = serde_json::to_value(result)
        .map_err(|error| format!("serialize Bash merge3 result: {error}"))?;
    Ok((status, output, result))
}

fn execute_rust(request: Request) -> Result<(i32, String, serde_json::Value), String> {
    if request.selector.dialect != "rust" {
        return Err(format!("unsupported Rust dialect: {}", request.selector.dialect));
    }
    if request.operation != "merge3" {
        return Err(format!("unsupported Rust benchmark operation: {}", request.operation));
    }

    let base = source(&request.sources, "base")?;
    let ours = source(&request.sources, "ours")?;
    let theirs = source(&request.sources, "theirs")?;
    let result = merge_rust_three_way(&base, &ours, &theirs, RustDialect::Rust);
    let status = match result.outcome {
        ThreeWayMergeOutcome::Clean => 0,
        ThreeWayMergeOutcome::Conflict => 1,
        ThreeWayMergeOutcome::Error => 2,
    };
    let output = result.output.clone().unwrap_or_default();
    let result = serde_json::to_value(result)
        .map_err(|error| format!("serialize Rust merge3 result: {error}"))?;
    Ok((status, output, result))
}

fn execute_go(request: Request) -> Result<(i32, String, serde_json::Value), String> {
    if request.selector.dialect != "go" {
        return Err(format!("unsupported Go dialect: {}", request.selector.dialect));
    }
    if request.operation != "merge3" {
        return Err(format!("unsupported Go benchmark operation: {}", request.operation));
    }

    let base = source(&request.sources, "base")?;
    let ours = source(&request.sources, "ours")?;
    let theirs = source(&request.sources, "theirs")?;
    let result = merge_go_three_way(&base, &ours, &theirs, GoDialect::Go);
    let status = match result.outcome {
        ThreeWayMergeOutcome::Clean => 0,
        ThreeWayMergeOutcome::Conflict => 1,
        ThreeWayMergeOutcome::Error => 2,
    };
    let output = result.output.clone().unwrap_or_default();
    let result = serde_json::to_value(result)
        .map_err(|error| format!("serialize Go merge3 result: {error}"))?;
    Ok((status, output, result))
}

fn execute_typescript(request: Request) -> Result<(i32, String, serde_json::Value), String> {
    let dialect = match request.selector.dialect.as_str() {
        "typescript" | "ts" => TypeScriptDialect::TypeScript,
        "tsx" => TypeScriptDialect::Tsx,
        dialect => return Err(format!("unsupported TypeScript dialect: {dialect}")),
    };
    if request.operation != "merge3" {
        return Err(format!("unsupported TypeScript benchmark operation: {}", request.operation));
    }

    let base = source(&request.sources, "base")?;
    let ours = source(&request.sources, "ours")?;
    let theirs = source(&request.sources, "theirs")?;
    let result = merge_typescript_three_way(&base, &ours, &theirs, dialect);
    let status = match result.outcome {
        ThreeWayMergeOutcome::Clean => 0,
        ThreeWayMergeOutcome::Conflict => 1,
        ThreeWayMergeOutcome::Error => 2,
    };
    let output = result.output.clone().unwrap_or_default();
    let result = serde_json::to_value(result)
        .map_err(|error| format!("serialize TypeScript merge3 result: {error}"))?;
    Ok((status, output, result))
}

fn execute_markdown(request: Request) -> Result<(i32, String, serde_json::Value), String> {
    if request.selector.dialect != "markdown" {
        return Err(format!("unsupported Markdown dialect: {}", request.selector.dialect));
    }
    if request.operation != "merge2" {
        return Err(format!("unsupported Markdown benchmark operation: {}", request.operation));
    }

    let incoming = source(&request.sources, "incoming")?;
    let current = source(&request.sources, "current")?;
    let result = merge_markdown_source_preserving(&current, &incoming, MarkdownDialect::Markdown);
    let status = if result.ok { 0 } else { 2 };
    let output = result.output.clone().unwrap_or_default();
    let result = serde_json::to_value(result)
        .map_err(|error| format!("serialize merge2 result: {error}"))?;
    Ok((status, output, result))
}

fn execute_rbs(request: Request) -> Result<(i32, String, serde_json::Value), String> {
    if request.selector.dialect != "rbs" {
        return Err(format!("unsupported RBS dialect: {}", request.selector.dialect));
    }
    if request.operation != "merge2" {
        return Err(format!("unsupported RBS benchmark operation: {}", request.operation));
    }

    let incoming = source(&request.sources, "incoming")?;
    let current = source(&request.sources, "current")?;
    let result = merge_rbs(&incoming, &current, RbsDialect::Rbs);
    let status = if result.ok { 0 } else { 2 };
    let output = result.output.clone().unwrap_or_default();
    let result = serde_json::to_value(result)
        .map_err(|error| format!("serialize merge2 result: {error}"))?;
    Ok((status, output, result))
}

fn execute_json(request: Request) -> Result<(i32, String, serde_json::Value), String> {
    let dialect = parse_dialect(&request.selector.dialect)?;
    match request.operation.as_str() {
        "diff2" => {
            let before = source(&request.sources, "before")?;
            let after = source(&request.sources, "after")?;
            let equivalent = json_semantically_equivalent(&before, &after, dialect)?;
            let changes = if equivalent {
                Vec::new()
            } else {
                vec![serde_json::json!({"path": "", "kind": "modify"})]
            };
            let output = serde_json::to_string(&changes)
                .map_err(|error| format!("serialize diff output: {error}"))?;
            Ok((
                0,
                output,
                serde_json::json!({
                    "ok": true,
                    "changes": changes,
                    "diagnostics": [],
                    "verification": {"before_parsed": true, "after_parsed": true}
                }),
            ))
        }
        "merge2" => {
            let incoming = source(&request.sources, "incoming")?;
            let current = source(&request.sources, "current")?;
            let result = merge_json_source_preserving(&incoming, &current, dialect);
            let status = if result.ok { 0 } else { 2 };
            let output = result.output.clone().unwrap_or_default();
            let result = serde_json::to_value(result)
                .map_err(|error| format!("serialize merge2 result: {error}"))?;
            Ok((status, output, result))
        }
        "merge3" => {
            let base = source(&request.sources, "base")?;
            let ours = source(&request.sources, "ours")?;
            let theirs = source(&request.sources, "theirs")?;
            let result = merge_json_three_way(&base, &ours, &theirs, dialect);
            let status = match result.outcome {
                ThreeWayMergeOutcome::Clean => 0,
                ThreeWayMergeOutcome::Conflict => 1,
                ThreeWayMergeOutcome::Error => 2,
            };
            let output = result.output.clone().unwrap_or_default();
            let result = serde_json::to_value(result)
                .map_err(|error| format!("serialize merge3 result: {error}"))?;
            Ok((status, output, result))
        }
        operation => Err(format!("unsupported benchmark operation: {operation}")),
    }
}

fn execute_yaml(request: Request) -> Result<(i32, String, serde_json::Value), String> {
    if request.selector.dialect != "yaml" {
        return Err(format!("unsupported YAML dialect: {}", request.selector.dialect));
    }
    if request.operation != "merge2" {
        return Err(format!("unsupported YAML benchmark operation: {}", request.operation));
    }

    let incoming = source(&request.sources, "incoming")?;
    let current = source(&request.sources, "current")?;
    let result = merge_yaml(&incoming, &current, YamlDialect::Yaml);
    let status = if result.ok { 0 } else { 2 };
    let output = result.output.clone().unwrap_or_default();
    let result = serde_json::to_value(result)
        .map_err(|error| format!("serialize merge2 result: {error}"))?;
    Ok((status, output, result))
}

fn execute_ruby(request: Request) -> Result<(i32, String, serde_json::Value), String> {
    if request.selector.dialect != "ruby" {
        return Err(format!("unsupported Ruby dialect: {}", request.selector.dialect));
    }
    if request.operation != "merge2" {
        return Err(format!("unsupported Ruby benchmark operation: {}", request.operation));
    }

    let incoming = source(&request.sources, "incoming")?;
    let current = source(&request.sources, "current")?;
    let result = merge_ruby(&incoming, &current, RubyDialect::Ruby);
    let status = if result.ok { 0 } else { 2 };
    let output = result.output.clone().unwrap_or_default();
    let result = serde_json::to_value(result)
        .map_err(|error| format!("serialize merge2 result: {error}"))?;
    Ok((status, output, result))
}

fn execute_toml(request: Request) -> Result<(i32, String, serde_json::Value), String> {
    if request.selector.dialect != "toml" {
        return Err(format!("unsupported TOML dialect: {}", request.selector.dialect));
    }
    if request.operation != "merge2" {
        return Err(format!("unsupported TOML benchmark operation: {}", request.operation));
    }

    let incoming = source(&request.sources, "incoming")?;
    let current = source(&request.sources, "current")?;
    let result = merge_toml(&incoming, &current, TomlDialect::Toml, None);
    let status = if result.ok { 0 } else { 2 };
    let output = result.output.clone().unwrap_or_default();
    let result = serde_json::to_value(result)
        .map_err(|error| format!("serialize merge2 result: {error}"))?;
    Ok((status, output, result))
}

fn source(sources: &BTreeMap<String, String>, role: &str) -> Result<String, String> {
    let encoded = sources.get(role).ok_or_else(|| format!("missing {role} source"))?;
    let bytes =
        STANDARD.decode(encoded).map_err(|error| format!("invalid {role} base64: {error}"))?;
    String::from_utf8(bytes).map_err(|error| format!("{role} source is not UTF-8: {error}"))
}

pub fn parse_dialect(value: &str) -> Result<JsonDialect, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "json" => Ok(JsonDialect::Json),
        "jsonc" => Ok(JsonDialect::Jsonc),
        "json5" => Ok(JsonDialect::Json5),
        _ => Err(format!("unsupported JSON dialect: {value}")),
    }
}

fn parse_benchmark_dialect(value: &str) -> Result<BenchmarkDialect, String> {
    if value.trim().eq_ignore_ascii_case("bash") || value.trim().eq_ignore_ascii_case("sh") {
        Ok(BenchmarkDialect::Bash(BashDialect::Bash))
    } else if value.trim().eq_ignore_ascii_case("go") {
        Ok(BenchmarkDialect::Go(GoDialect::Go))
    } else if value.trim().eq_ignore_ascii_case("toml") {
        Ok(BenchmarkDialect::Toml(TomlDialect::Toml))
    } else if value.trim().eq_ignore_ascii_case("markdown")
        || value.trim().eq_ignore_ascii_case("md")
    {
        Ok(BenchmarkDialect::Markdown(MarkdownDialect::Markdown))
    } else if value.trim().eq_ignore_ascii_case("rbs") {
        Ok(BenchmarkDialect::Rbs(RbsDialect::Rbs))
    } else if value.trim().eq_ignore_ascii_case("ruby") || value.trim().eq_ignore_ascii_case("rb") {
        Ok(BenchmarkDialect::Ruby(RubyDialect::Ruby))
    } else if value.trim().eq_ignore_ascii_case("rust") || value.trim().eq_ignore_ascii_case("rs") {
        Ok(BenchmarkDialect::Rust(RustDialect::Rust))
    } else if value.trim().eq_ignore_ascii_case("yaml") || value.trim().eq_ignore_ascii_case("yml")
    {
        Ok(BenchmarkDialect::Yaml(YamlDialect::Yaml))
    } else if value.trim().eq_ignore_ascii_case("typescript")
        || value.trim().eq_ignore_ascii_case("ts")
    {
        Ok(BenchmarkDialect::TypeScript(TypeScriptDialect::TypeScript))
    } else if value.trim().eq_ignore_ascii_case("tsx") {
        Ok(BenchmarkDialect::TypeScript(TypeScriptDialect::Tsx))
    } else {
        parse_dialect(value).map(BenchmarkDialect::Json)
    }
}

fn error_response(
    request_id: Option<String>,
    started: Instant,
    message: String,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": RESPONSE_SCHEMA,
        "request_id": request_id,
        "process_id": std::process::id(),
        "status": 2,
        "duration_ns": elapsed_nanoseconds(started),
        "output_base64": "",
        "result": {},
        "stderr": message
    })
}

fn elapsed_nanoseconds(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_fixture(directory: &std::path::Path, name: &str, source: &str) -> String {
        let path = directory.join(name);
        fs::write(&path, source).unwrap();
        path.to_string_lossy().into_owned()
    }

    fn request_for(
        operation: &str,
        family: &str,
        dialect: &str,
        sources: &[(&str, &str)],
    ) -> String {
        serde_json::json!({
            "schema_version": REQUEST_SCHEMA,
            "request_id": "request-1",
            "operation": operation,
            "selector": {
                "provider_id": format!("rust.{family}"),
                "family": family,
                "dialect": dialect,
                "backend": "kreuzberg-language-pack",
                "profile": "source_preserving",
                "require": "json/merge"
            },
            "path_name": "fixture.json",
            "sources": sources.iter().map(|(role, value)| {
                ((*role).to_string(), STANDARD.encode(value.as_bytes()))
            }).collect::<BTreeMap<_, _>>()
        })
        .to_string()
    }

    fn request(operation: &str, dialect: &str, sources: &[(&str, &str)]) -> String {
        request_for(operation, "json", dialect, sources)
    }

    #[test]
    fn serves_merge2_and_merge3_requests_in_one_process() {
        let input = [
            request(
                "merge2",
                "json",
                &[("incoming", "{\"template\":true}"), ("current", "{\"current\":true}")],
            ),
            request(
                "merge3",
                "json",
                &[
                    ("base", "{\"left\":1,\"right\":1}"),
                    ("ours", "{\"left\":2,\"right\":1}"),
                    ("theirs", "{\"left\":1,\"right\":2}"),
                ],
            ),
        ]
        .join("\n");
        let mut output = Vec::new();

        serve(&mut input.as_bytes(), &mut output).unwrap();

        let responses = String::from_utf8(output).unwrap();
        let responses = responses
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(responses[0]["status"], 0);
        assert_eq!(responses[1]["status"], 0);
        assert_eq!(responses[0]["process_id"], responses[1]["process_id"]);
        assert_eq!(
            String::from_utf8(
                STANDARD.decode(responses[1]["output_base64"].as_str().unwrap()).unwrap()
            )
            .unwrap(),
            "{\"left\":2,\"right\":2}"
        );
    }

    #[test]
    fn serves_yaml_merge2_without_claiming_yaml_merge3() {
        let merge2 = execute_line(&request_for(
            "merge2",
            "yaml",
            "yaml",
            &[
                ("incoming", "service:\n  image: app:latest\n  replicas: 1\n"),
                ("current", "service:\n  replicas: 3\n"),
            ],
        ));
        let merge3 = execute_line(&request_for(
            "merge3",
            "yaml",
            "yaml",
            &[("base", "a: 1\n"), ("ours", "a: 2\n"), ("theirs", "a: 3\n")],
        ));

        assert_eq!(merge2["status"], 0);
        assert_eq!(
            String::from_utf8(STANDARD.decode(merge2["output_base64"].as_str().unwrap()).unwrap())
                .unwrap(),
            "service:\n  image: app:latest\n  replicas: 3\n"
        );
        assert_eq!(merge3["status"], 2);
        assert!(
            merge3["stderr"].as_str().unwrap().contains("unsupported YAML benchmark operation")
        );
    }

    #[test]
    fn serves_markdown_merge2_without_claiming_markdown_merge3() {
        let incoming =
            "# Title\n\ntemplate body\n\n# Added\n\nnew section\n\n# Last\n\ntemplate ending\n";
        let current = "# Title\r\n\r\ncurrent body\r\n\r\n# Last\r\n\r\ncurrent ending";
        let expected = "# Title\r\n\r\ncurrent body\r\n\r\n# Added\n\nnew section\n\n# Last\r\n\r\ncurrent ending";
        let merge2 = execute_line(&request_for(
            "merge2",
            "markdown",
            "markdown",
            &[("incoming", incoming), ("current", current)],
        ));
        let merge3 = execute_line(&request_for(
            "merge3",
            "markdown",
            "markdown",
            &[("base", current), ("ours", current), ("theirs", incoming)],
        ));

        assert_eq!(merge2["status"], 0);
        assert_eq!(
            String::from_utf8(STANDARD.decode(merge2["output_base64"].as_str().unwrap()).unwrap())
                .unwrap(),
            expected
        );
        assert_eq!(merge3["status"], 2);
        assert!(
            merge3["stderr"].as_str().unwrap().contains("unsupported Markdown benchmark operation")
        );
    }

    #[test]
    fn serves_source_preserving_bash_merge3_without_claiming_merge2() {
        let base = "left() { echo one; }\nright() { echo one; }\n";
        let ours = "left() { echo two; }\nright() { echo one; }\n";
        let theirs = "left() { echo one; }\nright() { echo two; }\n";
        let expected = "left() { echo two; }\nright() { echo two; }\n";
        let merge3 = execute_line(&request_for(
            "merge3",
            "bash",
            "bash",
            &[("base", base), ("ours", ours), ("theirs", theirs)],
        ));
        let merge2 = execute_line(&request_for(
            "merge2",
            "bash",
            "bash",
            &[("incoming", theirs), ("current", ours)],
        ));

        assert_eq!(merge3["status"], 0);
        assert_eq!(
            String::from_utf8(STANDARD.decode(merge3["output_base64"].as_str().unwrap()).unwrap())
                .unwrap(),
            expected
        );
        assert_eq!(merge2["status"], 2);
        assert!(
            merge2["stderr"].as_str().unwrap().contains("unsupported Bash benchmark operation")
        );
    }

    #[test]
    fn serves_source_preserving_rust_merge3_without_claiming_merge2() {
        let base = "fn left() -> i32 { 1 }\n\nfn right() -> i32 { 1 }\n";
        let ours = "fn left() -> i32 { 2 }\n\nfn right() -> i32 { 1 }\n";
        let theirs = "fn left() -> i32 { 1 }\n\nfn right() -> i32 { 2 }\n";
        let expected = "fn left() -> i32 { 2 }\n\nfn right() -> i32 { 2 }\n";
        let merge3 = execute_line(&request_for(
            "merge3",
            "rust",
            "rust",
            &[("base", base), ("ours", ours), ("theirs", theirs)],
        ));
        let merge2 = execute_line(&request_for(
            "merge2",
            "rust",
            "rust",
            &[("incoming", theirs), ("current", ours)],
        ));

        assert_eq!(merge3["status"], 0);
        assert_eq!(
            String::from_utf8(STANDARD.decode(merge3["output_base64"].as_str().unwrap()).unwrap())
                .unwrap(),
            expected
        );
        assert_eq!(merge2["status"], 2);
        assert!(
            merge2["stderr"].as_str().unwrap().contains("unsupported Rust benchmark operation")
        );
    }

    #[test]
    fn serves_source_preserving_go_merge3_without_claiming_merge2() {
        let base =
            "package main\n\nfunc left() int { return 1 }\n\nfunc right() int { return 1 }\n";
        let ours =
            "package main\n\nfunc left() int { return 2 }\n\nfunc right() int { return 1 }\n";
        let theirs =
            "package main\n\nfunc left() int { return 1 }\n\nfunc right() int { return 2 }\n";
        let expected =
            "package main\n\nfunc left() int { return 2 }\n\nfunc right() int { return 2 }\n";
        let merge3 = execute_line(&request_for(
            "merge3",
            "go",
            "go",
            &[("base", base), ("ours", ours), ("theirs", theirs)],
        ));
        let merge2 = execute_line(&request_for(
            "merge2",
            "go",
            "go",
            &[("incoming", theirs), ("current", ours)],
        ));

        assert_eq!(merge3["status"], 0);
        assert_eq!(
            String::from_utf8(STANDARD.decode(merge3["output_base64"].as_str().unwrap()).unwrap())
                .unwrap(),
            expected
        );
        assert_eq!(merge2["status"], 2);
        assert!(merge2["stderr"].as_str().unwrap().contains("unsupported Go benchmark operation"));
    }

    #[test]
    fn serves_source_preserving_typescript_merge3_without_claiming_merge2() {
        let base =
            "function left(): number { return 1; }\nfunction right(): number { return 1; }\n";
        let ours =
            "function left(): number { return 2; }\nfunction right(): number { return 1; }\n";
        let theirs =
            "function left(): number { return 1; }\nfunction right(): number { return 2; }\n";
        let merge3 = execute_line(&request_for(
            "merge3",
            "typescript",
            "typescript",
            &[("base", base), ("ours", ours), ("theirs", theirs)],
        ));
        let merge2 = execute_line(&request_for(
            "merge2",
            "typescript",
            "typescript",
            &[("incoming", theirs), ("current", ours)],
        ));

        assert_eq!(merge3["status"], 0);
        assert_eq!(
            String::from_utf8(STANDARD.decode(merge3["output_base64"].as_str().unwrap()).unwrap())
                .unwrap(),
            "function left(): number { return 2; }\nfunction right(): number { return 2; }\n"
        );
        assert_eq!(merge2["status"], 2);
        assert!(
            merge2["stderr"]
                .as_str()
                .unwrap()
                .contains("unsupported TypeScript benchmark operation")
        );
    }

    #[test]
    fn serves_toml_merge2_without_claiming_toml_merge3() {
        let incoming = "[patient.name]\ngiven = \"Pat\"\nfamily = \"Template\"\n";
        let current = "[patient.name]\nfamily = \"Destination\"\n";
        let merge2 = execute_line(&request_for(
            "merge2",
            "toml",
            "toml",
            &[("incoming", incoming), ("current", current)],
        ));
        let merge3 = execute_line(&request_for(
            "merge3",
            "toml",
            "toml",
            &[("base", "a = 1\n"), ("ours", "a = 2\n"), ("theirs", "a = 3\n")],
        ));

        assert_eq!(merge2["status"], 0);
        assert_eq!(
            String::from_utf8(STANDARD.decode(merge2["output_base64"].as_str().unwrap()).unwrap())
                .unwrap(),
            "[patient.name]\ngiven = \"Pat\"\nfamily = \"Destination\"\n"
        );
        assert_eq!(merge3["status"], 2);
        assert!(
            merge3["stderr"].as_str().unwrap().contains("unsupported TOML benchmark operation")
        );
    }

    #[test]
    fn serves_ruby_merge2_without_claiming_ruby_merge3() {
        let incoming = concat!(
            "class Greeter\n",
            "  def greet(name)\n",
            "    \"Hello #{name}\"\n",
            "  end\n\n",
            "  def wave\n",
            "    :wave\n",
            "  end\n",
            "end\n",
        );
        let current = concat!(
            "class Greeter\n",
            "  def greet(name)\n",
            "    name.upcase\n",
            "  end\n",
            "end\n",
        );
        let expected = concat!(
            "class Greeter\n",
            "  def greet(name)\n",
            "    name.upcase\n",
            "  end\n\n",
            "  def wave\n",
            "    :wave\n",
            "  end\n",
            "end\n",
        );
        let merge2 = execute_line(&request_for(
            "merge2",
            "ruby",
            "ruby",
            &[("incoming", incoming), ("current", current)],
        ));
        let merge3 = execute_line(&request_for(
            "merge3",
            "ruby",
            "ruby",
            &[("base", current), ("ours", current), ("theirs", incoming)],
        ));

        assert_eq!(merge2["status"], 0);
        assert_eq!(
            String::from_utf8(STANDARD.decode(merge2["output_base64"].as_str().unwrap()).unwrap())
                .unwrap(),
            expected
        );
        assert_eq!(merge3["status"], 2);
        assert!(
            merge3["stderr"].as_str().unwrap().contains("unsupported Ruby benchmark operation")
        );
    }

    #[test]
    fn serves_rbs_merge2_without_claiming_rbs_merge3() {
        let incoming = concat!(
            "# shared declaration\n",
            "class Account\n",
            "  def id: () -> String\n",
            "end\n\n",
            "class TemplateOnly\n",
            "end\n",
        );
        let current = concat!(
            "# destination declaration\n",
            "class Account\n",
            "  def id: () -> Integer\n",
            "end\n",
        );
        let expected = concat!(
            "# destination declaration\n",
            "class Account\n",
            "  def id: () -> Integer\n",
            "end\n\n",
            "class TemplateOnly\n",
            "end\n",
        );
        let merge2 = execute_line(&request_for(
            "merge2",
            "rbs",
            "rbs",
            &[("incoming", incoming), ("current", current)],
        ));
        let merge3 = execute_line(&request_for(
            "merge3",
            "rbs",
            "rbs",
            &[("base", current), ("ours", current), ("theirs", incoming)],
        ));

        assert_eq!(merge2["status"], 0);
        assert_eq!(
            String::from_utf8(STANDARD.decode(merge2["output_base64"].as_str().unwrap()).unwrap())
                .unwrap(),
            expected
        );
        assert_eq!(merge3["status"], 2);
        assert!(merge3["stderr"].as_str().unwrap().contains("unsupported RBS benchmark operation"));
    }

    #[test]
    fn diff2_ignores_format_only_changes_but_rejects_malformed_input() {
        let equivalent = execute_line(&request(
            "diff2",
            "json",
            &[("before", "{\"value\":1}"), ("after", "{ \"value\" : 1 }")],
        ));
        let malformed = execute_line(&request(
            "diff2",
            "json",
            &[("before", "{\"value\":1}"), ("after", "{\"value\":")],
        ));

        assert_eq!(equivalent["status"], 0);
        assert_eq!(equivalent["result"]["changes"], serde_json::json!([]));
        assert_eq!(malformed["status"], 2);
        assert!(malformed["stderr"].as_str().unwrap().contains("syntax errors"));
    }

    #[test]
    fn cold_merge2_and_diff_wrappers_use_the_benchmark_file_contract() {
        let directory = tempfile::tempdir().unwrap();
        let incoming =
            write_fixture(directory.path(), "incoming.json", "{\n  \"template\": true\n}\n");
        let current =
            write_fixture(directory.path(), "current.json", "{\n  \"current\": true\n}\n");
        let mut output = Vec::new();
        let mut error = Vec::new();

        let status = run_merge2_files(
            &[incoming, current.clone(), "fixture.json".into()],
            &mut output,
            &mut error,
        );

        assert_eq!(status, 0, "{}", String::from_utf8_lossy(&error));
        let response: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(response["output"], "{\n  \"current\": true,\n  \"template\": true\n}\n");

        output.clear();
        let status = run_diff_files(
            &[current.clone(), current, "fixture.json".into()],
            &mut output,
            &mut error,
        );
        assert_eq!(status, 0, "{}", String::from_utf8_lossy(&error));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&output).unwrap()["changes"],
            serde_json::json!([])
        );
    }

    #[test]
    fn cold_merge3_wrapper_mutates_ours_only_for_clean_results() {
        let directory = tempfile::tempdir().unwrap();
        let base = write_fixture(directory.path(), "base.json", "{\"left\":1,\"right\":1}");
        let ours = write_fixture(directory.path(), "ours.json", "{\"left\":2,\"right\":1}");
        let theirs = write_fixture(directory.path(), "theirs.json", "{\"left\":1,\"right\":2}");
        let mut error = Vec::new();

        let status = run_merge3_files(
            &[base.clone(), ours.clone(), theirs, "fixture.json".into(), "7".into()],
            &mut error,
        );

        assert_eq!(status, 0, "{}", String::from_utf8_lossy(&error));
        assert_eq!(fs::read_to_string(&ours).unwrap(), "{\"left\":2,\"right\":2}");

        fs::write(&ours, "{\"left\":3,\"right\":1}").unwrap();
        let theirs =
            write_fixture(directory.path(), "conflicting.json", "{\"left\":4,\"right\":1}");
        error.clear();
        let status = run_merge3_files(
            &[base, ours.clone(), theirs, "fixture.json".into(), "7".into()],
            &mut error,
        );

        assert_eq!(status, 1);
        assert_eq!(fs::read_to_string(ours).unwrap(), "{\"left\":3,\"right\":1}");
    }

    #[test]
    fn cold_wrapper_diagnostics_use_shared_snake_case_categories() {
        let directory = tempfile::tempdir().unwrap();
        let base = write_fixture(directory.path(), "base.json", "{\"ok\":true}\n");
        let ours = write_fixture(directory.path(), "ours.json", "{\"ok\": tru\n");
        let theirs = write_fixture(directory.path(), "theirs.json", "{\"ok\":false}\n");
        let mut error = Vec::new();

        let status = run_merge3_files(&[base, ours, theirs, "fixture.json".into()], &mut error);

        assert_eq!(status, 2);
        assert!(String::from_utf8(error).unwrap().starts_with("smorg-rs: parse_error:"));
    }
}
