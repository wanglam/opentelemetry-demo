#!/bin/bash

echo "Testing shipping service build with CPU metrics..."

cd src/shippingservice

echo "Building shipping service..."
docker build -t test-shipping-service .

if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
    echo "The shipping service now includes CPU metrics that will be sent via OpenTelemetry."
    echo ""
    echo "Metrics exported:"
    echo "- system_cpu_usage (gauge): System-wide CPU usage percentage"
    echo "- process_cpu_usage (gauge): Process-specific CPU usage percentage" 
    echo "- process_memory_usage (gauge): Process memory usage in bytes"
    echo ""
    echo "These metrics will be sent to your OTEL collector endpoint and should appear in your metrics index."
else
    echo "❌ Build failed. Check the error messages above."
fi
