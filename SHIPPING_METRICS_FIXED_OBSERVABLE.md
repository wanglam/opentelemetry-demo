# Shipping Service CPU Metrics - Fixed with Observable Gauges

## Issue Resolution

You were correct about the API compatibility issue. OpenTelemetry 0.20 doesn't have `f64_gauge()` method. I've fixed this by using **observable gauges** with callbacks, which is the correct API for this version.

## What's Fixed

### 1. **Correct OpenTelemetry 0.20 API Usage**
```rust
// OLD (doesn't exist in 0.20):
let gauge = meter.f64_gauge("metric_name").init();
gauge.record(value, &labels);

// NEW (correct for 0.20):
let _gauge = meter
    .f64_observable_gauge("metric_name")
    .with_description("Description")
    .with_callback(move |observer| {
        observer.observe(value, &labels);
    })
    .init();
```

### 2. **Shared State Management**
- Created `CpuMetricsState` struct to hold current metric values
- Uses `Arc<Mutex<>>` for thread-safe access between callback and refresh task
- Background task updates the state, callbacks read from it

### 3. **Observable Gauges with Callbacks**
The implementation now uses three observable gauges:

#### `system_cpu_usage` (f64_observable_gauge)
- Callback reads current system CPU usage from shared state
- Labels: `service.name`, `metric.type`

#### `process_cpu_usage` (f64_observable_gauge)  
- Callback reads current process CPU usage from shared state
- Labels: `service.name`, `process.pid`, `process.name`, `metric.type`

#### `process_memory_usage` (u64_observable_gauge)
- Callback reads current process memory usage from shared state
- Labels: `service.name`, `process.pid`, `process.name`, `metric.type`

## How It Works

1. **Initialization**: Creates shared state with system info and current PID
2. **Observable Gauge Registration**: Registers gauges with callbacks that read from shared state
3. **Background Refresh**: Updates shared state every 5 seconds with fresh CPU/memory data
4. **Automatic Export**: OpenTelemetry automatically calls callbacks and exports metrics

## Key Benefits

### ✅ **API Compatibility**
- Uses correct OpenTelemetry 0.20 observable gauge API
- No deprecated or non-existent method calls
- Follows OpenTelemetry Rust SDK patterns

### ✅ **Efficient Resource Usage**
- Single background task for data collection
- Callbacks are lightweight (just read from memory)
- No blocking operations in metric callbacks

### ✅ **Thread Safety**
- Uses `Arc<Mutex<>>` for safe concurrent access
- Background task updates, callbacks read
- No race conditions or data corruption

### ✅ **Proper Metric Export**
- Metrics are automatically exported via OTLP
- Follows OpenTelemetry semantic conventions
- Includes proper labels and descriptions

## Expected Behavior

### **Service Logs**
```
INFO Metrics provider initialized
INFO CPU metrics observable gauges registered successfully  
INFO Starting CPU metrics collection for shipping service (PID: 1234)
INFO Updated metrics - System CPU: 15.20%, Process CPU: 2.10%, Memory: 45678912 bytes
```

### **Metrics in Your Index**
You should now see these metrics with proper values:
- `system_cpu_usage{service_name="shippingservice",metric_type="system_cpu"}`
- `process_cpu_usage{service_name="shippingservice",process_pid="1234",metric_type="process_cpu"}`
- `process_memory_usage{service_name="shippingservice",process_pid="1234",metric_type="process_memory"}`

## Build and Deploy

```bash
# This should now build successfully
docker compose build --no-cache shippingservice

# Deploy the updated service
docker compose up -d shippingservice
```

## Verification

1. **Check Build Success**: No more compilation errors
2. **Check Service Logs**: Look for "observable gauges registered successfully"
3. **Check Metrics Backend**: Metrics should appear in your index within 5-10 seconds
4. **Verify Values**: CPU and memory values should be realistic and updating

The key difference is using **observable gauges with callbacks** instead of direct gauge recording, which is the correct pattern for OpenTelemetry 0.20 Rust SDK.
