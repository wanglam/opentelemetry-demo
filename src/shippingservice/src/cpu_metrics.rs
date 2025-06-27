// Copyright The OpenTelemetry Authors
// SPDX-License-Identifier: Apache-2.0

use sysinfo::{System, SystemExt, ProcessExt, CpuExt, PidExt};
use std::time::Duration;
use tokio::time::interval;
use log::*;
use serde_json::json;
use chrono::Utc;

pub async fn start_cpu_metrics_collection() {
    let mut system = System::new_all();
    let current_pid = match sysinfo::get_current_pid() {
        Ok(pid) => pid,
        Err(e) => {
            error!("Failed to get current process ID: {}", e);
            return;
        }
    };
    
    let mut interval = interval(Duration::from_secs(5)); // Collect every 5 seconds
    info!("Starting CPU metrics collection for shipping service (PID: {})", current_pid.as_u32());
    
    loop {
        interval.tick().await;
        system.refresh_all();
        
        let timestamp = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        
        // Collect system CPU usage
        let system_cpu_usage = system.global_cpu_info().cpu_usage();
        
        // Collect process-specific CPU usage
        if let Some(process) = system.process(current_pid) {
            let process_cpu_usage = process.cpu_usage();
            let memory_usage = process.memory();
            
            // Log CPU metrics in structured format for observability
            let cpu_metrics = json!({
                "timestamp": timestamp,
                "service": "shippingservice",
                "metrics": {
                    "system_cpu_usage_percent": system_cpu_usage,
                    "process_cpu_usage_percent": process_cpu_usage,
                    "process_memory_bytes": memory_usage,
                    "process_pid": current_pid.as_u32()
                },
                "metric_type": "cpu_usage"
            });
            
            info!("CPU_METRICS: {}", cpu_metrics.to_string());
            
            // Also log at debug level for more detailed monitoring
            debug!("CPU Usage - System: {:.2}%, Process: {:.2}%, Memory: {} bytes", 
                   system_cpu_usage, process_cpu_usage, memory_usage);
        } else {
            warn!("Could not find process with PID: {}", current_pid.as_u32());
        }
    }
}
