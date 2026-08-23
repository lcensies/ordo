use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(
    name = "ordo",
    version,
    about = "Unorthodox file transfer via shell one-liners",
    after_help = "EXAMPLES:
  # Print transfer one-liners to stdout (paste manually)
  ordo send ./tool -t stdout -r /tmp/tool

  # Type into a tmux pane (slow channel: add --delay)
  ordo send ./tool -t tmux:session.0 -r /tmp/tool --delay 50

  # Stream over an existing ssh connection
  ordo send ./tool -t ssh:user@host -r /tmp/tool --verify

  # Run through a local shell, make executable, execute
  ordo send ./tool -t exec:sh -r /tmp/tool --exec

BACKENDS (-t):
  stdout          print commands (default for dry inspection)
  tmux:<target>   tmux send-keys to <target> pane
  ssh:<[user@]host>  pipe commands into remote sh over ssh
  exec:<cmd>      pipe commands into stdin of local <cmd>"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
    Send(SendArgs),
}

#[derive(clap::Args)]
pub struct SendArgs {
    /// Source: file path, - for stdin, or http(s):// URL (omit when using --stdin)
    pub source: Option<String>,

    /// Read source from stdin (equivalent to passing - as source)
    #[arg(long)]
    pub stdin: bool,

    /// Output backend: stdout | tmux:<target> | ssh:<[user@]host> | exec:<cmd>
    #[arg(long, short = 't')]
    pub to: String,

    /// Destination path on remote
    #[arg(long, short = 'r')]
    pub remote_path: String,

    /// Encoding strategy (basenc/base32 is less signatured than base64)
    #[arg(long, short = 'e', default_value = "basenc")]
    pub encoding: Encoding,

    /// Bytes per chunk
    #[arg(long, short = 'c', default_value = "1024")]
    pub chunk_size: usize,

    /// Verify checksum after transfer
    #[arg(long)]
    pub verify: bool,

    /// Delay between chunks in milliseconds (useful for tmux/VNC)
    #[arg(long, default_value = "0")]
    pub delay: u64,

    /// chmod remote file after transfer, octal MODE (default 755)
    #[arg(long, num_args = 0..=1, default_missing_value = "755", value_name = "MODE")]
    pub chmod: Option<String>,

    /// chmod method: perl builtin (no /bin/chmod execve) or direct
    #[arg(long, value_enum, default_value = "perl")]
    pub chmod_method: crate::backend::chmod::ChmodMethod,

    /// Execute remote file after transfer (implies --chmod 755 unless --no-chmod).
    /// Optional ARGS string is appended to the invocation.
    #[arg(long, num_args = 0..=1, default_missing_value = "", value_name = "ARGS")]
    pub exec: Option<String>,

    /// Skip the chmod normally implied by --exec
    #[arg(long, conflicts_with = "chmod")]
    pub no_chmod: bool,
}

#[derive(ValueEnum, Clone)]
pub enum Encoding {
    Basenc,
    Base64,
    Perl,
    Printf,
    Python,
}
