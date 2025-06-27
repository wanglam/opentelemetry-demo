// Copyright The OpenTelemetry Authors
// SPDX-License-Identifier: Apache-2.0

use opentelemetry::{global, KeyValue};
use sysinfo::{System, SystemExt, ProcessExt, CpuExt, PidExt};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::interval;
use log::*;

// Shared state for CPU metrics
struct CpuMetricsState {
    system: System,
    current_pid: sysinfo::Pid,
    system_cpu_usage: f64,
    process_cpu_usage: f64,
    process_memory_usage: u64,
}

impl CpuMetricsState {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let current_pid = sysinfo::get_current_pid()
            .map_err(|e| format!("Failed to get current process ID: {}", e))?;
        
        Ok(CpuMetricsState {
            system: System::new_all(),
            current_pid,
            system_cpu_usage: 0.0,
            process_cpu_usage: 0.0,
            process_memory_usage: 0,
        })
    }

    fn refresh(&mut self) {
        self.system.refresh_all();
        
        // Update system CPU usage
        self.system_cpu_usage = self.system.global_cpu_info().cpu_usage() as f64;
        
        // Update process-specific metrics
        if let Some(process) = self.system.process(self.current_pid) {
            self.process_cpu_usage = process.cpu_usage() as f64;
            self.process_memory_usage = process.memory();
        }
    }
}

pub async fn start_cpu_metrics_collection() {
    let cpu_state = match CpuMetricsState::new() {
        Ok(state) => Arc::new(Mutex::new(state)),
        Err(e) => {
            error!("Failed to initialize CPU metrics: {}", e);
            return;
        }
    };

    let meter = global::meter("shippingservice");
    
    // Create observable gauges with callbacks
    let state_clone1 = Arc::clone(&cpu_state);
    let _system_cpu_gauge = meter
        .f64_observable_gauge("system_cpu_usage")
        .with_description("System CPU usage percentage")
        .with_callback(move |observer| {
            if let Ok(state) = state_clone1.lock() {
                observer.observe(
                    state.system_cpu_usage,
                    &[
                        KeyValue::new("service.name", "shippingservice"),
                        KeyValue::new("metric.type", "system_cpu"),
                    ],
                );
            }
        })
        .init();

    let state_clone2 = Arc::clone(&cpu_state);
    let current_pid = cpu_state.lock().unwrap().current_pid;
    let _process_cpu_gauge = meter
        .f64_observable_gauge("process_cpu_usage")
        .with_description("Process CPU usage percentage")
        .with_callback(move |observer| {
            if let Ok(state) = state_clone2.lock() {
                observer.observe(
                    state.process_cpu_usage,
                    &[
                        KeyValue::new("service.name", "shippingservice"),
                        KeyValue::new("process.pid", current_pid.as_u32() as i64),
                        KeyValue::new("process.name", "shippingservice"),
                        KeyValue::new("metric.type", "process_cpu"),
                    ],
                );
            }
        })
        .init();

    let state_clone3 = Arc::clone(&cpu_state);
    let _process_memory_gauge = meter
        .u64_observable_gauge("process_memory_usage")
        .with_description("Process memory usage in bytes")
        .with_callback(move |observer| {
            if let Ok(state) = state_clone3.lock() {
                observer.observe(
                    state.process_memory_usage,
                    &[
                        KeyValue::new("service.name", "shippingservice"),
                        KeyValue::new("process.pid", current_pid.as_u32() as i64),
                        KeyValue::new("process.name", "shippingservice"),
                        KeyValue::new("metric.type", "process_memory"),
                    ],
                );
            }
        })
        .init();

    info!("CPU metrics observable gauges registered successfully");

    // Background task to refresh the metrics data
    let mut interval = interval(Duration::from_secs(5));
    info!("Starting CPU metrics collection for shipping service (PID: {})", current_pid.as_u32());
    
    loop {
        interval.tick().await;
        
        if let Ok(mut state) = cpu_state.lock() {
            state.refresh();
            info!("Updated metrics - System CPU: {:.2}%, Process CPU: {:.2}%, Memory: {} bytes", 
                   state.system_cpu_usage, state.process_cpu_usage, state.process_memory_usage);
        }
    }
}
