# CRM

## Overview

This project implements a CRM service using Rust and gRPC

## Projects

1. [Load Balancer](./load-balancer): A simple load balancer service using pingora.
2. [CRM](./crm): A Customer Relationship Management (CRM) service.
3. [User Stat](./user-stat): A service for collecting and analyzing user statistics.
4. [Notification](./crm-notification): A service for sending notifications(email, sms, in-app) to users.
5. [Metadata](./crm-metadata): A service for storing metadata.
6. [Core](./crm-core): A core library for shared functionality.

## Features

- gRPC-based communication
- Multiple interconnected services
- pingora for load balancing
- opentelemetry for tracing

## Prerequisites

- Rust (latest **nightly** version)
- Protocol Buffers compiler (protoc)
