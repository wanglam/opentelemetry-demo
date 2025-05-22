// Copyright The OpenTelemetry Authors
// SPDX-License-Identifier: Apache-2.0
package kafka

import (
	"context"
	"go.opentelemetry.io/otel/trace"
    "go.opentelemetry.io/otel/propagation"

	"time"
	"fmt"
	"os"
	pb "github.com/open-telemetry/opentelemetry-demo/src/accountingservice/genproto/oteldemo"

	"github.com/IBM/sarama"
	"github.com/sirupsen/logrus"
	"google.golang.org/protobuf/proto"
)

var log1 *logrus.Logger

func init() {
	log1 = logrus.New()
	log1.Level = logrus.DebugLevel
	log1.Formatter = &logrus.JSONFormatter{
		FieldMap: logrus.FieldMap{
			logrus.FieldKeyTime:  "time",
			logrus.FieldKeyLevel: "severity",
			logrus.FieldKeyMsg:   "message",
		},
		TimestampFormat: time.RFC3339Nano,
	}
	log1.Out = os.Stdout
}

func WithTraceFields(ctx context.Context) logrus.Fields {
    span := trace.SpanFromContext(ctx)
    sc := span.SpanContext()
    if !sc.IsValid() {
        return logrus.Fields{}
    }
    return logrus.Fields{
        "trace_id": sc.TraceID().String(),
        "span_id":  sc.SpanID().String(),
    }
}

type kafkaHeaderCarrier map[string]string

func (c kafkaHeaderCarrier) Get(key string) string {
    return c[key]
}

func (c kafkaHeaderCarrier) Set(key, value string) {
    c[key] = value
}

func (c kafkaHeaderCarrier) Keys() []string {
    keys := make([]string, 0, len(c))
    for k := range c {
        keys = append(keys, k)
    }
    return keys
}


var (
	Topic           = "orders"
	ProtocolVersion = sarama.V3_0_0_0
	GroupID         = "accountingservice"
)

func StartConsumerGroup(ctx context.Context, brokers []string, log *logrus.Logger) error {
	saramaConfig := sarama.NewConfig()
	saramaConfig.Version = ProtocolVersion
	// So we can know the partition and offset of messages.
	saramaConfig.Producer.Return.Successes = true
	saramaConfig.Consumer.Interceptors = []sarama.ConsumerInterceptor{NewOTelInterceptor()}

	consumerGroup, err := sarama.NewConsumerGroup(brokers, GroupID, saramaConfig)
	if err != nil {
		return err
	}

	handler := groupHandler{
		log: log,
	}

	err = consumerGroup.Consume(ctx, []string{Topic}, &handler)
	if err != nil {
		return err
	}
	return nil
}

type groupHandler struct {
	log *logrus.Logger
}

func (g *groupHandler) Setup(_ sarama.ConsumerGroupSession) error {
	return nil
}

func (g *groupHandler) Cleanup(_ sarama.ConsumerGroupSession) error {
	return nil
}

func (g *groupHandler) ConsumeClaim(session sarama.ConsumerGroupSession, claim sarama.ConsumerGroupClaim) error {
	for {
		select {
		case message := <-claim.Messages():
			orderResult := pb.OrderResult{}
			err := proto.Unmarshal(message.Value, &orderResult)
			if err != nil {
				return err
			}

			//log.Printf("Message claimed: orderId = %s, timestamp = %v, topic = %s", orderResult.OrderId, message.Timestamp, message.Topic)
			// Extract trace context from message headers
            carrier := make(kafkaHeaderCarrier)
            for _, header := range message.Headers {
                carrier[string(header.Key)] = string(header.Value)
            }
            propagator := propagation.TraceContext{}
            ctx := propagator.Extract(context.Background(), carrier)

			msg := fmt.Sprintf("Message claimed: orderId = %s, timestamp = %v, topic = %s", orderResult.OrderId,
message.Timestamp, message.Topic)
			log1.WithFields(WithTraceFields(ctx)).Infof(msg)

			session.MarkMessage(message, "")

		case <-session.Context().Done():
			return nil
		}
	}
}
