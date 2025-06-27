// Copyright The OpenTelemetry Authors
// SPDX-License-Identifier: Apache-2.0

use opentelemetry::{global, KeyValue};
use sysinfo::{System, SystemExt, ProcessExt, CpuExt, PidExt};
use std::time::Duration;
use tokio::time::interval;
use log::*;

pub async fn start_cpu_metrics_collection() {
    let meter = global::meter("shippingservice");
    
    // Create gauges for CPU metrics
    let system_cpu_gauge = meter
        .f64_gauge("system_cpu_usage")
        .with_description("System CPU usage percentage")
        .init();

    let process_cpu_gauge = meter
        .f64_gauge("process_cpu_usage") 
        .with_description("Process CPU usage percentage")
        .init();

    let process_memory_gauge = meter
        .u64_gauge("process_memory_usage")
        .with_description("Process memory usage in bytes")
        .init();

    let mut system = System::new_all();
    let current_pid = match sysinfo::get_current_pid() {
        Ok(pid) => pid,
        Err(e) => {
            error!("Failed to get current process ID: {}", e);
            return;
        }
    };
    
    let mut interval = interval(Duration::from_secs(5));
    info!("Starting CPU metrics collection for shipping service (PID: {})", current_pid.as_u32());
    
    loop {
        interval.tick().await;
        system.refresh_all();
        
        // Collect and record system CPU usage
        let system_cpu_usage = system.global_cpu_info().cpu_usage() as f64;
        system_cpu_gauge.record(system_cpu_usage, &[
            KeyValue::new("service.name", "shippingservice"),
            KeyValue::new("metric.type", "system_cpu"),
        ]);
        
        // Collect and record process-specific metrics
        if let Some(process) = system.process(current_pid) {
            let process_cpu_usage = process.cpu_usage() as f64;
            let memory_usage = process.memory();
            
            process_cpu_gauge.record(process_cpu_usage, &[
                KeyValue::new("service.name", "shippingservice"),
                KeyValue::new("process.pid", current_pid.as_u32() as i64),
                KeyValue::new("process.name", "shippingservice"),
                KeyValue::new("metric.type", "process_cpu"),
            ]);
            
            process_memory_gauge.record(memory_usage, &[
                KeyValue::new("service.name", "shippingservice"),
                KeyValue::new("process.pid", current_pid.as_u32() as i64),
                KeyValue::new("process.name", "shippingservice"),
                KeyValue::new("metric.type", "process_memory"),
            ]);
            
            info!("Recorded metrics - System CPU: {:.2}%, Process CPU: {:.2}%, Memory: {} bytes", 
                   system_cpu_usage, process_cpu_usage, memory_usage);
        } else {
            warn!("Could not find process with PID: {}", current_pid.as_u32());
        }
    }
}
