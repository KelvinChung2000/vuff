//! `vuff` — the unified SystemVerilog formatter + (eventually) linter CLI.

use std::ffi::OsString;
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use similar::TextDiff;
use vuff_config::{load_config, ConfigSource, FormatOptions, ResolvedConfig};

/// Standard Unix-ish exit codes.
mod exit {
    pub const OK: u8 = 0;
    pub const WOULD_CHANGE: u8 = 1;
    pub const ERROR: u8 = 2;
}

#[derive(Parser, Debug)]
#[command(name = "vuff", version, about = "SystemVerilog formatter (ruff-style)")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Format one or more SystemVerilog files (or stdin).
    Format(FormatArgs),
    /// Show the resolved configuration.
    Config {
        #[command(subcommand)]
        action: ConfigCmd,
    },
}

#[derive(Parser, Debug)]
struct FormatArgs {
    /// Paths to format. Omit to read from stdin.
    paths: Vec<PathBuf>,
    /// Exit 1 without writing if any file would change.
    #[arg(long)]
    check: bool,
    /// Print a unified diff of would-be changes.
    #[arg(long)]
    diff: bool,
    /// Filename associated with stdin (used for config discovery).
    #[arg(long)]
    stdin_filename: Option<PathBuf>,
    /// Explicit path to `vuff.toml`.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Fail if a format pass does not parse-round-trip. Always on; kept for
    /// symmetry with future debug flags.
    #[arg(long)]
    assert_stable: bool,
}

#[derive(Subcommand, Debug)]
enum ConfigCmd {
    /// Print the resolved configuration as TOML.
    Show,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("vuff: {err:#}");
            ExitCode::from(exit::ERROR)
        }
    }
}

fn run() -> Result<u8> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Format(args) => run_format(args),
        Cmd::Config {
            action: ConfigCmd::Show,
        } => run_config_show().map(|_| exit::OK),
    }
}

fn resolve_config(explicit: Option<&Path>, start: &Path) -> Result<ResolvedConfig> {
    let env = std::env::var_os("VUFF_CONFIG");
    let env_ref: Option<&OsString> = env.as_ref();
    let cfg = load_config(explicit, env_ref.map(AsRef::as_ref), start).context("load config")?;
    Ok(cfg)
}

fn run_format(args: FormatArgs) -> Result<u8> {
    // Discovery anchor: first path given, else stdin filename's dir, else cwd.
    let anchor: PathBuf = args
        .paths
        .first()
        .cloned()
        .or_else(|| args.stdin_filename.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let resolved = resolve_config(args.config.as_deref(), &anchor)?;
    let opts = resolved.options;

    if args.paths.is_empty() {
        return run_stdin(&opts, &args);
    }

    let mut any_change = false;
    for target in &args.paths {
        for file in collect_sv_files(target)? {
            if process_file(&file, &opts, &args)? {
                any_change = true;
            }
        }
    }

    Ok(if (args.check || args.diff) && any_change {
        exit::WOULD_CHANGE
    } else {
        exit::OK
    })
}

fn run_stdin(opts: &FormatOptions, args: &FormatArgs) -> Result<u8> {
    if std::io::stdin().is_terminal() {
        anyhow::bail!("stdin is a tty; pass paths or pipe input");
    }
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .context("read stdin")?;
    let out = vuff_sv_formatter::format_source(&buf, opts).context("format stdin")?;
    if args.check {
        return Ok(if out == buf {
            exit::OK
        } else {
            exit::WOULD_CHANGE
        });
    }
    if args.diff {
        print_diff(args.stdin_filename.as_deref(), &buf, &out);
        return Ok(if out == buf {
            exit::OK
        } else {
            exit::WOULD_CHANGE
        });
    }
    std::io::stdout()
        .write_all(out.as_bytes())
        .context("write stdout")?;
    Ok(exit::OK)
}

fn process_file(path: &Path, opts: &FormatOptions, args: &FormatArgs) -> Result<bool> {
    let src = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let out = vuff_sv_formatter::format_source(&src, opts)
        .with_context(|| format!("format {}", path.display()))?;
    let changed = out != src;
    if !changed {
        return Ok(false);
    }
    if args.check {
        eprintln!("would reformat: {}", path.display());
        return Ok(true);
    }
    if args.diff {
        print_diff(Some(path), &src, &out);
        return Ok(true);
    }
    std::fs::write(path, out).with_context(|| format!("write {}", path.display()))?;
    Ok(true)
}

fn collect_sv_files(target: &Path) -> Result<Vec<PathBuf>> {
    if target.is_file() {
        return Ok(vec![target.to_path_buf()]);
    }
    let walker = ignore::WalkBuilder::new(target).follow_links(false).build();
    let mut files = Vec::new();
    for entry in walker {
        let entry = entry.context("walk")?;
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let p = entry.path();
        if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            if matches!(ext, "sv" | "svh" | "v" | "vh") {
                files.push(p.to_path_buf());
            }
        }
    }
    Ok(files)
}

fn print_diff(path: Option<&Path>, old: &str, new: &str) {
    let label = path.map_or_else(|| "<stdin>".to_owned(), |p| p.display().to_string());
    let diff = TextDiff::from_lines(old, new);
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "--- {label}");
    let _ = writeln!(stdout, "+++ {label}");
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            similar::ChangeTag::Delete => '-',
            similar::ChangeTag::Insert => '+',
            similar::ChangeTag::Equal => ' ',
        };
        let _ = write!(stdout, "{sign}{change}");
    }
}

fn run_config_show() -> Result<()> {
    let anchor = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let resolved = resolve_config(None, &anchor)?;
    let opts = resolved.options;
    match resolved.source {
        ConfigSource::Defaults => println!("# source: built-in defaults"),
        ConfigSource::File(p) => println!("# source: {}", p.display()),
    }
    println!("[format]");
    println!("line_width = {}", opts.line_width);
    println!("indent_width = {}", opts.indent_width);
    println!("indent_style = {:?}", opts.indent_style);
    println!("begin_style = {:?}", opts.begin_style);
    println!("port_list_style = {:?}", opts.port_list_style);
    println!("trailing_comma = {:?}", opts.trailing_comma);
    Ok(())
}
