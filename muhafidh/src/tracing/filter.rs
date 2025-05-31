use tracing::Level;
use tracing::Metadata;
use tracing_subscriber::layer::Context;
use tracing_subscriber::layer::Filter;
use tracing_subscriber::registry::LookupSpan;

// Custom filter for exact debug level matching
pub struct DebugOnlyFilter;

impl<S> Filter<S> for DebugOnlyFilter
where
    S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn enabled(
        &self,
        meta: &Metadata<'_>,
        _ctx: &Context<'_, S>,
    ) -> bool {
        let target = meta.target();
        meta.level() == &Level::DEBUG && target.starts_with("muhafidh")
    }
}

// Custom filter for error and warn levels
pub struct ErrorWarnFilter;

impl<S> Filter<S> for ErrorWarnFilter
where
    S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn enabled(
        &self,
        meta: &Metadata<'_>,
        _ctx: &Context<'_, S>,
    ) -> bool {
        let target = meta.target();
        (meta.level() == &Level::ERROR || meta.level() == &Level::WARN) && target.starts_with("muhafidh")
    }
}

// Custom filter for info levels
#[cfg(feature = "dev")]
pub struct InfoOnlyFilter;

#[cfg(feature = "dev")]
impl<S> Filter<S> for InfoOnlyFilter
where
    S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn enabled(
        &self,
        meta: &Metadata<'_>,
        _ctx: &Context<'_, S>,
    ) -> bool {
        let target = meta.target();
        meta.level() == &Level::INFO && target.starts_with("muhafidh")
    }
}

// Custom filter for error levels
pub struct ErrorOnlyFilter;

impl<S> Filter<S> for ErrorOnlyFilter
where
    S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn enabled(
        &self,
        meta: &Metadata<'_>,
        _ctx: &Context<'_, S>,
    ) -> bool {
        let target = meta.target();
        meta.level() == &Level::ERROR && target.starts_with("muhafidh")
    }
}

// Custom filter for warn levels
pub struct WarnOnlyFilter;

impl<S> Filter<S> for WarnOnlyFilter
where
    S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn enabled(
        &self,
        meta: &Metadata<'_>,
        _ctx: &Context<'_, S>,
    ) -> bool {
        let target = meta.target();
        meta.level() == &Level::WARN && target.starts_with("muhafidh")
    }
}