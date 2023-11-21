use actix_web::{web, App, HttpServer, Responder};

async fn echo(req_body: String) -> impl Responder {
    format!("Echo: {}", req_body)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/echo", web::post().to(echo))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}


// This is a conceptual representation and needs an async runtime like Tokio
async fn client_send_message(message: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let res = client.post("http://127.0.0.1:8080/echo")
        .body(message.to_string())
        .send()
        .await?;

    println!("Response: {}", res.text().await?);
    Ok(())
}
