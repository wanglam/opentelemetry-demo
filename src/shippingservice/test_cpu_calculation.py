#!/usr/bin/env python3
"""
Test script to verify CPU calculation logic matches what we implemented in Rust
"""
import time

def read_proc_stat():
    """Read CPU stats from /proc/stat"""
    try:
        with open('/proc/stat', 'r') as f:
            first_line = f.readline().strip()
            if not first_line.startswith('cpu '):
                return None
            
            parts = first_line.split()
            if len(parts) < 9:
                return None
                
            return {
                'user': int(parts[1]),
                'nice': int(parts[2]),
                'system': int(parts[3]),
                'idle': int(parts[4]),
                'iowait': int(parts[5]),
                'irq': int(parts[6]),
                'softirq': int(parts[7]),
                'steal': int(parts[8]),
            }
    except Exception as e:
        print(f"Error reading /proc/stat: {e}")
        return None

def calculate_cpu_usage(stats1, stats2):
    """Calculate CPU usage between two stat readings"""
    if not stats1 or not stats2:
        return None
        
    # Calculate totals
    total1 = sum(stats1.values())
    total2 = sum(stats2.values())
    
    # Calculate idle totals
    idle1 = stats1['idle'] + stats1['iowait']
    idle2 = stats2['idle'] + stats2['iowait']
    
    # Calculate active totals
    active1 = total1 - idle1
    active2 = total2 - idle2
    
    # Calculate differences
    total_diff = total2 - total1
    active_diff = active2 - active1
    
    if total_diff == 0:
        return 0.0
        
    cpu_usage = (active_diff / total_diff) * 100.0
    return cpu_usage

def main():
    print("Testing CPU calculation logic...")
    print("This should match the Rust implementation")
    print("Run 'yes > /dev/null &' in another terminal to test high CPU usage")
    print("Press Ctrl+C to stop")
    
    try:
        while True:
            stats1 = read_proc_stat()
            time.sleep(2)
            stats2 = read_proc_stat()
            
            if stats1 and stats2:
                cpu_usage = calculate_cpu_usage(stats1, stats2)
                if cpu_usage is not None:
                    print(f"Container CPU Usage: {cpu_usage:.2f}%")
                else:
                    print("Failed to calculate CPU usage")
            else:
                print("Failed to read /proc/stat")
                
    except KeyboardInterrupt:
        print("\nStopped")

if __name__ == "__main__":
    main()
