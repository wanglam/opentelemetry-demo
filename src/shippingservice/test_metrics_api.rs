// Test file to verify OpenTelemetry metrics API compatibility
use opentelemetry::{global, KeyValue};

fn test_observable_gauge_api() {
    let meter = global::meter("test");
    
    // Test if f64_observable_gauge exists
    let _gauge = meter
        .f64_observable_gauge("test_metric")
        .with_description("Test metric")
        .with_callback(|observer| {
            observer.observe(42.0, &[KeyValue::new("test", "value")]);
        })
        .init();
    
    println!("Observable gauge API test passed");
}

fn main() {
    test_observable_gauge_api();
}
