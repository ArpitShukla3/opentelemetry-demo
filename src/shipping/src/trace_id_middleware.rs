// Copyright The OpenTelemetry Authors
// SPDX-License-Identifier: Apache-2.0

use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error, HttpMessage};
use actix_web::body::BoxBody;
use opentelemetry::global;
use std::future::Future;
use std::pin::Pin;

pub struct TraceIdMiddleware;

impl<S> actix_web::dev::Transform<S, ServiceRequest> for TraceIdMiddleware
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type InitError = ();
    type Transform = TraceIdMiddlewareInner<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(TraceIdMiddlewareInner { service }))
    }
}

pub struct TraceIdMiddlewareInner<S> {
    service: S,
}

impl<S> actix_web::dev::Service<ServiceRequest> for TraceIdMiddlewareInner<S>
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let fut = self.service.call(req);

        Box::pin(async move {
            let mut res = fut.await?;
            
            // Get trace_id from the current span using global tracer
            let tracer = global::tracer("shipping");
            let span = tracer.start("trace_id_middleware");
            let trace_id = span.span_context().trace_id().to_hex();
            
            if !trace_id.is_empty() {
                res.headers_mut().insert(
                    "x-trace-id".parse().unwrap(),
                    trace_id.parse().unwrap(),
                );
            }
            
            Ok(res)
        })
    }
}
