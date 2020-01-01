use futures::prelude::*;

use futures::task::{Context, Poll};
use hyper::server::conn::AddrStream;
use hyper::service::Service as TowerService;
use hyper::{Body, Method, Request, Response, StatusCode};
use std::convert::Infallible;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use crate::data::Payload;
use crate::ErrorData;

#[derive(Debug, Clone)]
pub(crate) struct Service {
    data: Arc<RwLock<ErrorData>>,
}

impl Service {
    pub(crate) fn new(data: &Arc<RwLock<ErrorData>>) -> Self {
        Self { data: data.clone() }
    }

    async fn serve(&self, req: Request<Body>) -> Response<Body> {
        let method = req.method().clone();
        let is_get = method == Method::GET || method == Method::HEAD;
        let is_post = method == Method::POST;

        let path = req.uri().path();

        if is_get && path == "/" {
            Response::builder()
                .status(StatusCode::MOVED_PERMANENTLY)
                .body(Body::empty())
                .unwrap()
        } else if is_post && path == "/v1/notices" {
            self.create_notice(req).await
        } else {
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap()
        }
    }

    async fn create_notice(&self, mut req: Request<Body>) -> Response<Body> {
        let body = std::mem::replace(req.body_mut(), Body::empty());
        let body = if let Ok(body) = read_body(body).await {
            body
        } else {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap();
        };
        let body = if let Ok(body) = serde_json::from_slice::<Payload>(&body) {
            body
        } else {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::empty())
                .unwrap();
        };
        let uuid = body.error.token.unwrap_or_else(Uuid::new_v4);
        {
            let mut data = self.data.write().unwrap();
            data.errors.push(body);
        }
        Response::builder()
            .status(StatusCode::CREATED)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .header("X-UUID", uuid.to_string())
            .body(Body::from(format!("{{\"id\":\"{}\"}}", uuid)))
            .unwrap()
    }
}

impl<'a> TowerService<&'a AddrStream> for Service {
    type Response = Service;
    type Error = Infallible;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: &'a AddrStream) -> Self::Future {
        future::ready(Ok(self.clone()))
    }
}

impl TowerService<Request<Body>> for Service {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let this = self.clone();
        async move { Ok(this.serve(req).await) }.boxed()
    }
}

async fn read_body(mut body: Body) -> Result<Vec<u8>, hyper::error::Error> {
    let mut buf = Vec::new();
    while let Some(chunk) = body.next().await {
        let chunk = chunk?;
        buf.extend_from_slice(&chunk);
    }
    Ok(buf)
}
