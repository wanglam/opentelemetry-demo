# Shipping Service - OpenTelemetry Metrics Export

## Issue Resolution

You were correct - the previous implementation was only logging CPU metrics as JSON logs, **not sending them as actual OpenTelemetry metrics**. I've now fixed this to properly export metrics via the OpenTelemetry protocol.

## What's Changed

### 1. **Proper OpenTelemetry Metrics Pipeline**
- Re-enabled metrics features in OpenTelemetry dependencies
- Added proper meter provider initialization
- Configured OTLP metrics export to your collector

### 2. **Real Metrics Export (Not Just Logs)**
The new implementation creates actual OpenTelemetry metrics:

```rust
// Creates real OTel gauges, not just logs
let system_cpu_gauge = meter
    .f64_gauge("system_cpu_usage")
    .with_description("System CPU usage percentage")
    .init();

// Records actual metric values
system_cpu_gauge.record(cpu_usage, &[
    KeyValue::new("service.name", "shippingservice"),
]);
```

### 3. **Metrics Exported to Your Index**

The following metrics will now appear in your metrics index:

#### `system_cpu_usage`
- **Type**: Gauge (f64)
- **Description**: System-wide CPU usage percentage
- **Labels**: 
  - `service.name`: "shippingservice"
  - `metric.type`: "system_cpu"

#### `process_cpu_usage`
- **Type**: Gauge (f64) 
- **Description**: Process-specific CPU usage percentage
- **Labels**:
  - `service.name`: "shippingservice"
  - `process.pid`: Process ID
  - `process.name`: "shippingservice"
  - `metric.type`: "process_cpu"

#### `process_memory_usage`
- **Type**: Gauge (u64)
- **Description**: Process memory usage in bytes
- **Labels**:
  - `service.name`: "shippingservice"
  - `process.pid`: Process ID
  - `process.name`: "shippingservice"
  - `metric.type`: "process_memory"

## Configuration

### **Environment Variables**
The metrics will be sent to your OTEL collector using:
- `OTEL_EXPORTER_OTLP_ENDPOINT` (defaults to "http://otelcol:4317")
- Metrics are exported every 5 seconds

### **Metric Collection Frequency**
- Metrics are collected and recorded every 5 seconds
- Background refresh ensures accurate CPU measurements

## Verification Steps

After rebuilding and deploying:

1. **Check Service Logs**
   ```
   INFO Metrics provider initialized
   INFO CPU metrics collection started
   INFO Recorded metrics - System CPU: 15.20%, Process CPU: 2.10%, Memory: 45678912 bytes
   ```

2. **Check Your Metrics Index**
   Look for these metric names:
   - `system_cpu_usage`
   - `process_cpu_usage` 
   - `process_memory_usage`

3. **Verify OTEL Collector**
   Check your OTEL collector logs for incoming metrics from the shipping service

4. **Query Your Metrics Backend**
   ```promql
   # Example Prometheus queries
   system_cpu_usage{service_name="shippingservice"}
   process_cpu_usage{service_name="shippingservice"}
   process_memory_usage{service_name="shippingservice"}
   ```

## Build and Deploy

```bash
# Rebuild the shipping service
docker compose build --no-cache shippingservice

# Restart the service
docker compose up -d shippingservice
```

## Troubleshooting

If metrics still don't appear:

1. **Check OTEL Collector Configuration**
   - Ensure it's configured to receive OTLP metrics
   - Verify the endpoint is accessible from the container

2. **Check Network Connectivity**
   - Verify shipping service can reach the OTEL collector
   - Check for any firewall or network issues

3. **Enable Debug Logging**
   - Add debug logs to see if metrics are being sent
   - Check OTEL collector debug logs

The key difference now is that these are **real OpenTelemetry metrics** being sent via OTLP protocol to your metrics backend, not just log entries. They should appear in your metrics index alongside other service metrics.
