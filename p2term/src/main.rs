mod pty;
mod shell;

#[derive(Debug, clap::Parser)]
struct Args {
    /// The node id of the peer to connect to
    #[clap(long, short)]
    node_id: String,
}

#[tokio::main]
async fn main() {
    shell::shell_proxy().await.unwrap();
}
