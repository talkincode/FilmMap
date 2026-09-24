use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const CATALOG: &str = include_str!("../capabilities/catalog.json");
const SKILL: &str = include_str!("../skills/filmmap/SKILL.md");
const TOOL_MANIFEST: &str = include_str!("../tools/manifest.json");

#[derive(Parser)]
#[command(
    name = "filmmap",
    version,
    about = "Build evidence-backed media indexes for editing agents"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a FilmMap workspace with a starter profile and JSONL index.
    Init { path: Option<PathBuf> },
    /// List the stable capability vocabulary.
    Capability {
        #[command(subcommand)]
        command: CapabilityCommands,
    },
    /// Inspect or validate a machine-specific capability profile.
    Profile {
        #[command(subcommand)]
        command: ProfileCommands,
    },
    /// Compare a requirement with a profile and report executable/missing work.
    Plan {
        #[arg(long)]
        requirement: PathBuf,
        #[arg(long)]
        profile: PathBuf,
    },
    /// Add a media file to the index with a content hash and basic file evidence.
    Artifact {
        #[command(subcommand)]
        command: ArtifactCommands,
    },
    /// Append an explicit observation to a JSONL evidence index.
    Observe {
        #[command(subcommand)]
        command: ObserveCommands,
    },
    /// Check JSONL records and stable IDs for basic integrity.
    Validate { index: PathBuf },
    /// Search indexed records by case-insensitive text and optional time range.
    Query {
        index: PathBuf,
        query: String,
        #[arg(long)]
        from: Option<f64>,
        #[arg(long)]
        to: Option<f64>,
    },
    /// Install the bundled Agent skill or extension scripts.
    Install {
        #[command(subcommand)]
        command: InstallCommands,
    },
}
#[derive(Subcommand)]
enum CapabilityCommands {
    List,
}
#[derive(Subcommand)]
enum ProfileCommands {
    Detect {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Validate {
        path: PathBuf,
    },
}
#[derive(Subcommand)]
enum ArtifactCommands {
    Add {
        index: PathBuf,
        path: PathBuf,
        #[arg(long, default_value = "video")]
        kind: String,
    },
}
#[derive(Subcommand)]
enum ObserveCommands {
    Add {
        index: PathBuf,
        artifact_id: String,
        #[arg(long)]
        capability: String,
        #[arg(long)]
        at: Option<f64>,
        #[arg(long)]
        value: String,
        #[arg(long, default_value = "agent")]
        source: String,
        #[arg(long)]
        confidence: Option<f64>,
    },
}
#[derive(Subcommand)]
enum InstallCommands {
    Skill {
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    Tools {
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}

#[derive(Serialize, Deserialize)]
struct Profile {
    schema_version: String,
    profile_id: String,
    generated_at: String,
    capabilities: Vec<CapabilityState>,
}
#[derive(Serialize, Deserialize)]
struct CapabilityState {
    id: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    executable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

fn main() -> Result<()> {
    run(Cli::parse())
}
fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init { path } => init(path.unwrap_or_else(|| PathBuf::from("."))),
        Commands::Capability {
            command: CapabilityCommands::List,
        } => {
            println!("{CATALOG}");
            Ok(())
        }
        Commands::Profile { command } => match command {
            ProfileCommands::Detect { output } => {
                let p = detect_profile();
                let text = serde_json::to_string_pretty(&p)?;
                if let Some(path) = output {
                    fs::write(&path, &text).with_context(|| format!("write {}", path.display()))?;
                } else {
                    println!("{text}");
                }
                Ok(())
            }
            ProfileCommands::Validate { path } => {
                let p: Profile = serde_json::from_slice(
                    &fs::read(&path).with_context(|| format!("read {}", path.display()))?,
                )?;
                validate_profile(&p)?;
                println!(
                    "valid profile: {} ({} capabilities)",
                    p.profile_id,
                    p.capabilities.len()
                );
                Ok(())
            }
        },
        Commands::Plan {
            requirement,
            profile,
        } => plan(&requirement, &profile),
        Commands::Artifact {
            command: ArtifactCommands::Add { index, path, kind },
        } => add_artifact(&index, &path, &kind),
        Commands::Observe {
            command:
                ObserveCommands::Add {
                    index,
                    artifact_id,
                    capability,
                    at,
                    value,
                    source,
                    confidence,
                },
        } => append_jsonl(
            &index,
            &json!({"record_type":"observation","artifact_id":artifact_id,"capability":capability,"time_seconds":at,"value":value,"source":{"kind":source},"confidence":confidence}),
        ),
        Commands::Validate { index } => validate_index(&index),
        Commands::Query {
            index,
            query,
            from,
            to,
        } => query_index(&index, &query, from, to),
        Commands::Install {
            command: InstallCommands::Skill { dir },
        } => install_text(dir.unwrap_or_else(default_skill_dir), "SKILL.md", SKILL),
        Commands::Install {
            command: InstallCommands::Tools { dir },
        } => install_tools(dir.unwrap_or_else(default_tools_dir)),
    }
}
fn init(root: PathBuf) -> Result<()> {
    fs::create_dir_all(&root)?;
    for (name, value) in [("profile.json", serde_json::to_string_pretty(&detect_profile())?), ("index.jsonl", String::new()), ("filmmap.json", "{\n  \"schema_version\": \"filmmap/v1\",\n  \"index\": \"index.jsonl\",\n  \"profile\": \"profile.json\"\n}\n".to_string())] {
        let path = root.join(name); if !path.exists() { fs::write(path, value)?; }
    }
    println!("initialized FilmMap workspace at {}", root.display());
    Ok(())
}
fn detect_profile() -> Profile {
    let entries: Vec<Value> = serde_json::from_str(CATALOG).unwrap_or_default();
    let capabilities = entries
        .iter()
        .map(|e| {
            let id = e["id"].as_str().unwrap_or_default().to_string();
            let exe = e["executable"].as_str();
            if let Some(exe) = exe {
                if let Some(path) = find_on_path(exe) {
                    CapabilityState {
                        id,
                        status: "available".into(),
                        executable: Some(path),
                        version: None,
                        note: None,
                    }
                } else {
                    CapabilityState {
                        id,
                        status: "unavailable".into(),
                        executable: None,
                        version: None,
                        note: Some(format!("executable {exe} not found on PATH")),
                    }
                }
            } else {
                CapabilityState {
                    id,
                    status: "unverified".into(),
                    executable: None,
                    version: None,
                    note: Some(
                        "provided by an external agent or service; not probed by FilmMap".into(),
                    ),
                }
            }
        })
        .collect();
    Profile {
        schema_version: "filmmap/profile/v1".into(),
        profile_id: "local-auto-detect".into(),
        generated_at: "unspecified".into(),
        capabilities,
    }
}
fn find_on_path(name: &str) -> Option<String> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|p| p.join(name))
        .find(|p| p.is_file())
        .map(|p| p.display().to_string())
}
fn validate_profile(p: &Profile) -> Result<()> {
    if p.schema_version != "filmmap/profile/v1" {
        bail!("unsupported schema_version: {}", p.schema_version);
    }
    let catalog: Vec<Value> = serde_json::from_str(CATALOG)?;
    let valid: Vec<&str> = catalog.iter().filter_map(|v| v["id"].as_str()).collect();
    for c in &p.capabilities {
        if !valid.contains(&c.id.as_str()) {
            bail!("unknown capability id: {}", c.id);
        }
        if !["available", "unavailable", "unverified", "failed"].contains(&c.status.as_str()) {
            bail!("invalid status for {}: {}", c.id, c.status);
        }
    }
    Ok(())
}
fn plan(req_path: &Path, profile_path: &Path) -> Result<()> {
    let req: Value = serde_json::from_slice(&fs::read(req_path)?)?;
    let p: Profile = serde_json::from_slice(&fs::read(profile_path)?)?;
    validate_profile(&p)?;
    let required = req["required_capabilities"]
        .as_array()
        .context("requirement.required_capabilities must be an array")?;
    let mut ready = Vec::new();
    let mut missing = Vec::new();
    let mut uncertain = Vec::new();
    for item in required {
        let id = item.as_str().context("capability IDs must be strings")?;
        match p
            .capabilities
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.status.as_str())
        {
            Some("available") => ready.push(id),
            Some("unverified") => uncertain.push(id),
            _ => missing.push(id),
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"requirement_id":req["id"],"ready":ready,"unverified":uncertain,"missing":missing,"executable":missing.is_empty() && uncertain.is_empty()})
        )?
    );
    Ok(())
}
fn add_artifact(index: &Path, path: &Path, kind: &str) -> Result<()> {
    let mut file =
        fs::File::open(path).with_context(|| format!("open media {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let n = file
            .read(&mut buffer)
            .with_context(|| format!("hash media {}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let digest = format!("{:x}", hasher.finalize());
    let id = format!("sha256:{digest}");
    let metadata = fs::metadata(path)?;
    if index.exists() {
        let content = fs::read_to_string(index)
            .with_context(|| format!("read existing index {}", index.display()))?;
        let mut already_referenced = false;
        for (line_no, line) in content.lines().enumerate() {
            let record: Value = serde_json::from_str(line)
                .with_context(|| format!("invalid JSON at index line {}", line_no + 1))?;
            if record["artifact_id"] == id && record["path"] == json!(path) {
                already_referenced = true;
                break;
            }
        }
        if !already_referenced {
            if content.lines().any(|line| {
                serde_json::from_str::<Value>(line).is_ok_and(|record| record["artifact_id"] == id)
            }) {
                append_jsonl(
                    index,
                    &json!({"record_type":"artifact_path","artifact_id":id,"path":path,"evidence":{"filesystem_path":path.display().to_string()}}),
                )?;
                println!("{id}");
                return Ok(());
            }
        } else {
            println!("{id}");
            return Ok(());
        }
    }
    append_jsonl(
        index,
        &json!({"record_type":"artifact","artifact_id":id,"path":path,"kind":kind,"size_bytes":metadata.len(),"sha256":digest,"evidence":{"filesystem_path":path.display().to_string()}}),
    )?;
    println!("{id}");
    Ok(())
}
fn append_jsonl(index: &Path, value: &Value) -> Result<()> {
    if let Some(p) = index.parent() {
        fs::create_dir_all(p)?;
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(index)
        .with_context(|| format!("open index {}", index.display()))?;
    writeln!(f, "{}", value).with_context(|| format!("append index {}", index.display()))?;
    Ok(())
}
fn validate_index(index: &Path) -> Result<()> {
    let data = fs::read_to_string(index)?;
    let mut seen = std::collections::HashSet::new();
    let mut references = Vec::new();
    let mut count = 0;
    for (i, line) in data.lines().enumerate() {
        let v: Value = serde_json::from_str(line)
            .with_context(|| format!("invalid JSON at line {}", i + 1))?;
        let t = v["record_type"].as_str().context("record_type required")?;
        if !["artifact", "artifact_path", "observation", "resolution"].contains(&t) {
            bail!("unknown record_type at line {}: {t}", i + 1);
        }
        if t == "artifact" {
            let id = v["artifact_id"].as_str().context("artifact_id required")?;
            if !seen.insert(id.to_string()) {
                bail!("duplicate artifact_id at line {}: {id}", i + 1);
            }
        } else if v["artifact_id"].as_str().is_none() {
            bail!("artifact_id required at line {}", i + 1);
        } else if let Some(id) = v["artifact_id"].as_str() {
            references.push((i + 1, id.to_string()));
        }
        count += 1;
    }
    for (line, id) in references {
        if !seen.contains(&id) {
            bail!("artifact reference at line {line} has no artifact record: {id}");
        }
    }
    println!("valid index: {count} records");
    Ok(())
}
fn query_index(index: &Path, q: &str, from: Option<f64>, to: Option<f64>) -> Result<()> {
    let mut found = Vec::new();
    for line in fs::read_to_string(index)?.lines() {
        let v: Value = serde_json::from_str(line)?;
        let text = v.to_string().to_lowercase();
        if !text.contains(&q.to_lowercase()) {
            continue;
        }
        if let Some(t) = v["time_seconds"].as_f64()
            && (from.is_some_and(|x| t < x) || to.is_some_and(|x| t > x))
        {
            continue;
        }
        found.push(v);
    }
    println!("{}", serde_json::to_string_pretty(&found)?);
    Ok(())
}
fn install_text(dir: PathBuf, name: &str, text: &str) -> Result<()> {
    fs::create_dir_all(&dir)?;
    let dst = dir.join(name);
    if dst.exists() {
        let old = fs::read_to_string(&dst)?;
        if old == text {
            println!("already installed: {}", dst.display());
            return Ok(());
        }
        bail!("refusing to overwrite different file: {}", dst.display());
    }
    fs::write(&dst, text)?;
    println!("installed {}", dst.display());
    Ok(())
}
fn install_tools(dir: PathBuf) -> Result<()> {
    fs::create_dir_all(&dir)?;
    let entries: Value = serde_json::from_str(TOOL_MANIFEST)?;
    for entry in entries
        .as_array()
        .context("tool manifest must be an array")?
    {
        let name = entry["name"]
            .as_str()
            .context("tool name must be a string")?;
        let source = entry["path"]
            .as_str()
            .context("tool path must be a string")?;
        if Path::new(name).file_name().and_then(|value| value.to_str()) != Some(name) {
            bail!("tool name must be a plain filename: {name}");
        }
        let content = embedded_tool(source)?;
        let dst = dir.join(name);
        if dst.exists() {
            let same = fs::read(&dst)? == content.as_bytes();
            if !same {
                bail!("refusing to overwrite different file: {}", dst.display());
            }
        } else {
            fs::write(&dst, content)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut p = fs::metadata(&dst)?.permissions();
                p.set_mode(0o755);
                fs::set_permissions(&dst, p)?;
            }
        }
    }
    println!("tools installed in {}", dir.display());
    Ok(())
}
fn embedded_tool(path: &str) -> Result<&'static str> {
    match path {
        "tools/filmmap-probe.sh" => Ok(include_str!("../tools/filmmap-probe.sh")),
        "tools/filmmap-frames.sh" => Ok(include_str!("../tools/filmmap-frames.sh")),
        "tools/filmmap-scan.sh" => Ok(include_str!("../tools/filmmap-scan.sh")),
        _ => bail!("tool is not embedded in this CLI build: {path}"),
    }
}
fn default_skill_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".agents/skills/filmmap")
}
fn default_tools_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/share/filmmap/tools")
}

#[cfg(test)]
mod e2e {
    use super::*;
    #[test]
    fn cli_workspace_artifact_observation_validate_query_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("project");
        init(root.clone()).unwrap();
        let media = d.path().join("clip.bin");
        fs::write(&media, b"test media").unwrap();
        add_artifact(&root.join("index.jsonl"), &media, "video").unwrap();
        let id = format!("sha256:{:x}", Sha256::digest(b"test media"));
        append_jsonl(
            &root.join("index.jsonl"),
            &json!({"record_type":"observation","artifact_id":id,"capability":"image.describe","time_seconds":3.0,"value":"mountain landscape","source":{"kind":"manual"},"confidence":0.91}),
        ).unwrap();
        validate_index(&root.join("index.jsonl")).unwrap();
        query_index(&root.join("index.jsonl"), "mountain", Some(2.0), Some(4.0)).unwrap();
    }
}
