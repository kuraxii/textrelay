//! BLE GATT Server — 使 Linux 主机充当 BLE Peripheral
//!
//! iPhone 作为 Central 主动连接，向 Write 特征值写入 UTF-8 文本，
//! 然后通过注入的 [`Transport`] 提交到 fcitx5 addon。

use std::sync::Arc;

use bluer::{
    adv::Advertisement,
    gatt::local::{
        characteristic_control, Application, Characteristic, CharacteristicControlEvent,
        CharacteristicWrite, CharacteristicWriteMethod, Service,
    },
};
use futures::{future, pin_mut, StreamExt};
use tokio::io::AsyncReadExt;

use crate::transport::Transport;

/// 自定义 GATT Service UUID
const SERVICE_UUID: uuid::Uuid = uuid::Uuid::from_u128(0xe0e1f000_1234_1000_8000_00805f9b34fb);
/// 文本写入特征值 UUID（iPhone 写入 UTF-8 文本到此特征值）
const TEXT_CHAR_UUID: uuid::Uuid = uuid::Uuid::from_u128(0xe0e1f001_1234_1000_8000_00805f9b34fb);

/// BLE GATT 服务端
///
/// 启动后注册一个自定义 Service + Write 特征值，等待手机连接并写入文本。
/// 收到写入后通过内部持有的 [`Transport`] 提交到 fcitx5 上屏。
pub struct BleServer {
    transport: Arc<dyn Transport>,
}

impl BleServer {
    /// 创建一个 BLE 服务端
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        Self { transport }
    }

    /// 启动 BLE GATT 服务并持续监听写入
    ///
    /// 这是一个长时间运行的 async 函数，应在单独的 tokio task 中 spawn。
    pub async fn run(self) {
        if let Err(e) = self.run_inner().await {
            tracing::error!("BLE server exited with error: {e}");
        }
    }

    async fn run_inner(self) -> bluer::Result<()> {
        // 1. 创建 BlueZ session 并获取默认蓝牙适配器
        let session = bluer::Session::new().await?;
        let adapter = session.default_adapter().await?;

        // 确保适配器已开启
        adapter.set_powered(true).await?;

        let addr = adapter.address().await?;
        tracing::info!("BLE: 适配器 {} ({addr})", adapter.name());

        // 2. 启动 LE Advertisement（让手机能发现我们）
        let advertisement = Advertisement {
            service_uuids: vec![SERVICE_UUID].into_iter().collect(),
            discoverable: Some(true),
            local_name: Some("TextRelay".to_string()),
            ..Default::default()
        };
        let _adv_handle = adapter.advertise(advertisement).await?;
        tracing::info!("BLE: 已发布广告 \"TextRelay\"");

        // 3. 注册 GATT Application
        //    一个 Service，一个 Write 特征值
        let (char_control, char_handle) = characteristic_control();
        let app = Application {
            services: vec![Service {
                uuid: SERVICE_UUID,
                primary: true,
                characteristics: vec![Characteristic {
                    uuid: TEXT_CHAR_UUID,
                    // 同时支持 Write Request（带响应）和 Write Command（无响应）
                    write: Some(CharacteristicWrite {
                        write: true,
                        write_without_response: true,
                        method: CharacteristicWriteMethod::Io,
                        ..Default::default()
                    }),
                    control_handle: char_handle,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        let _app_handle = adapter.serve_gatt_application(app).await?;
        tracing::info!("BLE: GATT 服务已注册，等待手机连接...");

        // 4. 事件循环：监听写入事件并读取数据
        let transport = self.transport;
        let mut read_buf = Vec::new();
        let mut reader_opt: Option<bluer::gatt::CharacteristicReader> = None;
        pin_mut!(char_control);

        loop {
            tokio::select! {
                evt = char_control.next() => {
                    match evt {
                        Some(CharacteristicControlEvent::Write(req)) => {
                            let mtu = req.mtu();
                            let dev = req.device_address();
                            tracing::debug!("BLE: 写入请求来自 {dev} (MTU {mtu})");
                            read_buf = vec![0u8; mtu];
                            match req.accept() {
                                Ok(reader) => reader_opt = Some(reader),
                                Err(e) => tracing::error!("BLE: 接受写入失败: {e}"),
                            }
                        }
                        Some(CharacteristicControlEvent::Notify(_)) => {
                            // 本特征值不支持 notify，忽略
                        }
                        None => {
                            tracing::warn!("BLE: 特征控制流已结束");
                            break;
                        }
                    }
                }
                read_res = async {
                    match &mut reader_opt {
                        Some(reader) => reader.read(&mut read_buf).await,
                        None => future::pending().await,
                    }
                } => {
                    match read_res {
                        Ok(0) => {
                            tracing::debug!("BLE: 写入流结束");
                            reader_opt = None;
                        }
                        Ok(n) => {
                            let text = String::from_utf8_lossy(&read_buf[..n]);
                            let text = text.trim().to_string();
                            if text.is_empty() {
                                continue;
                            }
                            match transport.commit(&text).await {
                                Ok(true) => tracing::info!("BLE: 已上屏 ({len} 字节)", len = n),
                                Ok(false) => tracing::warn!("BLE: 无焦点输入框"),
                                Err(e) => tracing::error!("BLE: 提交失败: {e}"),
                            }
                        }
                        Err(e) => {
                            tracing::error!("BLE: 读取写入流失败: {e}");
                            reader_opt = None;
                        }
                    }
                }
            }
        }

        // 5. 清理（app_handle / adv_handle 析构时自动取消注册）
        tracing::info!("BLE: 服务结束");
        Ok(())
    }
}