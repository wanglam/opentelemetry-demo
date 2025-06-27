# Container vs Host CPU Metrics - Important Clarification

## The Reality of Container CPU Metrics

When the shipping service runs in a Docker container, the CPU metrics you're getting are **container-scoped**, not host system metrics.

### What You're Actually Measuring

#### `container_cpu_usage` (renamed from system_cpu_usage)
- **Scope**: CPU usage within the container's namespace
- **NOT**: Host system CPU usage
- **Reflects**: Container's view of CPU resources
- **Limited by**: Docker CPU limits/constraints if set

#### `process_cpu_usage`
- **Scope**: Shipping service process within the container
- **This is accurate** for monitoring the service performance
- **Most useful** for application monitoring

#### `process_memory_usage`
- **Scope**: Process memory within container
- **Accurate** for monitoring service memory consumption

## Why Container CPU ≠ Host CPU

### Docker Isolation
```
Host System (100% CPU available)
├── Container 1 (sees its own "system" CPU)
├── Container 2 (sees its own "system" CPU)  
└── Container N (sees its own "system" CPU)
```

Each container has an isolated view of system resources.

### What sysinfo Reports in Container
```rust
// Inside container - this is NOT host CPU
system.global_cpu_info().cpu_usage() // Container's CPU view
```

## Recommended Monitoring Strategy

### 1. **For Service Health** (Current Implementation)
✅ **Keep these metrics** - they're valuable for service monitoring:
- `process_cpu_usage` - Service performance
- `process_memory_usage` - Service memory consumption
- `container_cpu_usage` - Container resource usage

### 2. **For Host System Monitoring**
Use dedicated host monitoring:

#### Option A: Node Exporter (Recommended)
```yaml
# docker-compose.yml
node-exporter:
  image: prom/node-exporter
  ports:
    - "9100:9100"
  volumes:
    - /proc:/host/proc:ro
    - /sys:/host/sys:ro
  command:
    - '--path.procfs=/host/proc'
    - '--path.sysfs=/host/sys'
```

#### Option B: Host Agent
- Datadog Agent on host
- New Relic Infrastructure agent
- Cloud provider monitoring (CloudWatch, Azure Monitor, etc.)

### 3. **For Container Resource Limits**
Monitor Docker stats:
```bash
docker stats shippingservice
```

## Updated Metrics Explanation

### `container_cpu_usage`
- **What it shows**: CPU usage as seen from inside the container
- **Use case**: Monitor if the container is CPU-constrained
- **Alert on**: High values might indicate need for more CPU allocation

### `process_cpu_usage`  
- **What it shows**: Shipping service CPU consumption
- **Use case**: Monitor service performance and efficiency
- **Alert on**: Unusual spikes or sustained high usage

### `process_memory_usage`
- **What it shows**: Service memory consumption
- **Use case**: Monitor for memory leaks or high memory usage
- **Alert on**: Growing memory usage or approaching limits

## Dashboard Recommendations

### Service Performance Dashboard
```promql
# Service CPU usage
process_cpu_usage{service_name="shippingservice"}

# Service memory usage  
process_memory_usage{service_name="shippingservice"}

# Container resource usage
container_cpu_usage{service_name="shippingservice"}
```

### Host System Dashboard (Separate)
```promql
# Host CPU usage (from node-exporter)
100 - (avg(rate(node_cpu_seconds_total{mode="idle"}[5m])) * 100)

# Host memory usage
(1 - (node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes)) * 100
```

## Key Takeaways

1. **Container metrics ≠ Host metrics** - This is by design for isolation
2. **Process metrics are most valuable** for service monitoring
3. **Use dedicated tools for host monitoring** (Node Exporter, etc.)
4. **Container CPU metrics are still useful** for resource allocation decisions

The current implementation is actually **correct and valuable** for service monitoring - just understand that it's container-scoped, not host-scoped.
