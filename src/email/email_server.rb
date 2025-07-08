# Copyright The OpenTelemetry Authors
# SPDX-License-Identifier: Apache-2.0

require "ostruct"
require "pony"
require "sinatra"
require 'json'
require 'time'  # for ISO 8601 formatting

require "opentelemetry/sdk"
require "opentelemetry/exporter/otlp"
require "opentelemetry/instrumentation/sinatra"
require "opentelemetry/logs/sdk"

set :port, ENV["EMAIL_PORT"]

# Configure OpenTelemetry logging
log_exporter = OpenTelemetry::Exporter::OTLP::LogsExporter.new
log_processor = OpenTelemetry::Logs::SDK::Export::BatchLogRecordProcessor.new(log_exporter)
logger_provider = OpenTelemetry::Logs::SDK::LoggerProvider.new
logger_provider.add_log_record_processor(log_processor)
OpenTelemetry::Logs.logger_provider = logger_provider

# Get OpenTelemetry logger
otel_logger = OpenTelemetry::Logs.logger_provider.logger("email-service")

OpenTelemetry::SDK.configure do |c|
  c.use "OpenTelemetry::Instrumentation::Sinatra"
end

# Helper function for structured logging with OpenTelemetry
def log_with_otel(message, attributes = {}, severity: :info)
  # Convert severity to OpenTelemetry severity number
  severity_map = {
    debug: 5,
    info: 9,
    warn: 13,
    error: 17,
    fatal: 21
  }
  
  severity_number = severity_map[severity] || 9
  
  # Create log record
  log_record = OpenTelemetry::Logs::SDK::LogRecord.new(
    timestamp: Time.now,
    severity_number: severity_number,
    severity_text: severity.to_s.upcase,
    body: message,
    attributes: attributes
  )
  
  # Emit log record
  otel_logger.emit(log_record)
  
  # Also print to console for local development
  puts({
    time: Time.now.utc.iso8601(3),
    level: severity.to_s.upcase,
    message: message
  }.merge(attributes).to_json)
end

post "/send_order_confirmation" do
  data = JSON.parse(request.body.read, object_class: OpenStruct)

  # get the current auto-instrumented span
  current_span = OpenTelemetry::Trace.current_span
  current_span.add_attributes({
    "app.order.id" => data.order.order_id,
  })

  # Log request received
  log_with_otel("Email request received", {
    "app.order.id" => data.order.order_id,
    "app.email.recipient" => data.email
  })

  begin
    send_email(data)
    status 200
  rescue => e
    # Log error
    current_span.record_exception(e)
    current_span.set_status(OpenTelemetry::Trace::Status::ERROR, e.message)
    
    log_with_otel("Email sending failed", {
      "app.order.id" => data.order.order_id,
      "app.email.recipient" => data.email,
      "error" => e.message
    }, severity: :error)
    
    status 500
    { error: "Failed to send email" }.to_json
  end
end

error do
  OpenTelemetry::Trace.current_span.record_exception(env['sinatra.error'])
  
  # Log the error with trace correlation
  log_with_trace("Unhandled error in email service", {
    "error" => env['sinatra.error'].message,
    "error_class" => env['sinatra.error'].class.name
  }, level: "ERROR")
end

def send_email(data)
  # create and start a manual span
  tracer = OpenTelemetry.tracer_provider.tracer('email')
  tracer.in_span("send_email") do |span|
    begin
      Pony.mail(
        to:       data.email,
        from:     "noreply@example.com",
        subject:  "Your confirmation email",
        body:     erb(:confirmation, locals: { order: data.order }),
        via:      :test
      )
      
      span.set_attribute("app.email.recipient", data.email)
      
      # Log successful email sending
      log_with_trace("Order confirmation email sent", {
        "app.email.recipient" => data.email,
        "app.order.id" => data.order.order_id
      })
      
    rescue => e
      span.record_exception(e)
      span.set_status(OpenTelemetry::Trace::Status::ERROR, e.message)
      
      # Log email sending failure
      log_with_trace("Failed to send order confirmation email", {
        "app.email.recipient" => data.email,
        "app.order.id" => data.order.order_id,
        "error" => e.message
      }, level: "ERROR")
      
      raise e  # Re-raise the exception to be handled by the caller
    end
  end
end
