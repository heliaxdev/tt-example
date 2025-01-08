use std::path::PathBuf;

#[derive(clap::Parser, Clone, Debug)]
pub struct AppConfig {
    #[clap(long, env)]
    #[arg(required = true)]
    pub rpc: String,

    #[clap(long, env)]
    #[arg(required = true)]
    pub source_private_key: String,

    #[clap(long, env)]
    #[arg(required = true)]
    pub target_address: String,

    #[clap(long, env)]
    #[arg(required = true)]
    pub amount: u64,

    #[clap(long, env)]
    #[arg(required = true)]
    pub chain_id: String,

    #[clap(long, env)]
    pub expiration_timestamp_utc: Option<i64>,

    #[clap(long, env)]
    pub memo: Option<String>,

    #[clap(long, env)]
    pub base_dir: Option<PathBuf>,
}