# Copyright The OpenTelemetry Authors
# SPDX-License-Identifier: Apache-2.0


FROM golang:1.21.3-alpine AS builder
WORKDIR /usr/src/app/

# Install git (and any other needed tools)
RUN apk add --no-cache git

COPY ./src/checkoutservice/ ./
RUN GOPROXY=direct go build -o /go/bin/checkoutservice/

# -----------------------------------------------------------------------------

FROM alpine

WORKDIR /usr/src/app/

COPY --from=builder /go/bin/checkoutservice/ ./

EXPOSE ${CHECKOUT_SERVICE_PORT}
ENTRYPOINT [ "./checkoutservice" ]
