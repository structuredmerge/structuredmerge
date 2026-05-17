use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use ast_merge::{
    MergeResult, ProfilePromotionEvaluation, ProfilePromotionStatus,
    ProfileSelectionEnforcementMode, ProfileSelectionRequirement,
    evaluate_profile_selection_requirement, initial_profile_promotion_policy,
};
use ast_merge_git::{Merge3Request, merge3};
use go_merge::{GoDialect, merge_go};
use json_merge::{JsonDialect, merge_json};
use plain_merge::merge_text;

const EXIT_SUCCESS: i32 = 0;
const EXIT_UNRESOLVED_CONFLICT: i32 = 1;
const EXIT_USER_ERROR: i32 = 2;
const EXIT_INTERNAL_ERROR: i32 = 3;

#[derive(Debug)]
struct MergeDriverOptions {
    ancestor: String,
    current: String,
    other: String,
    path_name: Option<String>,
    output: Option<String>,
    strict: bool,
    fallback: String,
    check_only: bool,
    exit_code: bool,
    profile_id: Option<String>,
    profile_report: bool,
    require_profile_status: Option<String>,
}

#[derive(Debug, Default)]
struct PathSettings {
    language: Option<String>,
    conflict_marker_size: usize,
    profile_id: Option<String>,
    require_profile_status: Option<String>,
}

#[derive(Debug)]
struct DiffDriverOptions {
    path_name: Option<String>,
    old_path: String,
    new_path: String,
}

#[derive(Debug)]
struct ConflictDiffOptions {
    path_name: Option<String>,
    file_path: String,
    exit_code: bool,
}

#[derive(Debug, Eq, PartialEq)]
struct ConflictRegion {
    start_line: usize,
    separator_line: usize,
    end_line: usize,
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    std::process::exit(run(&args, &mut stdout, &mut stderr));
}

fn run(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let Some(command) = args.first() else {
        print_usage(stderr);
        return EXIT_USER_ERROR;
    };

    match command.as_str() {
        "merge-driver" => run_merge_driver(&args[1..], stdout, stderr),
        "diff-driver" => run_diff_driver(&args[1..], stdout, stderr),
        "conflicts" => run_conflicts(&args[1..], stdout, stderr),
        "languages" => run_languages(&args[1..], stdout, stderr),
        "help" | "-h" | "--help" => {
            print_usage(stdout);
            EXIT_SUCCESS
        }
        _ => {
            let _ = writeln!(stderr, "unknown command {command:?}");
            print_usage(stderr);
            EXIT_USER_ERROR
        }
    }
}

fn print_usage(out: &mut dyn Write) {
    let _ = writeln!(
        out,
        "usage: smorg-rs merge-driver [--path-name PATH] [--output PATH] [--strict] [--fallback=none|line|local|full-file] %O %A %B [%P]"
    );
    let _ = writeln!(
        out,
        "       smorg-rs merge-driver --ancestor %O --current %A --other %B --path-name %P"
    );
    let _ = writeln!(out, "       smorg-rs diff-driver [--path-name PATH] OLD NEW");
    let _ = writeln!(
        out,
        "       smorg-rs diff-driver PATH OLD-FILE OLD-HEX OLD-MODE NEW-FILE NEW-HEX NEW-MODE [OLD-PREFIX NEW-PREFIX]"
    );
    let _ = writeln!(out, "       smorg-rs conflicts diff [--path-name PATH] [--exit-code] FILE");
    let _ = writeln!(out, "       smorg-rs languages --gitattributes");
}

fn run_merge_driver(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let Some(options) = parse_merge_driver_options(args, stderr) else {
        return EXIT_USER_ERROR;
    };
    let ancestor_source = match fs::read_to_string(&options.ancestor) {
        Ok(source) => source,
        Err(error) => {
            let _ = writeln!(stderr, "read ancestor: {error}");
            return EXIT_USER_ERROR;
        }
    };
    let current_source = match fs::read_to_string(&options.current) {
        Ok(source) => source,
        Err(error) => {
            let _ = writeln!(stderr, "read current: {error}");
            return EXIT_USER_ERROR;
        }
    };
    let other_source = match fs::read_to_string(&options.other) {
        Ok(source) => source,
        Err(error) => {
            let _ = writeln!(stderr, "read other: {error}");
            return EXIT_USER_ERROR;
        }
    };

    let effective_path = options.effective_path();
    let settings = load_path_settings(&effective_path);
    let profile_id = options.profile_id.as_deref().or(settings.profile_id.as_deref());
    let require_profile_status =
        options.require_profile_status.as_deref().or(settings.require_profile_status.as_deref());
    let profile_exit = report_and_enforce_profile(
        profile_id,
        options.profile_report,
        require_profile_status,
        stdout,
        stderr,
    );
    if profile_exit != EXIT_SUCCESS {
        return profile_exit;
    }
    let mut result = merge_by_path(
        &effective_path,
        settings.language.as_deref(),
        &ancestor_source,
        &current_source,
        &other_source,
    );
    if !result.ok || result.output.is_none() {
        if options.strict || options.fallback == "none" {
            print_diagnostics(stderr, &result);
            return EXIT_UNRESOLVED_CONFLICT;
        }
        result.output = Some(current_source.clone());
    }

    let output = result.output.expect("fallback should provide output");
    if options.check_only {
        if options.exit_code && output != current_source {
            return EXIT_UNRESOLVED_CONFLICT;
        }
        return EXIT_SUCCESS;
    }

    let output_path = options.output.as_deref().unwrap_or(&options.current);
    if let Err(error) = fs::write(output_path, output) {
        let _ = writeln!(stderr, "write output: {error}");
        return EXIT_INTERNAL_ERROR;
    }

    EXIT_SUCCESS
}

fn parse_merge_driver_options(
    args: &[String],
    stderr: &mut dyn Write,
) -> Option<MergeDriverOptions> {
    let mut ancestor = None;
    let mut current = None;
    let mut other = None;
    let mut path_name = None;
    let mut output = None;
    let mut strict = false;
    let mut fallback = "full-file".to_string();
    let mut check_only = false;
    let mut exit_code = false;
    let mut profile_id = None;
    let mut profile_report = false;
    let mut require_profile_status = None;
    let mut positionals = Vec::new();

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--ancestor" => {
                index += 1;
                ancestor = args.get(index).cloned();
            }
            "--current" => {
                index += 1;
                current = args.get(index).cloned();
            }
            "--other" => {
                index += 1;
                other = args.get(index).cloned();
            }
            "--path-name" => {
                index += 1;
                path_name = args.get(index).cloned();
            }
            "--output" => {
                index += 1;
                output = args.get(index).cloned();
            }
            "--strict" => strict = true,
            "--check-only" => check_only = true,
            "--exit-code" => exit_code = true,
            "--profile" => {
                index += 1;
                profile_id = args.get(index).cloned();
            }
            "--profile-report" => profile_report = true,
            "--require-profile-status" => {
                index += 1;
                require_profile_status = args.get(index).cloned();
            }
            value if value.starts_with("--fallback=") => {
                fallback = value.trim_start_matches("--fallback=").to_string();
            }
            "--fallback" => {
                index += 1;
                fallback = args.get(index).cloned().unwrap_or_default();
            }
            value if value.starts_with("--") => {
                let _ = writeln!(stderr, "unknown merge-driver option {value:?}");
                return None;
            }
            value => positionals.push(value.to_string()),
        }
        index += 1;
    }

    ancestor = ancestor.or_else(|| positionals.first().cloned());
    current = current.or_else(|| positionals.get(1).cloned());
    other = other.or_else(|| positionals.get(2).cloned());
    path_name = path_name.or_else(|| positionals.get(3).cloned());

    if !["none", "line", "local", "full-file"].contains(&fallback.as_str()) {
        let _ = writeln!(stderr, "unsupported fallback mode {fallback:?}");
        return None;
    }

    Some(MergeDriverOptions {
        ancestor: ancestor?,
        current: current?,
        other: other?,
        path_name,
        output,
        strict,
        fallback,
        check_only,
        exit_code,
        profile_id,
        profile_report,
        require_profile_status,
    })
}

fn report_and_enforce_profile(
    profile_id: Option<&str>,
    profile_report: bool,
    require_status: Option<&str>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    if profile_id.is_none() && require_status.is_none() && !profile_report {
        return EXIT_SUCCESS;
    }
    let profile_id = profile_id.unwrap_or(ast_merge::PROMOTION_PROFILE_JSON_KEYED_OBJECT);
    let evaluation = ProfilePromotionEvaluation {
        profile_id: profile_id.to_string(),
        status: ProfilePromotionStatus::Available,
        blocking_reasons: vec![
            "profile promotion evidence is not loaded by this CLI command".to_string(),
        ],
        diagnostics: vec![],
    };
    let minimum_profile_status = match require_status {
        Some("recommended") => ProfilePromotionStatus::Recommended,
        Some("default") => ProfilePromotionStatus::Default,
        Some(_) | None => ProfilePromotionStatus::Available,
    };
    let requirement = ProfileSelectionRequirement {
        profile_id: profile_id.to_string(),
        promotion_policy_id: initial_profile_promotion_policy().policy_id,
        minimum_profile_status,
        enforcement_mode: if require_status.is_some() {
            ProfileSelectionEnforcementMode::Required
        } else {
            ProfileSelectionEnforcementMode::Advisory
        },
    };
    let decision = evaluate_profile_selection_requirement(&requirement, None, &evaluation);
    if profile_report {
        let _ = writeln!(stdout, "{}", serde_json::to_string(&decision).unwrap());
    }
    if !decision.allowed {
        let _ = writeln!(stderr, "{}", decision.blocking_reasons[0]);
        return EXIT_USER_ERROR;
    }
    EXIT_SUCCESS
}

fn run_languages(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    if args != ["--gitattributes"] {
        let _ = writeln!(stderr, "languages currently requires --gitattributes");
        return EXIT_USER_ERROR;
    }
    for line in [
        "*.go merge=smorg-rs diff=smorg-rs smorg.language=go",
        "*.json merge=smorg-rs diff=smorg-rs smorg.language=json",
        "*.jsonc merge=smorg-rs diff=smorg-rs smorg.language=jsonc",
    ] {
        let _ = writeln!(stdout, "{line}");
    }
    EXIT_SUCCESS
}

fn run_diff_driver(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let Some(options) = parse_diff_driver_options(args, stderr) else {
        return EXIT_USER_ERROR;
    };

    let old_source = match fs::read_to_string(&options.old_path) {
        Ok(source) => source,
        Err(error) => {
            let _ = writeln!(stderr, "read old file: {error}");
            return EXIT_USER_ERROR;
        }
    };
    let new_source = match fs::read_to_string(&options.new_path) {
        Ok(source) => source,
        Err(error) => {
            let _ = writeln!(stderr, "read new file: {error}");
            return EXIT_USER_ERROR;
        }
    };

    print_structured_diff(stdout, &options.effective_path(), &old_source, &new_source);
    EXIT_SUCCESS
}

fn parse_diff_driver_options(args: &[String], stderr: &mut dyn Write) -> Option<DiffDriverOptions> {
    let mut path_name = None;
    let mut positionals = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--path-name" => {
                index += 1;
                path_name = args.get(index).cloned();
            }
            value if value.starts_with("--") => {
                let _ = writeln!(stderr, "unknown diff-driver option {value:?}");
                return None;
            }
            value => positionals.push(value.to_string()),
        }
        index += 1;
    }

    match positionals.len() {
        2 => Some(DiffDriverOptions {
            path_name,
            old_path: positionals[0].clone(),
            new_path: positionals[1].clone(),
        }),
        7 | 9 => Some(DiffDriverOptions {
            path_name: path_name.or_else(|| Some(positionals[0].clone())),
            old_path: positionals[1].clone(),
            new_path: positionals[4].clone(),
        }),
        _ => {
            let _ = writeln!(stderr, "diff-driver requires either 2, 7, or 9 positional arguments");
            None
        }
    }
}

impl DiffDriverOptions {
    fn effective_path(&self) -> String {
        self.path_name.clone().unwrap_or_else(|| self.new_path.clone())
    }
}

fn print_structured_diff(
    stdout: &mut dyn Write,
    path_name: &str,
    old_source: &str,
    new_source: &str,
) {
    let _ = writeln!(stdout, "structured-diff {path_name}");
    if old_source == new_source {
        let _ = writeln!(stdout, "status unchanged");
        return;
    }
    let _ = writeln!(stdout, "status changed");
    let _ = writeln!(stdout, "old-lines {}", line_count(old_source));
    let _ = writeln!(stdout, "new-lines {}", line_count(new_source));
}

fn line_count(source: &str) -> usize {
    if source.is_empty() {
        0
    } else if source.ends_with('\n') {
        source.matches('\n').count()
    } else {
        source.matches('\n').count() + 1
    }
}

fn run_conflicts(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let Some(subcommand) = args.first() else {
        let _ = writeln!(stderr, "conflicts requires a subcommand");
        return EXIT_USER_ERROR;
    };
    match subcommand.as_str() {
        "diff" => run_conflicts_diff(&args[1..], stdout, stderr),
        _ => {
            let _ = writeln!(stderr, "unknown conflicts subcommand {subcommand:?}");
            EXIT_USER_ERROR
        }
    }
}

fn run_conflicts_diff(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let Some(options) = parse_conflicts_diff_options(args, stderr) else {
        return EXIT_USER_ERROR;
    };
    let source = match fs::read_to_string(&options.file_path) {
        Ok(source) => source,
        Err(error) => {
            let _ = writeln!(stderr, "read conflicted file: {error}");
            return EXIT_USER_ERROR;
        }
    };
    let effective_path = options.path_name.clone().unwrap_or_else(|| options.file_path.clone());
    let settings = load_path_settings(&effective_path);
    let regions = find_conflict_regions(&source, settings.conflict_marker_size);
    print_conflict_diff(stdout, &effective_path, &regions);
    if options.exit_code && !regions.is_empty() {
        return EXIT_UNRESOLVED_CONFLICT;
    }
    EXIT_SUCCESS
}

fn parse_conflicts_diff_options(
    args: &[String],
    stderr: &mut dyn Write,
) -> Option<ConflictDiffOptions> {
    let mut path_name = None;
    let mut exit_code = false;
    let mut positionals = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--path-name" => {
                index += 1;
                path_name = args.get(index).cloned();
            }
            "--exit-code" => exit_code = true,
            value if value.starts_with("--") => {
                let _ = writeln!(stderr, "unknown conflicts diff option {value:?}");
                return None;
            }
            value => positionals.push(value.to_string()),
        }
        index += 1;
    }

    if positionals.len() != 1 {
        let _ = writeln!(stderr, "conflicts diff requires exactly one file path");
        return None;
    }
    Some(ConflictDiffOptions { path_name, file_path: positionals[0].clone(), exit_code })
}

fn find_conflict_regions(source: &str, marker_size: usize) -> Vec<ConflictRegion> {
    let marker_size = marker_size.max(1);
    let start_prefix = "<".repeat(marker_size);
    let separator_prefix = "=".repeat(marker_size);
    let end_prefix = ">".repeat(marker_size);
    let mut regions = Vec::new();
    let mut current: Option<ConflictRegion> = None;

    for (index, line) in source.split('\n').enumerate() {
        let line_number = index + 1;
        if line.starts_with(&start_prefix) {
            current =
                Some(ConflictRegion { start_line: line_number, separator_line: 0, end_line: 0 });
        } else if line.starts_with(&separator_prefix) {
            if let Some(region) = current.as_mut() {
                if region.separator_line == 0 {
                    region.separator_line = line_number;
                }
            }
        } else if line.starts_with(&end_prefix) {
            if let Some(mut region) = current.take() {
                region.end_line = line_number;
                regions.push(region);
            }
        }
    }
    regions
}

fn print_conflict_diff(stdout: &mut dyn Write, path_name: &str, regions: &[ConflictRegion]) {
    let _ = writeln!(stdout, "conflicts {path_name}");
    let _ = writeln!(stdout, "count {}", regions.len());
    for (index, region) in regions.iter().enumerate() {
        let _ = writeln!(
            stdout,
            "conflict {} lines {}-{} separator {}",
            index + 1,
            region.start_line,
            region.end_line,
            region.separator_line
        );
    }
}

impl MergeDriverOptions {
    fn effective_path(&self) -> String {
        self.path_name.clone().unwrap_or_else(|| self.current.clone())
    }
}

fn merge_by_path(
    path_name: &str,
    language: Option<&str>,
    ancestor_source: &str,
    current_source: &str,
    other_source: &str,
) -> MergeResult<String> {
    match normalize_language(language, path_name).as_str() {
        "go" => merge_go(other_source, current_source, GoDialect::Go),
        "json" => merge3_result(merge3(&Merge3Request {
            base_source: ancestor_source.to_string(),
            ours_source: current_source.to_string(),
            theirs_source: other_source.to_string(),
            path_name: Some(path_name.to_string()),
            language: Some("json".to_string()),
            dialect: Some("json".to_string()),
            profile_id: Some("json.keyed-object".to_string()),
            fallback_policy: Some("none".to_string()),
            conflict_marker_size: None,
            render_policy: Some("canonical".to_string()),
        })),
        "jsonc" => merge_json(other_source, current_source, JsonDialect::Jsonc),
        _ => merge_text(other_source, current_source),
    }
}

fn merge3_result(result: ast_merge_git::Merge3Response) -> MergeResult<String> {
    if result.ok {
        MergeResult {
            ok: true,
            diagnostics: result.diagnostics,
            output: result.merged_source,
            policies: vec![],
        }
    } else {
        MergeResult { ok: false, diagnostics: result.diagnostics, output: None, policies: vec![] }
    }
}

fn normalize_language(language: Option<&str>, path_name: &str) -> String {
    match language.unwrap_or_default().trim().to_ascii_lowercase().as_str() {
        "go" | "golang" => return "go".to_string(),
        "json" => return "json".to_string(),
        "jsonc" | "json with comments" => return "jsonc".to_string(),
        "plain" | "text" | "plaintext" | "text/plain" => return "text".to_string(),
        _ => {}
    }

    match Path::new(path_name).extension().and_then(|extension| extension.to_str()) {
        Some("go") => "go".to_string(),
        Some("json") => "json".to_string(),
        Some("jsonc") => "jsonc".to_string(),
        _ => "text".to_string(),
    }
}

fn load_path_settings(path_name: &str) -> PathSettings {
    let mut settings = PathSettings {
        language: None,
        conflict_marker_size: 7,
        profile_id: None,
        require_profile_status: None,
    };
    for attributes_path in attribute_files_for_path(path_name) {
        let Ok(source) = fs::read_to_string(attributes_path) else {
            continue;
        };
        apply_attributes(&mut settings, path_name, &source);
    }
    settings
}

fn attribute_files_for_path(path_name: &str) -> Vec<PathBuf> {
    let clean_path = PathBuf::from(path_name);
    let Some(parent) = clean_path.parent() else {
        return vec![PathBuf::from(".gitattributes")];
    };
    if parent.as_os_str().is_empty() || parent.is_absolute() || path_name.starts_with("..") {
        return vec![PathBuf::from(".gitattributes")];
    }

    let mut files = vec![PathBuf::from(".gitattributes")];
    let mut current = PathBuf::new();
    for component in parent.components() {
        current.push(component.as_os_str());
        files.push(current.join(".gitattributes"));
    }
    files
}

fn apply_attributes(settings: &mut PathSettings, path_name: &str, source: &str) {
    for raw_line in source.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 2 || !attribute_pattern_matches(fields[0], path_name) {
            continue;
        }
        for field in fields.iter().skip(1) {
            let Some((key, value)) = field.split_once('=') else {
                continue;
            };
            match key {
                "smorg.language" | "linguist-language" => {
                    settings.language = Some(value.to_string());
                }
                "smorg.profile" => {
                    settings.profile_id = Some(value.to_string());
                }
                "smorg.requireProfileStatus" => {
                    settings.require_profile_status = Some(value.to_string());
                }
                "conflict-marker-size" => {
                    if let Ok(marker_size) = value.parse::<usize>() {
                        if marker_size > 0 {
                            settings.conflict_marker_size = marker_size;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn attribute_pattern_matches(pattern: &str, path_name: &str) -> bool {
    if pattern == path_name {
        return true;
    }
    if !pattern.contains('/') {
        return Path::new(path_name)
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| simple_glob_matches(pattern, name));
    }
    simple_glob_matches(pattern, path_name)
}

fn simple_glob_matches(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some((prefix, suffix)) = pattern.split_once('*') {
        return value.starts_with(prefix) && value.ends_with(suffix);
    }
    pattern == value
}

fn print_diagnostics(stderr: &mut dyn Write, result: &MergeResult<String>) {
    for diagnostic in &result.diagnostics {
        let _ = writeln!(stderr, "{:?}: {}", diagnostic.category, diagnostic.message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::process::Command;
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

    struct TestDir {
        path: PathBuf,
        previous: PathBuf,
        _guard: MutexGuard<'static, ()>,
    }

    impl TestDir {
        fn new() -> Self {
            let guard = TEST_MUTEX.get_or_init(|| Mutex::new(())).lock().expect("test mutex");
            let previous = env::current_dir().expect("current dir");
            let unique =
                SystemTime::now().duration_since(UNIX_EPOCH).expect("system clock").as_nanos();
            let path = env::temp_dir().join(format!("smorg-rs-test-{unique}"));
            fs::create_dir_all(&path).expect("create temp dir");
            env::set_current_dir(&path).expect("chdir temp");
            Self { path, previous, _guard: guard }
        }

        fn write(&self, name: &str, source: &str) -> String {
            let path = self.path.join(name);
            fs::write(&path, source).expect("write test file");
            path.to_string_lossy().to_string()
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            env::set_current_dir(&self.previous).expect("restore current dir");
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn merge_driver_updates_current_file() {
        let dir = TestDir::new();
        let ancestor = dir.write("ancestor.json", r#"{"name":"structuredmerge"}"#);
        let current = dir.write("current.tmp", r#"{"name":"structuredmerge","current":true}"#);
        let other = dir.write("other.tmp", r#"{"name":"structuredmerge","other":true}"#);

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run(
            &[
                "merge-driver".to_string(),
                "--path-name".to_string(),
                "package.json".to_string(),
                ancestor,
                current.clone(),
                other,
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(exit, EXIT_SUCCESS, "stderr={}", String::from_utf8_lossy(&stderr));
        let merged = fs::read_to_string(current).expect("read current file");
        assert!(merged.contains(r#""current":true"#), "{merged}");
        assert!(merged.contains(r#""other":true"#), "{merged}");
        assert!(stdout.is_empty(), "merge-driver should keep stdout quiet");
    }

    #[test]
    fn merge_driver_uses_smorg_language_attribute() {
        let dir = TestDir::new();
        fs::write(".gitattributes", "*.data smorg.language=json\n").expect("write attributes");
        let ancestor = dir.write("ancestor.tmp", r#"{"name":"structuredmerge"}"#);
        let current = dir.write("current.tmp", r#"{"name":"structuredmerge","current":true}"#);
        let other = dir.write("other.tmp", r#"{"name":"structuredmerge","other":true}"#);

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run(
            &[
                "merge-driver".to_string(),
                ancestor,
                current.clone(),
                other,
                "package.data".to_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(exit, EXIT_SUCCESS, "stderr={}", String::from_utf8_lossy(&stderr));
        let merged = fs::read_to_string(current).expect("read current file");
        assert!(merged.contains(r#""current":true"#), "{merged}");
        assert!(merged.contains(r#""other":true"#), "{merged}");
    }

    #[test]
    fn strict_failure_returns_conflict_exit_code() {
        let dir = TestDir::new();
        let ancestor = dir.write("ancestor.json", r#"{"name":"structuredmerge"}"#);
        let current = dir.write("current.json", r#"{"name":"#);
        let other = dir.write("other.json", r#"{"other":true}"#);

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run(
            &[
                "merge-driver".to_string(),
                "--strict".to_string(),
                ancestor,
                current,
                other,
                "package.json".to_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(exit, EXIT_UNRESOLVED_CONFLICT);
        assert!(
            String::from_utf8_lossy(&stderr).contains("ParseError"),
            "stderr={}",
            String::from_utf8_lossy(&stderr)
        );
        assert!(
            String::from_utf8_lossy(&stderr).contains("ours parse error"),
            "stderr={}",
            String::from_utf8_lossy(&stderr)
        );
    }

    #[test]
    fn merge_driver_uses_ancestor_for_json_same_key_conflicts() {
        let dir = TestDir::new();
        let ancestor = dir.write("ancestor.json", r#"{"name":"structuredmerge"}"#);
        let current = dir.write("current.json", r#"{"name":"ours"}"#);
        let other = dir.write("other.json", r#"{"name":"theirs"}"#);

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run(
            &[
                "merge-driver".to_string(),
                "--strict".to_string(),
                ancestor,
                current.clone(),
                other,
                "package.json".to_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(exit, EXIT_UNRESOLVED_CONFLICT);
        assert!(
            String::from_utf8_lossy(&stderr).contains("merge_conflict"),
            "stderr={}",
            String::from_utf8_lossy(&stderr)
        );
        assert_eq!(fs::read_to_string(current).expect("read current"), r#"{"name":"ours"}"#);
    }

    #[test]
    fn merge_driver_conforms_to_git_driver_json_integration_fixture() {
        if Command::new("git").arg("--version").output().is_err() {
            return;
        }
        let fixture = read_git_driver_json_fixture();
        let cases = fixture["cases"].as_array().expect("fixture cases should be an array");
        for case in cases {
            let dir = TestDir::new();
            run_git(&dir.path, &["init"]);
            run_git(&dir.path, &["config", "user.email", "smorg-rs@example.invalid"]);
            run_git(&dir.path, &["config", "user.name", "smorg-rs test"]);
            dir.write(".gitattributes", "*.json merge=smorg-rs smorg.language=json\n");
            let path_name = case["path_name"].as_str().expect("path_name should be a string");
            dir.write(
                path_name,
                case["base_source"].as_str().expect("base_source should be a string"),
            );
            run_git(&dir.path, &["add", "."]);
            run_git(&dir.path, &["commit", "-m", "base"]);

            let ancestor =
                dir.write("ancestor.tmp", case["base_source"].as_str().expect("base source"));
            let current = dir.write(path_name, case["ours_source"].as_str().expect("ours source"));
            let other =
                dir.write("other.tmp", case["theirs_source"].as_str().expect("theirs source"));
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();

            let exit = run(
                &[
                    "merge-driver".to_string(),
                    "--strict".to_string(),
                    ancestor,
                    current.clone(),
                    other,
                    path_name.to_string(),
                ],
                &mut stdout,
                &mut stderr,
            );
            let expected = &case["expected"];
            assert_eq!(
                exit,
                expected["exit_code"].as_i64().expect("exit_code should be an integer") as i32,
                "case={} stderr={}",
                case["case_id"].as_str().unwrap_or_default(),
                String::from_utf8_lossy(&stderr)
            );
            for needle in expected["stderr_contains"].as_array().expect("stderr_contains array") {
                let needle = needle.as_str().expect("stderr needle should be a string");
                assert!(
                    String::from_utf8_lossy(&stderr).contains(needle),
                    "case={} stderr={}",
                    case["case_id"].as_str().unwrap_or_default(),
                    String::from_utf8_lossy(&stderr)
                );
            }

            let merged_source = fs::read_to_string(&current).expect("read current");
            if let Some(merged_json) = expected.get("merged_json") {
                let merged: Value =
                    serde_json::from_str(&merged_source).expect("merged output should parse");
                assert_eq!(
                    &merged,
                    merged_json,
                    "case={}",
                    case["case_id"].as_str().unwrap_or_default()
                );
            } else if let Some(expected_source) =
                expected.get("merged_source").and_then(|source| source.as_str())
            {
                assert_eq!(
                    merged_source,
                    expected_source,
                    "case={}",
                    case["case_id"].as_str().unwrap_or_default()
                );
            }
        }
    }

    fn read_git_driver_json_fixture() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
            "../../../fixtures/diagnostics/slice-951-git-driver-json-integration/git-driver-json-integration.json",
        );
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read fixture {}: {error}", path.display()));
        serde_json::from_str(&source).expect("fixture should parse")
    }

    fn run_git(dir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .expect("git should run");
        assert!(
            output.status.success(),
            "git {} failed:\n{}{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn check_only_exit_code_reports_pending_change_without_writing() {
        let dir = TestDir::new();
        let ancestor = dir.write("ancestor.json", r#"{"name":"structuredmerge"}"#);
        let current = dir.write("current.json", r#"{"name":"structuredmerge","current":true}"#);
        let other = dir.write("other.json", r#"{"name":"structuredmerge","other":true}"#);

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run(
            &[
                "merge-driver".to_string(),
                "--check-only".to_string(),
                "--exit-code".to_string(),
                ancestor,
                current.clone(),
                other,
                "package.json".to_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(exit, EXIT_UNRESOLVED_CONFLICT);
        let current_source = fs::read_to_string(current).expect("read current file");
        assert!(!current_source.contains(r#""other":true"#), "{current_source}");
    }

    #[test]
    fn profile_report_and_required_status_blocks_merge() {
        let dir = TestDir::new();
        let ancestor = dir.write("ancestor.json", r#"{"name":"structuredmerge"}"#);
        let current = dir.write("current.json", r#"{"name":"structuredmerge","current":true}"#);
        let other = dir.write("other.json", r#"{"name":"structuredmerge","other":true}"#);

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run(
            &[
                "merge-driver".to_string(),
                "--profile".to_string(),
                "json.keyed-object".to_string(),
                "--profile-report".to_string(),
                "--require-profile-status".to_string(),
                "recommended".to_string(),
                ancestor,
                current,
                other,
                "package.json".to_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(exit, EXIT_USER_ERROR);
        assert!(
            String::from_utf8_lossy(&stdout).contains(r#""rejection_code":"profile_status_unmet""#)
        );
        assert!(
            String::from_utf8_lossy(&stderr)
                .contains("profile status available is below required recommended")
        );
    }

    #[test]
    fn merge_driver_uses_smorg_profile_attributes() {
        let dir = TestDir::new();
        fs::write(
            ".gitattributes",
            "*.json smorg.profile=json.keyed-object smorg.requireProfileStatus=recommended\n",
        )
        .expect("write attributes");
        let ancestor = dir.write("ancestor.json", r#"{"name":"structuredmerge"}"#);
        let current = dir.write("current.json", r#"{"name":"structuredmerge","current":true}"#);
        let other = dir.write("other.json", r#"{"name":"structuredmerge","other":true}"#);

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run(
            &[
                "merge-driver".to_string(),
                "--profile-report".to_string(),
                ancestor,
                current,
                other,
                "package.json".to_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(exit, EXIT_USER_ERROR);
        let report = String::from_utf8_lossy(&stdout);
        assert!(report.contains(r#""profile_id":"json.keyed-object""#), "{report}");
        assert!(report.contains(r#""rejection_code":"profile_status_unmet""#), "{report}");
    }

    #[test]
    fn languages_prints_gitattributes() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run(
            &["languages".to_string(), "--gitattributes".to_string()],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(exit, EXIT_SUCCESS, "stderr={}", String::from_utf8_lossy(&stderr));
        let output = String::from_utf8(stdout).expect("utf8 output");
        assert!(output.contains("*.go merge=smorg-rs diff=smorg-rs smorg.language=go"));
        assert!(output.contains("*.json merge=smorg-rs diff=smorg-rs smorg.language=json"));
    }

    #[test]
    fn diff_driver_accepts_two_argument_form() {
        let dir = TestDir::new();
        let old_path = dir.write("old.go", "package main\n\nfunc Old() {}\n");
        let new_path = dir.write("new.go", "package main\n\nfunc New() {}\n");

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run(
            &[
                "diff-driver".to_string(),
                "--path-name".to_string(),
                "main.go".to_string(),
                old_path,
                new_path,
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(exit, EXIT_SUCCESS, "stderr={}", String::from_utf8_lossy(&stderr));
        let output = String::from_utf8(stdout).expect("utf8 output");
        assert!(output.contains("structured-diff main.go"), "{output}");
        assert!(output.contains("status changed"), "{output}");
    }

    #[test]
    fn diff_driver_accepts_git_external_diff_forms() {
        for argument_count in [7, 9] {
            let dir = TestDir::new();
            let old_path = dir.write("old.json", r#"{"old":true}"#);
            let new_path = dir.write("new.json", r#"{"new":true}"#);
            let mut args = vec![
                "diff-driver".to_string(),
                "package.json".to_string(),
                old_path,
                "abc123".to_string(),
                "100644".to_string(),
                new_path,
                "def456".to_string(),
                "100644".to_string(),
            ];
            if argument_count == 9 {
                args.push("a/".to_string());
                args.push("b/".to_string());
            }

            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let exit = run(&args, &mut stdout, &mut stderr);
            assert_eq!(exit, EXIT_SUCCESS, "stderr={}", String::from_utf8_lossy(&stderr));
            let output = String::from_utf8(stdout).expect("utf8 output");
            assert!(output.contains("structured-diff package.json"), "{output}");
        }
    }

    #[test]
    fn conflicts_diff_reports_regions_and_exit_code() {
        let dir = TestDir::new();
        let conflicted = dir.write(
            "conflicted.go",
            "package main\n<<<<<<< ours\nfunc Current() {}\n=======\nfunc Other() {}\n>>>>>>> theirs\n",
        );

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run(
            &[
                "conflicts".to_string(),
                "diff".to_string(),
                "--path-name".to_string(),
                "main.go".to_string(),
                "--exit-code".to_string(),
                conflicted,
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(exit, EXIT_UNRESOLVED_CONFLICT);
        let output = String::from_utf8(stdout).expect("utf8 output");
        assert!(output.contains("conflicts main.go"), "{output}");
        assert!(output.contains("count 1"), "{output}");
        assert!(output.contains("conflict 1 lines 2-6 separator 4"), "{output}");
    }

    #[test]
    fn conflicts_diff_uses_conflict_marker_size_attribute() {
        let dir = TestDir::new();
        fs::write(".gitattributes", "*.go conflict-marker-size=9\n").expect("write attributes");
        let conflicted =
            dir.write("conflicted.go", "<<<<<<<<< ours\nx\n=========\ny\n>>>>>>>>> theirs\n");

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run(
            &[
                "conflicts".to_string(),
                "diff".to_string(),
                "--path-name".to_string(),
                "conflicted.go".to_string(),
                conflicted,
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(exit, EXIT_SUCCESS, "stderr={}", String::from_utf8_lossy(&stderr));
        let output = String::from_utf8(stdout).expect("utf8 output");
        assert!(output.contains("count 1"), "{output}");
    }
}
