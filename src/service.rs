use rs_geo_playground::{run_service, ServiceConfig};
use structopt::StructOpt;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let config = ServiceConfig::from_args();
    run_service(config)?.await
}
