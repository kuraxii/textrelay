use std::fmt;

/// 传输层错误
#[derive(Debug)]
pub enum TransportError {
    /// 无法连接后端（插件未安装 / 无法连接 socket）
    NotConnected(String),
    /// 后端响应超时
    Timeout,
    /// 通信协议错误（IO 失败或异常应答）
    Protocol(String),
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportError::NotConnected(msg) => write!(f, "{msg}"),
            TransportError::Timeout => write!(f, "fcitx5 响应超时"),
            TransportError::Protocol(msg) => write!(f, "通信失败: {msg}"),
        }
    }
}

impl std::error::Error for TransportError {}

/// 文本提交传输层抽象
///
/// 当前实现：[UnixTransport](super::unix::UnixTransport) 走 fcitx5 addon 的 unix socket。
/// 追加蓝牙支持时只需新实现此 trait，HTTP 层无需改动。
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    /// 提交文本到当前焦点输入框。
    ///
    /// 返回 `Ok(true)` = 已上屏, `Ok(false)` = 当前无焦点输入框。
    async fn commit(&self, text: &str) -> Result<bool, TransportError>;
}

pub mod unix;