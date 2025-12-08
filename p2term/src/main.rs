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
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Debug, clap::Subcommand)]
enum SubCommand {
    /// Connect to a peer term
    Connect {
        #[clap(flatten)]
        args: ConnectArgs,
    },
    /// Generate a new keypair for use when making a connection
    GenerateKeys {
        /// Secret key output file
        /// If specified, will write the secret key to the file as raw bytes
        /// If left unspecified, will write the secret key as hexadecimal to stdout
        #[clap(short, long)]
        secret_key_output_file: Option<PathBuf>,
    },
}

#[derive(Debug, clap::Parser)]
struct ConnectArgs {
    /// The `node id`/`public key` of the peer to connect to
    #[clap(long, short, env = "P2TERM_PEER")]
    peer: String,

    /// Secret key hex
    #[clap(long, env = "P2TERM_SECRET_KEY_HEX")]
    secret_key_hex: Option<String>,

    /// Secret key file
    #[clap(long, env = "P2TERM_SECRET_KEY_FILE")]
    secret_key_file: Option<PathBuf>,

    /// Shell to use on server, must be available on the server
    #[clap(long, env = "P2TERM_SHELL")]
    shell: Option<String>,

    /// Cwd for the shell on the server
    #[clap(long, env = "P2TERM_CWD")]
    cwd: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    match args.subcmd {
        SubCommand::Connect { args } => match start_connection(args).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {}", unpack(&*e));
                ExitCode::FAILURE
            }
        },
        SubCommand::GenerateKeys {
            secret_key_output_file,
        } => {
            let sk = generate_secret_key();
            println!(
                "Generated key pair with public key hex: {}",
                sk.public().to_hex()
            );
            if let Some(path) = secret_key_output_file {
                if let Err(e) = std::fs::write(&path, sk.to_bytes()) {
                    eprintln!(
                        "failed to write secret key to file: {}error: {}",
                        path.display(),
                        unpack(&e)
                    );
                    return ExitCode::FAILURE;
                }
            } else {
                println!("Secret key: {}", sk.to_hex());
            }
            ExitCode::SUCCESS
        }
    }
}

async fn start_connection(args: ConnectArgs) -> anyhow::Result<()> {
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

fn parse_args(args: &ConnectArgs) -> anyhow::Result<ParsedArgs> {
    let peer = PublicKey::try_from_hex(args.peer.as_bytes())?;
    let secret_key = any_secret_key(
        args.secret_key_hex.as_deref(),
        args.secret_key_file.as_deref(),
    )?;
    Ok(ParsedArgs { peer, secret_key })
}
