# Copyright The OpenTelemetry Authors
# SPDX-License-Identifier: Apache-2.0
# Copyright 2023 The OpenTelemetry Authors
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

FROM alpine

RUN apk update
RUN apk add --no-cache curl
RUN apk add git cmake make g++ grpc-dev re2-dev protobuf-dev c-ares-dev

ARG OPENTELEMETRY_CPP_VERSION=1.18.0
ARG PROTOBUF_VERSION=21.12

RUN curl -LO https://github.com/protocolbuffers/protobuf/archive/refs/tags/v21.12.tar.gz \
    && tar -xzf v21.12.tar.gz \
    && cd protobuf-21.12/cmake \
    && cmake .. -DBUILD_SHARED_LIBS=ON -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/usr/local -Dprotobuf_BUILD_TESTS=OFF \
    && make -j$(nproc) \
    && make install \
    && cd ../.. \
    && rm -rf v21.12.tar.gz protobuf-21.12




RUN git clone --depth 1 --branch v${OPENTELEMETRY_CPP_VERSION} https://github.com/open-telemetry/opentelemetry-cpp \
    && cd opentelemetry-cpp/ \
    && mkdir build \
    && cd build \
    && cmake .. -DCMAKE_CXX_STANDARD=17 -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
          -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF \
          -DWITH_EXAMPLES=OFF -DWITH_OTLP_GRPC=ON -DWITH_ABSEIL=ON \
    && make -j$(nproc || sysctl -n hw.ncpu || echo 1) install && cd ../..

COPY . /currencyservice

RUN cd /currencyservice \
    && rm -rf build \
	&& mkdir -p build && cd build \
    && cmake .. \
    && make -j$(nproc || sysctl -n hw.ncpu || echo 1) install

EXPOSE ${CURRENCY_SERVICE_PORT}
ENTRYPOINT ./usr/local/bin/currencyservice ${CURRENCY_SERVICE_PORT}
