# CPU Metrics Implementation for Shipping Service

## Overview
This document describes the CPU metrics implementation added to the shipping service, similar to the implementation in the checkout service.

## Changes Made

### 1. Dependencies Added
- Added `sysinfo = "0.29"` to Cargo.toml for system information collection
- Added `metrics` feature to `opentelemetry` dependency
- Added `metrics` feature to `opentelemetry-otlp` dependency

### 2. New Module: cpu_metrics.rs
Created a dedicated module for CPU metrics collection that:
- Collects system-wide CPU usage percentage
- Collects process-specific CPU usage percentage
- Updates metrics every second
- Uses OpenTelemetry observable gauges for metric reporting

### 3. Main.rs Updates
- Added meter provider initialization
- Added CPU metrics collection as a background task
- Integrated with existing OpenTelemetry pipeline

## Metrics Exported

### system.cpu.usage
- **Type**: Observable Gauge (f64)
- **Unit**: %
- **Description**: System CPU usage percentage
- **Labels**: 
  - `service.name`: "shippingservice"

### process.cpu.usage
- **Type**: Observable Gauge (f64)
- **Unit**: %
- **Description**: Process CPU usage percentage
- **Labels**:
  - `process.pid`: Process ID
  - `process.name`: "shippingservice"
  - `service.name`: "shippingservice"

## Implementation Details

The CPU metrics collection:
1. Runs as a background tokio task
2. Updates metrics every 1 second
3. Uses the `sysinfo` crate for cross-platform system information
4. Follows OpenTelemetry semantic conventions
5. Includes proper error handling and logging

## Comparison with Checkout Service

The implementation follows the same pattern as the checkout service:
- Uses OpenTelemetry metrics pipeline
- Collects both system and process CPU metrics
- Runs as a background task
- Exports metrics via OTLP

The main difference is the language (Rust vs Go) and the system information library used (`sysinfo` vs Go's `runtime` package).

## Testing

To verify the metrics are being collected:
1. Build and run the shipping service
2. Check the logs for "CPU metrics collection started"
3. Monitor the OpenTelemetry collector for incoming metrics
4. Verify metrics appear in your observability backend (Prometheus, Grafana, etc.)

## Environment Variables

The metrics endpoint can be configured using:
- `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT`: Defaults to "http://otelcol:4317/v1/metrics"
