use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use serde::Serialize;
use std::convert::Infallible;
use tokio::runtime::Runtime;

#[derive(Serialize)]
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
    if req.method() == Method::POST && req.uri().path() == "/completion" {
        handle_completion(req).await
    } else {
        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .unwrap())
    }
}

async fn handle_completion(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let whole_body = hyper::body::to_bytes(req.into_body()).await;
    let body_bytes = match whole_body {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("Error reading body: {}", err);
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

    let rollup_url = match std::env::var("ROLLUP_HTTP_SERVER_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("ROLLUP_HTTP_SERVER_URL not set");
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("ROLLUP_HTTP_SERVER_URL not set"))
                .unwrap());
        }
    };

    let gio_url = format!("{}/gio", rollup_url);

    match reqwest::Client::new()
        .post(&gio_url)
        .json(&gio_request)
        .send()
        .await
    {
        Ok(res) if res.status().is_success() => {
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from("Forwarded successfully"))
                .unwrap())
        }
        Ok(res) => {
            let status = res.status();
            let msg = format!("Failed to forward request. Upstream status: {}", status);
            eprintln!("{}", msg);
            Ok(Response::builder()
                .status(status)
                .body(Body::from(msg))
                .unwrap())
        }
        Err(err) => {
            eprintln!("Error sending request to rollup server: {}", err);
            Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::from(format!("Error forwarding: {}", err)))
                .unwrap())
        }
    }
}
