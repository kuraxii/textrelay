use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::timeout;

use super::{Transport, TransportError};

/// 获取 fcitx5-textrelay addon 监听的 unix socket 路径
fn socket_path() -> PathBuf {
    let runtime = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", unsafe { libc::getuid() }));
    PathBuf::from(runtime).join("textrelay.sock")
}

/// 通过 fcitx5-textrelay addon 的 unix socket 提交文本
///
/// 协议：发送 UTF-8 文本后 shutdown 写端，服务端应答 1 字节：
/// `0x01` = 已提交到焦点输入上下文，`0x00` = 无焦点输入框。
pub struct UnixTransport {
    path: PathBuf,
    timeout: Duration,
}

impl Default for UnixTransport {
    fn default() -> Self {
        Self {
            path: socket_path(),
            timeout: Duration::from_secs(2),
        }
    }
}

impl UnixTransport {
    /// 创建一个新的 UnixTransport，使用默认的 socket 路径和 2 秒超时
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置 socket 路径
    #[allow(dead_code)]
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            path,
            timeout: Duration::from_secs(2),
        }
    }
}

#[async_trait]
impl Transport for UnixTransport {
    async fn commit(&self, text: &str) -> Result<bool, TransportError> {
        let path = self.path.clone();
        let timeout_dur = self.timeout;

        // 整个操作受超时保护
        timeout(timeout_dur, async move {
            // 1. 连接 unix socket
            let mut stream = UnixStream::connect(&path)
                .await
                .map_err(|e| match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        TransportError::NotConnected(
                            "fcitx5-textrelay 插件未安装或 fcitx5 未重启".into(),
                        )
                    }
                    std::io::ErrorKind::ConnectionRefused => {
                        TransportError::NotConnected(
                            "无法连接 fcitx5-textrelay，请重启 fcitx5".into(),
                        )
                    }
                    _ => TransportError::Protocol(e.to_string()),
                })?;

            // 2. 发送文本
            stream
                .write_all(text.as_bytes())
                .await
                .map_err(|e| TransportError::Protocol(e.to_string()))?;

            // 3. shutdown 写端，通知服务端数据发送完毕
            stream
                .shutdown()
                .await
                .map_err(|e| TransportError::Protocol(e.to_string()))?;

            // 4. 读取 1 字节应答
            let mut buf = [0u8; 1];
            stream
                .read_exact(&mut buf)
                .await
                .map_err(|e| TransportError::Protocol(e.to_string()))?;

            match buf[0] {
                0x01 => Ok(true),  // 已上屏
                0x00 => Ok(false), // 无焦点
                b => Err(TransportError::Protocol(format!(
                    "未知应答字节: 0x{:02x}",
                    b
                ))),
            }
        })
        .await
        .map_err(|_| TransportError::Timeout)?
    }
}