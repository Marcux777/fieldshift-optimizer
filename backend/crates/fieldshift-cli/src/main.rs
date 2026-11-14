// AI-assisted (OpenBot agent)
use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use fieldshift_io::snapshot::*;
use fieldshift_numeric::canon::sha256_hex;
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
#[derive(Parser)]
#[command(
    version,
    about = "FieldShift: pure Rust offline engine and POWER snapshot collector"
)]
struct Args {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    RunEvaluation {
        #[arg(long)]
        snapshot: Option<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    DiffReference {
        actual: PathBuf,
        reference: PathBuf,
    },
    CollectSnapshot {
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        refresh: bool,
    },
    VerifySnapshot {
        snapshot: Option<PathBuf>,
    },
}
fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::RunEvaluation { snapshot, output } => {
            let start = std::time::Instant::now();
            let (rr, ev, md) = fieldshift_cli::run_evaluation(&args.root, snapshot.as_deref())?;
            let out = output.unwrap_or_else(|| {
                args.root.join(format!(
                    "artifacts/rust/{}-{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("system clock")
                        .as_secs(),
                    std::process::id()
                ))
            });
            if let Some(p) = out.parent() {
                fs::create_dir_all(p)?;
            }
            fs::create_dir(&out)
                .context("output must not exist; historical artifacts are never overwritten")?;
            let mut sums = String::new();
            for (name, text) in [
                ("run_result.json", serde_json::to_string_pretty(&rr)?),
                ("eval_report.json", serde_json::to_string_pretty(&ev)?),
                ("eval_report.md", md),
            ] {
                fs::write(out.join(name), text.as_bytes())?;
                sums.push_str(&format!("{}  {name}\n", sha256_hex(text.as_bytes())));
            }
            fs::write(out.join("SHA256SUMS"), sums)?;
            let m = json!({"run_result_hash":rr["result_hash"],"eval_result_hash":ev["result_hash"],"outcome":ev["outcome"],"elapsed_seconds":start.elapsed().as_secs_f64(),"output":out,"snapshot_id":rr["snapshot_id"],"protocol_file_sha256":fieldshift_eval::evaluation::PROTOCOL_SHA256,"cargo_lock_sha256":sha256_hex(&fs::read(args.root.join("backend/Cargo.lock"))?)});
            fs::write(
                out.join("run_manifest.json"),
                serde_json::to_vec_pretty(&m)?,
            )?;
            println!("{}", serde_json::to_string_pretty(&m)?);
        }
        Command::DiffReference { actual, reference } => {
            let a = fs::read(&actual)?;
            let b = fs::read(&reference)?;
            let mut av: Value = serde_json::from_slice(&a)?;
            let mut bv: Value = serde_json::from_slice(&b)?;
            for v in [&mut av, &mut bv] {
                if let Some(o) = v.as_object_mut() {
                    o.remove("meta");
                    o.remove("result_hash");
                }
            }
            let mut diffs = Vec::new();
            fieldshift_cli::differences(&av, &bv, "$", &mut diffs);
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &json!({"classification":if diffs.is_empty(){if a==b{"IDENTICAL"}else{"SERIALIZATION_ONLY"}}else{"SCIENTIFIC"},"differences":diffs})
                )?
            );
            if !diffs.is_empty() {
                bail!("scientific differences found");
            }
        }
        Command::VerifySnapshot { snapshot } => {
            let p = match snapshot {
                Some(p) => p,
                None => latest_snapshot(&args.root.join("data/snapshots"))?,
            };
            let c = load_climate(&p)?;
            println!("{}", serde_json::to_string_pretty(&c.quality_report())?);
        }
        Command::CollectSnapshot { output, refresh } => {
            const URL:&str="https://power.larc.nasa.gov/api/temporal/daily/point?parameters=PRECTOTCORR,T2M,T2M_MAX,T2M_MIN&community=AG&longitude=-50.625&latitude=-18&start=19810101&end=20251231&format=JSON";
            if !refresh {
                let p = latest_snapshot(&args.root.join("data/snapshots"))?;
                let m = verify_manifest(&p)?;
                if m["request"]["url"] == URL {
                    println!("existing snapshot verified offline: {}", p.display());
                    return Ok(());
                }
                bail!("existing snapshot request differs; pass --refresh and --output for a new collection");
            }
            let root =
                output.context("--refresh requires --output outside frozen data/snapshots")?;
            fs::create_dir_all(&root)?;
            let frozen = args.root.join("data/snapshots").canonicalize()?;
            if root.canonicalize()?.starts_with(&frozen) {
                bail!("cannot write frozen snapshots");
            }
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .user_agent("fieldshift-rust/0.2")
                .build()?;
            let mut response = client.get(URL).send()?.error_for_status()?;
            let mut raw = Vec::new();
            std::io::Read::take(&mut response, 64 * 1024 * 1024 + 1).read_to_end(&mut raw)?;
            if raw.len() > 64 * 1024 * 1024 {
                bail!("POWER response exceeds 64 MiB");
            }
            let seconds = SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs()
                .to_string();
            let p = write_snapshot(&raw, &root, URL, &format!("unix:{seconds}"))?;
            std::io::stdout().write_all(format!("{}\n", p.display()).as_bytes())?;
        }
    }
    Ok(())
}
