use std::{
    collections::BTreeMap,
    io::{BufRead, Write},
    time::Instant,
};

use ast_merge::ThreeWayMergeOutcome;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use json_merge::{
    JsonDialect, json_semantically_equivalent, merge_json_source_preserving, merge_json_three_way,
};
use serde::Deserialize;

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
    if request.selector.family != "json" {
        return Err(format!("unsupported benchmark family: {}", request.selector.family));
    }
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

    fn request(operation: &str, dialect: &str, sources: &[(&str, &str)]) -> String {
        serde_json::json!({
            "schema_version": REQUEST_SCHEMA,
            "request_id": "request-1",
            "operation": operation,
            "selector": {
                "provider_id": "ruby.json",
                "family": "json",
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
}
