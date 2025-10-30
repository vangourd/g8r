use anyhow::Result;
use opentelemetry::global;
use std::fs::OpenOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_telemetry() -> Result<()> {
    let exporter_type = std::env::var("OTEL_EXPORTER").unwrap_or_else(|_| "stdout".to_string());
    
    match exporter_type.as_str() {
        "jaeger" => init_jaeger()?,
        "otlp" => init_otlp()?,
        "file" => init_file()?,
        "stdout" => init_stdout()?,
        _ => {
            eprintln!("Unknown OTEL_EXPORTER: {}, falling back to stdout", exporter_type);
            init_stdout()?;
        }
    }
    
    Ok(())
}

fn init_jaeger() -> Result<()> {
    let service_name = std::env::var("OTEL_SERVICE_NAME")
        .unwrap_or_else(|_| "g8r".to_string());
    
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name(service_name)
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;
    
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    
    tracing_subscriber::registry()
        .with(telemetry)
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    Ok(())
}

fn init_otlp() -> Result<()> {
    let service_name = std::env::var("OTEL_SERVICE_NAME")
        .unwrap_or_else(|_| "g8r".to_string());
    
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .with_trace_config(
            opentelemetry_sdk::trace::config().with_resource(
                opentelemetry_sdk::Resource::new(vec![
                    opentelemetry::KeyValue::new("service.name", service_name),
                ])
            )
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;
    
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    
    tracing_subscriber::registry()
        .with(telemetry)
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    Ok(())
}

fn init_stdout() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    Ok(())
}

fn init_file() -> Result<()> {
    let log_file_path = std::env::var("LOG_FILE")
        .unwrap_or_else(|_| "g8r.log".to_string());
    
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)?;
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::sync::Arc::new(log_file))
                .with_ansi(false)
        )
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    eprintln!("Logging to file: {}", log_file_path);
    
    Ok(())
}

pub fn shutdown_telemetry() {
    global::shutdown_tracer_provider();
}
