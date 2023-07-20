use tracing::{info, instrument};
use tracing_subscriber;

pub mod config;

#[instrument]
pub fn main() {
	// install global collector configured based on RUST_LOG env var.
	tracing_subscriber::fmt::init();

	// Load configuration variables
	let config = config::Config::from_env(&config::SystemEnvironment);
	info!("Initializing config with {:?}", config);
}
