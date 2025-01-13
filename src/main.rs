use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::convert::Infallible;
use hex;

#[derive(Serialize, Deserialize, Debug)]
struct GIORequest {
    domain: u16,
    id: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = ([0, 0, 0, 0], 8080).into();

    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(router))
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!("Server running at http://{}...", addr);

    server.await?;
    Ok(())
}

async fn router(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/completion") => handle_completion(req).await,
        (&Method::POST, "/gio") => handle_gio(req).await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .unwrap()),
    }
}

async fn handle_completion(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let whole_body = hyper::body::to_bytes(req.into_body()).await;
    let body_bytes = match whole_body {
        Ok(bytes) => bytes,
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("Could not read request body"))
                .unwrap());
        }
    };

    let raw_json_str = String::from_utf8_lossy(&body_bytes);
    let hex_encoded = hex::encode(raw_json_str.as_bytes());

    let gio_request = GIORequest {
        domain: 0x27,
        id: hex_encoded,
    };

    let gio_request_json = serde_json::to_string(&gio_request).unwrap();

    let mock_request = Request::builder()
        .method(Method::POST)
        .uri("/gio")
        .header("Content-Type", "application/json")
        .body(Body::from(gio_request_json))
        .unwrap();

    handle_gio(mock_request).await
}

async fn handle_gio(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let whole_body = hyper::body::to_bytes(req.into_body()).await;
    match whole_body {
        Ok(body) => {
            let gio_request: Result<GIORequest, _> = serde_json::from_slice(&body);
            match gio_request {
                Ok(gio_req) => {
                    println!("Received GIORequest: {:?}", gio_req);
                    let resp = json!({ "status": "success" });
                    Ok(Response::new(Body::from(resp.to_string())))
                }
                Err(_) => {
                    let resp = json!({ "error": "Invalid JSON" });
                    Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from(resp.to_string()))
                        .unwrap())
                }
            }
        }
        Err(_) => {
            let resp = json!({ "error": "Could not read request body" });
            Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(resp.to_string()))
                .unwrap())
        }
    }
}
