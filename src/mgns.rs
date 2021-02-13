use actix_service::Service;
use actix_web::{web, App, HttpServer};
use actix_web_opentelemetry::RequestTracing;
use observability::tokio_exporter_compat;
use opentelemetry::{
    global,
    sdk::{propagation::TraceContextPropagator, trace::TracerProvider},
};
use tracing::subscriber::set_global_default;
use tracing_actix_web::TracingLogger;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

const PKG_NAME: &str = env!("CARGO_PKG_NAME");

async fn index() -> &'static str {
    "Hello!"
}

pub mod observability;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info,actix_web=error");

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer = BunyanFormattingLayer::new(PKG_NAME.into(), std::io::stdout);

    global::set_text_map_propagator(TraceContextPropagator::new());
    let exporter = opentelemetry_jaeger::new_pipeline()
        .with_service_name(PKG_NAME)
        .init_exporter()
        .expect("could not install jaeger pipeline");

    let tracer_provider = TracerProvider::builder()
        .with_batch_exporter(tokio_exporter_compat(exporter))
        .build();
    let _uninstall = global::set_tracer_provider(tracer_provider);

    let exporter = opentelemetry_prometheus::exporter().init();
    let request_metrics = actix_web_opentelemetry::RequestMetrics::new(
        opentelemetry::global::meter("actix_web"),
        Some(|req: &actix_web::dev::ServiceRequest| {
            req.path() == "/metrics" && req.method() == actix_web::http::Method::GET
        }),
        Some(exporter),
    );

    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    // .with(telemetry);

    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");

    HttpServer::new(move || {
        App::new()
            .wrap_fn(|req, srv| {
                log::info!("hello from middleware!");
                srv.call(req)
            })
            .wrap(RequestTracing::new())
            .wrap(request_metrics.clone())
            .wrap(TracingLogger)
            .service(web::resource("/").to(index))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
