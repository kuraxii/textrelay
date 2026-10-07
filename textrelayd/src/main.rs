mod ble;
mod transport;
mod web;

use std::net::SocketAddr;
use std::sync::Arc;

use tracing_subscriber::EnvFilter;

/// 运行模式
enum Mode {
    /// 仅 HTTP 服务
    Http,
    /// 仅 BLE 服务
    Ble,
    /// HTTP + BLE 同时运行
    Both,
}

impl Mode {
    fn from_env() -> Self {
        match std::env::var("TEXTRELAYD_MODE")
            .as_deref()
        {
            Ok("ble") => Self::Ble,
            Ok("both") => Self::Both,
            _ => Self::Http, // 默认 HTTP only
        }
    }

    fn has_http(&self) -> bool {
        matches!(self, Self::Http | Self::Both)
    }

    fn has_ble(&self) -> bool {
        matches!(self, Self::Ble | Self::Both)
    }
}

#[tokio::main]
async fn main() {
    // 初始化日志（环境变量 RUST_LOG 控制级别，默认 info）
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mode = Mode::from_env();

    // 创建 unix socket 传输层 — HTTP 和 BLE 共享同一个实例
    let transport = Arc::new(transport::unix::UnixTransport::new()) as Arc<dyn transport::Transport>;

    // 可选：启动 BLE GATT 服务
    if mode.has_ble() {
        let ble_transport = transport.clone();
        tokio::spawn(async move {
            let server = ble::BleServer::new(ble_transport);
            server.run().await;
        });
    }

    // 可选：启动 HTTP 服务
    if mode.has_http() {
        let app = web::router(transport);

        let port = std::env::var("TEXTRELAYD_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(5000);

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        tracing::info!("HTTP 服务 → http://{addr}");

        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    } else {
        // BLE-only 模式：保持主线程运行
        tracing::info!("BLE 模式，无 HTTP 服务");
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    }
}