use opentelemetry::{global, propagation::Injector};
use tonic::{
    metadata::{MetadataKey, MetadataMap, MetadataValue},
    service::Interceptor,
    Request, Status,
};
use tracing::{info, warn, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::error::StdErrorExt as _;

/// Trace context propagation: send the trace context by injecting it into the metadata of the given
/// request.
#[derive(Clone)]
pub struct SendTrace;

impl Interceptor for SendTrace {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        global::get_text_map_propagator(|propagator| {
            let context = Span::current().context();
            propagator.inject_context(&context, &mut MetadataInjector(request.metadata_mut()))
        });

        Ok(request)
    }
}

/// A wrapper struct for injecting trace context into gRPC metadata.
struct MetadataInjector<'a>(&'a mut MetadataMap);

impl Injector for MetadataInjector<'_> {
    /// Sets a key-value pair in the metadata map.
    ///
    /// # Arguments
    ///
    /// * `key` - The key for the metadata entry.
    /// * `value` - The value for the metadata entry.
    ///
    /// This method attempts to convert the key and value into the appropriate types
    /// for gRPC metadata. If conversion fails, it logs a warning and skips the entry.
    fn set(&mut self, key: &str, value: String) {
        info!("injector set `{key}` -> `{value}`");
        match MetadataKey::from_bytes(key.as_bytes()) {
            Ok(key) => match MetadataValue::try_from(&value) {
                Ok(value) => {
                    self.0.insert(key, value);
                }

                Err(error) => warn!(value, error = error.as_chain(), "parse metadata value"),
            },

            Err(error) => warn!(key, error = error.as_chain(), "parse metadata key"),
        }
    }
}
