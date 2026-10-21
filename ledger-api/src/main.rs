#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("failed to bind 127.0.0.1:3000");
    println!(
        "listening on {}",
        listener.local_addr().expect("listener has a local address")
    );
    axum::serve(listener, ledger_api::app())
        .await
        .expect("server error");
}
