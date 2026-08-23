mod backend;
mod cli;
mod encoder;
mod source;
mod transfer;

use anyhow::Context;
use clap::Parser;
use cli::{Cli, Command, Encoding};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Send(args) => {
            // Build source
            let source_str = args.source.as_deref().unwrap_or("");
            let mut src: Box<dyn source::Source> = if args.stdin || source_str == "-" {
                Box::new(source::stdin::StdinSource)
            } else if source_str.starts_with("http://") || source_str.starts_with("https://") {
                Box::new(source::http::HttpSource::new(source_str))
            } else if source_str.is_empty() {
                anyhow::bail!("No source specified. Provide a file path, -, or use --stdin");
            } else {
                Box::new(source::file::FileSource::new(source_str))
            };

            // Build encoder
            let enc: Box<dyn encoder::Encoder> = match args.encoding {
                Encoding::Basenc => Box::new(encoder::basenc::BasencEncoder),
                Encoding::Base64 => Box::new(encoder::base64::Base64Encoder),
                Encoding::Perl => Box::new(encoder::perl::PerlEncoder),
                Encoding::Printf => Box::new(encoder::printf::PrintfEncoder),
                Encoding::Python => Box::new(encoder::python::PythonEncoder),
            };

            // Build backend
            let mut backend: Box<dyn backend::OutputBackend> = if args.to == "stdout" {
                Box::new(backend::stdout::StdoutBackend)
            } else if let Some(target) = args.to.strip_prefix("tmux:") {
                Box::new(backend::tmux::TmuxBackend {
                    target: target.to_string(),
                })
            } else if let Some(spec) = args.to.strip_prefix("ssh:") {
                Box::new(backend::ssh::SshBackend::new(spec))
            } else if let Some(cmd) = args.to.strip_prefix("exec:") {
                Box::new(backend::exec::ExecBackend::new(cmd))
            } else {
                anyhow::bail!(
                    "Unknown backend '{}'. Use: stdout | tmux:<target> | ssh:<host> | exec:<cmd>",
                    args.to
                );
            };

            // --exec implies chmod 755 unless --no-chmod; explicit --chmod wins
            let chmod: Option<&str> = if args.no_chmod {
                None
            } else {
                args.chmod.as_deref().or(args.exec.as_ref().map(|_| "755"))
            };

            transfer::run(
                src.as_mut(),
                enc.as_ref(),
                backend.as_mut(),
                &args.remote_path,
                args.chunk_size,
                args.verify,
                args.delay,
                chmod,
                args.chmod_method,
                args.exec.as_deref(),
            )
            .with_context(|| format!("transfer failed"))?;
        }
    }

    Ok(())
}
