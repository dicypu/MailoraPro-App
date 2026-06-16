use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Registry};

#[allow(dead_code)]
pub fn init_tracing() {
    let env_filter = EnvFilter::new("info");
    let fmt_layer = fmt::layer().with_target(false);

    let subscriber = Registry::default().with(env_filter).with(fmt_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set global subscriber");

    info!("Tracing initialized");
}
