use crate::shell::ShellProxy;
use clap::Parser;
use iroh::{PublicKey, SecretKey};
use p2term_lib::client::runtime;
use p2term_lib::client::server_handle::P2TermServerHandle;
use p2term_lib::convert::HexConvert;
use p2term_lib::crypto::{any_secret_key, generate_secret_key};
use p2term_lib::error::unpack;
use p2term_lib::proto::ClientOpt;
use std::path::PathBuf;
use std::process::ExitCode;

mod shell;

#[derive(Debug, clap::Parser)]
struct Args {
    /// The node id of the peer to connect to
    #[clap(long, short)]
    node_id: String,

    /// Secret key hex
    #[clap(long)]
    secret_key_hex: Option<String>,

    /// Secret key file
    #[clap(long)]
    secret_key_file: Option<PathBuf>,

    /// Shell to use on server, must be available on the server
    #[clap(long)]
    shell: Option<String>,

    /// Cwd for the shell on the server
    #[clap(long)]
    cwd: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> ExitCode {
    let sk = generate_secret_key();
    println!(
        "Generated secret key: {}, pk: {}",
        sk.to_hex(),
        sk.public().to_hex()
    );
    let args = Args::parse();
    match start_connection(args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", unpack(&*e));
            ExitCode::FAILURE
        }
    }
}

async fn start_connection(args: Args) -> anyhow::Result<()> {
    let parsed = parse_args(&args)?;
    let server_handle = P2TermServerHandle::connect(parsed.secret_key, parsed.peer).await?;
    let client_opt = ClientOpt {
        shell: args.shell,
        cwd: args.cwd,
    };
    runtime::run(server_handle, &client_opt, ShellProxy).await
}

struct ParsedArgs {
    peer: PublicKey,
    secret_key: SecretKey,
}

fn parse_args(args: &Args) -> anyhow::Result<ParsedArgs> {
    let peer = PublicKey::try_from_hex(args.node_id.as_bytes())?;
    let secret_key = any_secret_key(
        args.secret_key_hex.as_deref(),
        args.secret_key_file.as_deref(),
    )?;
    Ok(ParsedArgs { peer, secret_key })
}
