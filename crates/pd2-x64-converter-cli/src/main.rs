use pd2_x64_converter_core::{
  CancelToken, ConvertOptions, EntryStatus, RunManifest, ScanOptions, convert, scan,
};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
  match run(env::args().skip(1).collect()) {
    Ok(code) => code,
    Err(error) => {
      eprintln!("error: {error}");
      ExitCode::from(2)
    }
  }
}

fn run(args: Vec<String>) -> Result<ExitCode, String> {
  let Some(command) = args.first().map(String::as_str) else {
    print_usage();
    return Ok(ExitCode::from(2));
  };

  match command {
    "scan" => {
      let parsed = ParsedArgs::parse(&args[1..])?;
      if parsed.dry_run {
        return Err("--dry-run is only supported for convert".to_string());
      }
      let manifest = scan(ScanOptions {
        root: parsed.root,
        jobs: parsed.jobs,
        write_report: parsed.report,
      })
      .map_err(|error| error.to_string())?;
      print_manifest(&manifest, parsed.json)?;
      Ok(exit_for(&manifest))
    }
    "convert" => {
      let parsed = ParsedArgs::parse(&args[1..])?;
      if parsed.dry_run {
        eprintln!("{}", pd2_x64_converter_core::dry_run_warning());
      } else {
        eprintln!("{}", pd2_x64_converter_core::destructive_write_warning());
      }
      let manifest = convert(
        ConvertOptions {
          root: parsed.root,
          jobs: parsed.jobs,
          write_report: parsed.report,
          dry_run: parsed.dry_run,
        },
        CancelToken::default(),
      )
      .map_err(|error| error.to_string())?;
      print_manifest(&manifest, parsed.json)?;
      Ok(exit_for(&manifest))
    }
    "-h" | "--help" | "help" => {
      print_usage();
      Ok(ExitCode::SUCCESS)
    }
    other => Err(format!("unknown command {other:?}")),
  }
}

struct ParsedArgs {
  root: PathBuf,
  json: bool,
  report: bool,
  jobs: usize,
  dry_run: bool,
}

impl ParsedArgs {
  fn parse(args: &[String]) -> Result<Self, String> {
    let mut root = None;
    let mut json = false;
    let mut report = false;
    let mut jobs = 1;
    let mut dry_run = false;
    let mut index = 0;

    while index < args.len() {
      match args[index].as_str() {
        "--json" => json = true,
        "--report" => report = true,
        "--dry-run" => dry_run = true,
        "--jobs" => {
          index += 1;
          let value = args.get(index).ok_or("--jobs requires a value")?;
          jobs = value
            .parse::<usize>()
            .map_err(|_| format!("invalid --jobs value {value:?}"))?
            .max(1);
        }
        value if value.starts_with('-') => return Err(format!("unknown option {value:?}")),
        value => {
          if root.replace(PathBuf::from(value)).is_some() {
            return Err("only one root path is supported".to_string());
          }
        }
      }
      index += 1;
    }

    Ok(Self {
      root: root.ok_or("missing root path")?,
      json,
      report,
      jobs,
      dry_run,
    })
  }
}

fn print_manifest(manifest: &RunManifest, json: bool) -> Result<(), String> {
  if json {
    println!(
      "{}",
      serde_json::to_string_pretty(manifest).map_err(|error| error.to_string())?
    );
    return Ok(());
  }

  println!("root: {}", manifest.root);
  println!("status: {:?}", manifest.status);
  println!("dry_run: {}", manifest.dry_run);
  println!("warning: {}", manifest.destructive_write_warning);
  println!(
    "counts: planned={} converted={} already_x64={} unsupported={} warning={} failed={} cancelled={}",
    manifest.summary.planned,
    manifest.summary.converted,
    manifest.summary.already_x64,
    manifest.summary.unsupported,
    manifest.summary.warning,
    manifest.summary.failed,
    manifest.summary.cancelled
  );
  if let Some(path) = &manifest.report_path {
    println!("report: {path}");
  }
  for entry in manifest
    .entries
    .iter()
    .filter(|entry| entry.error.is_some())
    .take(5)
  {
    println!(
      "error: {}: {}",
      entry.relative_path,
      entry.error.as_deref().unwrap_or("unknown error")
    );
  }
  Ok(())
}

fn exit_for(manifest: &RunManifest) -> ExitCode {
  if manifest
    .entries
    .iter()
    .any(|entry| entry.status == EntryStatus::Failed)
  {
    ExitCode::from(1)
  } else {
    ExitCode::SUCCESS
  }
}

fn print_usage() {
  eprintln!("usage:");
  eprintln!("  pd2-x64-converter-cli scan <root> [--jobs N] [--json] [--report]");
  eprintln!("  pd2-x64-converter-cli convert <root> [--jobs N] [--json] [--report] [--dry-run]");
}
