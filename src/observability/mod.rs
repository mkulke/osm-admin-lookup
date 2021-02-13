use actix_web::{dev, http};
use opentelemetry::sdk::{
    export::trace::SpanExporter, trace::BatchSpanProcessor, trace::TracerProvider,
};

const PKG_NAME: &str = env!("CARGO_PKG_NAME");

pub fn is_metrics_route(req: &dev::ServiceRequest) -> bool {
    req.path() == "/metrics" && req.method() == http::Method::GET
}

// Compatibility with older tokio v0.2.x used by actix web v3. Not necessary with actix web v4.
pub fn tokio_exporter_compat<T: SpanExporter + 'static>(exporter: T) -> BatchSpanProcessor {
    let spawn = |fut| tokio::task::spawn_blocking(|| futures::executor::block_on(fut));
    BatchSpanProcessor::builder(
        exporter,
        spawn,
        tokio::time::delay_for,
        tokio::time::interval,
    )
    .build()
}

pub fn build_tracer_provider() -> TracerProvider {
    let jaeger_exporter = opentelemetry_jaeger::new_pipeline()
        .with_service_name(PKG_NAME)
        .init_exporter()
        .expect("could not install jaeger pipeline");
    TracerProvider::builder()
        .with_batch_exporter(tokio_exporter_compat(jaeger_exporter))
        .build()
}
