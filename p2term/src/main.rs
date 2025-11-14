use anyhow::Context;
use clap::Parser;
use iroh::{PublicKey, SecretKey};
use p2term_lib::convert::HexConvert;
use p2term_lib::crypto::any_secret_key;
use p2term_lib::error::unpack;
use std::path::PathBuf;
use std::process::ExitCode;

mod proto;
mod shell;

#[derive(Debug, clap::Parser)]
struct Args {
    /// The node id of the peer to connect to
    #[clap(long, short)]
    node_id: String,

    /// Secret key hex
    secret_key_hex: Option<String>,

    /// Secret key file
    secret_key_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", unpack(&*e));
            ExitCode::FAILURE
        }
    }
}

async fn run() -> anyhow::Result<()> {
    let args = Args::parse();
    let args = parse_args(&args)?;
    let (send, recv) = proto::connect(args.peer, args.secret_key).await?;
    shell::shell_proxy(recv, send)
        .await
        .context("failed to run shell proxy")
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
