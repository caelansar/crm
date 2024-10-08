use http::{HeaderMap, Request};
use opentelemetry::{global, propagation::Extractor, trace::TraceContextExt};
use tracing::{field, info_span, warn, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Create a span for the incoming request
pub fn make_span<B>(request: &http::Request<B>) -> Span {
    let headers = request.headers();
    info_span!("incoming request", ?headers, trace_id = field::Empty)
}

/// Trace context propagation: associate the current span with the OTel trace of the given request
pub fn accept_trace<B>(request: &Request<B>, span: &Span) {
    let parent_context = global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(request.headers()))
    });
    span.set_parent(parent_context);

    let trace_id = span.context().span().span_context().trace_id();
    span.record("trace_id", trace_id.to_string());
}

struct HeaderExtractor<'a>(&'a HeaderMap);

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| {
            let s = v.to_str();
            if let Err(ref error) = s {
                warn!(%error, ?v, "cannot convert header value to ASCII")
            };
            s.ok()
        })
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}
