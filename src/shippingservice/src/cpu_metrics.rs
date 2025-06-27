// Copyright The OpenTelemetry Authors
// SPDX-License-Identifier: Apache-2.0

use opentelemetry::{global, KeyValue};
use sysinfo::{System, SystemExt, ProcessExt, CpuExt};
use std::time::Duration;
use tokio::time::interval;
use log::*;

pub async fn start_cpu_metrics_collection() {
    let meter = global::meter("shippingservice");
    
    // Create observable gauges for CPU metrics
    let system_cpu_gauge = meter
        .f64_observable_gauge("system.cpu.usage")
        .with_description("System CPU usage percentage")
        .with_unit("%")
        .init();

    let process_cpu_gauge = meter
        .f64_observable_gauge("process.cpu.usage")
        .with_description("Process CPU usage percentage")
        .with_unit("%")
        .init();

    let mut system = System::new_all();
    let current_pid = match sysinfo::get_current_pid() {
        Ok(pid) => pid,
        Err(e) => {
            error!("Failed to get current process ID: {}", e);
            return;
        }
    };
    
    let mut interval = interval(Duration::from_secs(1));
    info!("Starting CPU metrics collection for shipping service");
    
    loop {
        interval.tick().await;
        system.refresh_all();
        
        // Collect system CPU usage
        let system_cpu_usage = system.global_cpu_info().cpu_usage() as f64;
        system_cpu_gauge.observe(system_cpu_usage, &[
            KeyValue::new("service.name", "shippingservice"),
        ]);
        
        // Collect process-specific CPU usage
        if let Some(process) = system.process(current_pid) {
            let process_cpu_usage = process.cpu_usage() as f64;
            process_cpu_gauge.observe(process_cpu_usage, &[
                KeyValue::new("process.pid", current_pid.as_u32() as i64),
                KeyValue::new("process.name", "shippingservice"),
                KeyValue::new("service.name", "shippingservice"),
            ]);
            
            debug!("CPU metrics - System: {:.2}%, Process: {:.2}%", 
                   system_cpu_usage, process_cpu_usage);
        } else {
            warn!("Could not find process with PID: {}", current_pid);
        }
    }
}
