use super::boundaries;
use super::boundary::Boundary;
use super::location::Location;
use super::RTree;
use actix_web::dev::Service as _;
use actix_web::{error, get, web, App, HttpServer, Responder, Result};
use futures_util::future::FutureExt;
use lazy_static::lazy_static;
use prometheus::{register_histogram_vec, register_int_counter_vec};
use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, Opts};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::error::Error;
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::task;
use tracing_actix_web::TracingLogger;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

lazy_static! {
    static ref RESPONSE_CODE_COLLECTOR: IntCounterVec = register_int_counter_vec!(
        Opts::new("http_requests_total", "Total Requests"),
        &["code", "method", "route"],
    )
    .unwrap();
    static ref RESPONSE_TIME_COLLECTOR: HistogramVec = register_histogram_vec!(
        HistogramOpts::new("http_request_duration_seconds", "Response Times"),
        &["code", "method", "route"]
    )
    .unwrap();
}

type AppState = Arc<RTree>;

impl From<Vec<&Boundary>> for LocateResponse {
    fn from(boundaries: Vec<&Boundary>) -> Self {
        let boundaries = boundaries
            .into_iter()
            .map(|boundary| BoundaryResponse {
                level: boundary.admin_level,
                name: boundary.name.clone(),
            })
            .collect();
        LocateResponse { boundaries }
    }
}

#[derive(Deserialize, Serialize)]
pub struct LocateResponse {
    pub boundaries: Vec<BoundaryResponse>,
}

#[derive(Deserialize, Serialize)]
pub struct BoundaryResponse {
    pub level: u8,
    pub name: String,
}

#[derive(Deserialize)]
pub struct LocateQuery {
    loc: String,
}

#[get("/locate")]
pub async fn locate(
    info: web::Query<LocateQuery>,
    state: web::Data<AppState>,
) -> Result<impl Responder> {
    let location: Location = info
        .loc
        .as_str()
        .try_into()
        .map_err(error::ErrorBadRequest)?;
    let response: LocateResponse =
        task::spawn_blocking(move || boundaries(&location, &state.clone()).into())
            .await
            .unwrap();
    Ok(web::Json(response))
}

#[get("/health")]
async fn health() -> &'static str {
    "Ok"
}

#[get("/metrics")]
async fn metrics() -> String {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = prometheus::gather();
    encoder.encode_to_string(&metric_families).unwrap()
}

fn track_metrics(code: u16, method: &str, route: &str, time: f64) {
    // fn track_metrics(code: u16, method: &str, route: &str) {
    // dos protection
    if route != "/locate" && route != "/health" {
        return;
    }

    let normalized_code = match code {
        200..=299 => "2XX",
        300..=399 => "3XX",
        400..=499 => "4XX",
        500..=599 => "5XX",
        _ => "invalid",
    };

    RESPONSE_CODE_COLLECTOR
        .with_label_values(&[normalized_code, method, route])
        .inc();

    RESPONSE_TIME_COLLECTOR
        .with_label_values(&[normalized_code, method, route])
        .observe(time);
}

fn init_logging() {
    let app_name = env!("CARGO_PKG_NAME");
    let formatting_layer = BunyanFormattingLayer::new(app_name.into(), std::io::stdout);
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
        .init();
}

pub async fn start(rtree: RTree, port: u16) -> std::result::Result<(), Box<dyn Error>> {
    init_logging();
    let state = Arc::new(rtree);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(TracingLogger::default())
            .wrap_fn(|req, srv| {
                let path = String::from(req.path());
                let method = String::from(req.method().as_str());
                let offset = OffsetDateTime::now_utc();
                srv.call(req).map(move |res| {
                    let time = OffsetDateTime::now_utc() - offset;
                    if let Ok(ref res) = res {
                        let status = res.response().status();
                        track_metrics(status.into(), &method, &path, time.as_seconds_f64());
                    }
                    res
                })
            })
            .service(health)
            .service(locate)
            .service(metrics)
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await?;
    Ok(())
}
