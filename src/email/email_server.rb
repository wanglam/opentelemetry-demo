# Copyright The OpenTelemetry Authors
# SPDX-License-Identifier: Apache-2.0

require "ostruct"
require "pony"
require "sinatra"
require 'json'
require 'time'  # for ISO 8601 formatting
require 'logger'

require "opentelemetry/sdk"
require "opentelemetry/exporter/otlp"
require "opentelemetry/instrumentation/sinatra"

set :port, ENV["EMAIL_PORT"]

OpenTelemetry::SDK.configure do |c|
  c.use "OpenTelemetry::Instrumentation::Sinatra"
end

# Try to set up ForwardingLogger for OpenTelemetry log integration
begin
  # Create a ForwardingLogger that sends logs to OpenTelemetry
  otel_logger = OpenTelemetry::SDK::ForwardingLogger.new(Logger.new(STDOUT))
  puts "ForwardingLogger initialized successfully"
rescue => e
  # Fallback to regular logger if ForwardingLogger is not available
  otel_logger = Logger.new(STDOUT)
  otel_logger.formatter = proc do |severity, datetime, progname, msg|
    {
      timestamp: datetime.utc.iso8601(3),
      level: severity,
      service: "email-service",
      message: msg
    }.to_json + "\n"
  end
  puts "Using fallback logger: #{e.message}"
end

# Enhanced logging using OpenTelemetry ForwardingLogger
def log_with_otel_logger(message, attributes = {}, level: :info)
  # Create structured log message
  log_data = {
    message: message,
    service: "email-service"
  }.merge(attributes)
  
  # Add trace correlation
  current_span = OpenTelemetry::Trace.current_span
  span_context = current_span.context
  if span_context.valid?
    log_data[:trace_id] = span_context.trace_id.unpack1('H*')
    log_data[:span_id] = span_context.span_id.unpack1('H*')
  end
  
  # Use the OpenTelemetry logger
  case level
  when :debug
    otel_logger.debug(log_data.to_json)
  when :info
    otel_logger.info(log_data.to_json)
  when :warn
    otel_logger.warn(log_data.to_json)
  when :error
    otel_logger.error(log_data.to_json)
  when :fatal
    otel_logger.fatal(log_data.to_json)
  else
    otel_logger.info(log_data.to_json)
  end
end

post "/send_order_confirmation" do
  data = JSON.parse(request.body.read, object_class: OpenStruct)

  # get the current auto-instrumented span
  current_span = OpenTelemetry::Trace.current_span
  current_span.add_attributes({
    "app.order.id" => data.order.order_id,
  })

  # Log request received
  log_with_otel_logger("Email request received", {
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
    
    log_with_otel_logger("Email sending failed", {
      "app.order.id" => data.order.order_id,
      "app.email.recipient" => data.email,
      "error" => e.message
    }, level: :error)
    
    status 500
    { error: "Failed to send email" }.to_json
  end
end

error do
  OpenTelemetry::Trace.current_span.record_exception(env['sinatra.error'])
  
  # Log the error with trace correlation
  log_with_otel_logger("Unhandled error in email service", {
    "error" => env['sinatra.error'].message,
    "error_class" => env['sinatra.error'].class.name
  }, level: :error)
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
      log_with_otel_logger("Order confirmation email sent", {
        "app.email.recipient" => data.email,
        "app.order.id" => data.order.order_id
      })
      
    rescue => e
      span.record_exception(e)
      span.set_status(OpenTelemetry::Trace::Status::ERROR, e.message)
      
      # Log email sending failure
      log_with_otel_logger("Failed to send order confirmation email", {
        "app.email.recipient" => data.email,
        "app.order.id" => data.order.order_id,
        "error" => e.message
      }, level: :error)
      
      raise e  # Re-raise the exception to be handled by the caller
    end
  end
end
