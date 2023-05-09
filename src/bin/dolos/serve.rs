use std::path::Path;
use std::sync::Arc;

use dolos::prelude::*;
use dolos::rolldb::RollDB;

#[derive(Debug, clap::Args)]
pub struct Args {}

#[tokio::main]
pub async fn run(config: &super::Config, _args: &Args) -> Result<(), Error> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .finish(),
    )
    .unwrap();

    let rolldb_path = config
        .rolldb
        .path
        .as_deref()
        .unwrap_or_else(|| Path::new("/db"));

    let db =
        RollDB::open(rolldb_path, config.rolldb.k_param.unwrap_or(1000)).map_err(Error::config)?;

    dolos::downstream::grpc::serve(db).await?;

    Ok(())
}
