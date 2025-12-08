use tracing::Metadata;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::{Context, Filter, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;

pub fn setup_observability() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(LogFilter))
        .init();
}

struct LogFilter;

impl<S> Filter<S> for LogFilter
where
    S: tracing::Subscriber,
{
    fn enabled(&self, meta: &Metadata<'_>, _cx: &Context<'_, S>) -> bool {
        let target = meta.target();
        let (module, _rest) = if let Some((module, rest)) = target.split_once("::") {
            (module, rest)
        } else {
            (target, "")
        };
        match module {
            "p2termd" => true,
            "iroh" => meta.level() <= &tracing::Level::WARN,
            "netlink_packet_route" => meta.level() <= &tracing::Level::ERROR,
            _ => meta.level() <= &tracing::Level::INFO,
        }
    }
}
