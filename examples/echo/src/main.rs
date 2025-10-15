use wsforge::{handler::handler, prelude::*};

async fn echo_handler(msg: Message, conn: Connection) -> Result<Message> {
    println!("Received from {}: {:?}", conn.id(), msg.as_text());
    Ok(msg)
}

async fn json_echo_handler(
    Json(data): Json<serde_json::Value>,
    conn: Connection,
) -> Result<String> {
    println!("JSON from {}: {:?}", conn.id(), data);
    Ok(format!("Echo: {}", data))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let router = Router::new()
        .route("/echo", handler(echo_handler))
        .route("/json", handler(json_echo_handler))
        .on_connect(|_manager, conn_id| {
            println!("✅ Client connected: {}", conn_id);
        })
        .on_disconnect(|_manager, conn_id| {
            println!("❌ Client disconnected: {}", conn_id);
        })
        .default_handler(handler(|msg: Message| async move {
            Ok(Message::text(format!(
                "Unknown route. You sent: {:?}",
                msg.as_text()
            )))
        }));

    println!("🚀 Echo server running on ws://127.0.0.1:8080");
    router.listen("127.0.0.1:8080").await?;

    Ok(())
}
