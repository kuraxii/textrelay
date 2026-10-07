# TextRelay（手机远程文本输入 → Linux）

通过手机浏览器输入文本（语音或打字），将文字直接上屏至 Linux 当前焦点输入框。

## 架构

```
手机浏览器 ──HTTP──→ textrelayd (Rust) ──unix socket──→ fcitx5-textrelay addon
                                                           │
                                              InputContext::commitString()
                                                           │
                                                   当前焦点输入框
```

整个链路：

1. **textrelayd**（本 Rust 服务）提供手机端可访问的 HTTP 页面
2. 用户在手机输入文本，`POST /commit` 发给 textrelayd
3. textrelayd 通过 unix socket 将文本发给 **fcitx5-textrelay addon**
4. addon 调用 `InputContext::commitString()` — 与输入法选词上屏完全同一条路径
5. 不使用剪贴板，不模拟键盘事件，与 libpinyin 等引擎无关不干扰

协议：连接 `$XDG_RUNTIME_DIR/textrelay.sock`，发送 UTF-8 文本，`shutdown` 写端，
服务端应答 1 字节：`0x01` = 已上屏，`0x00` = 无焦点输入框。

> 为什么不能直接调 fcitx5 的 DBus？`org.fcitx.Fcitx.Controller1` 只有激活/切换
> 输入法等管理方法，没有对外提交文本的接口；`CommitString` 只存在于 fcitx 与
> 应用之间。必须以 addon 形式运行在 fcitx5 进程内部。

手动调试 unix socket：

```bash
python3 -c "import socket,os;s=socket.socket(socket.AF_UNIX);\
s.connect(os.path.join(os.environ.get('XDG_RUNTIME_DIR',\
'/run/user/'+str(os.getuid())),'textrelay.sock'));\
s.sendall('你好'.encode());s.shutdown(socket.SHUT_WR);print(s.recv(1))"
```

## 运行环境

- Rust 工具链（`rustc` + `cargo`）
- Linux 输入法框架：Fcitx5（与具体输入法引擎无关）
- 本仓库提供的 `fcitx5-textrelay` addon（见下）

## 1. 构建并安装 fcitx5-textrelay addon

```bash
cd fcitx5-textrelay
./build-rpm.sh          # 需要 gcc-c++，产物在 ~/rpmbuild/RPMS/x86_64/
sudo dnf install ~/rpmbuild/RPMS/x86_64/fcitx5-textrelay-0.3.0-1.*.rpm
```

安装后重启 fcitx5 使 addon 生效（任意一种）：

```bash
busctl --user call org.fcitx.Fcitx5 /controller org.fcitx.Fcitx.Controller1 Restart
# 或注销重登
```

验证 addon 已加载（应出现 socket 文件）：

```bash
ls -la $XDG_RUNTIME_DIR/textrelay.sock
```

卸载：`sudo dnf remove fcitx5-textrelay`。

## 2. 构建并启动 textrelayd

```bash
cd textrelayd
cargo run
```

服务默认监听 `http://0.0.0.0:5000`。可通过环境变量覆盖端口：

```bash
TEXTRELAYD_PORT=8080 cargo run
```

## 3. 手机端使用

确保手机与 Linux 处于同一局域网，手机浏览器访问：

```
http://<Linux局域网IP>:5000
```

在 Linux 桌面上**保持一个文本输入框处于焦点状态**，然后在手机输入框里输入
文本（语音或打字），点击 **发送上屏并清空**，文字会直接出现在电脑当前焦点的输入框
中。页面会显示状态：已上屏 / 电脑上没有聚焦的输入框 / 插件未安装。

## 目录说明

- `textrelayd/` — Rust 实现的 HTTP 中继服务
  - `src/main.rs` — 入口：tracing 初始化，构造 Transport，启动 axum 服务
  - `src/transport/` — 传输层抽象（`Transport` trait）
    - `mod.rs` — 定义 `Transport` trait 和 `TransportError`
    - `unix.rs` — `UnixTransport`：通过 unix socket 对接 fcitx5 addon
  - `src/web/` — HTTP 层
    - `mod.rs`, `handler.rs` — axum 路由与 handler，后端无关
    - `html.rs` — 内联手机端 HTML 页面
- `fcitx5-textrelay/` — fcitx5 addon 源码与 RPM 打包
  - `textrelay.cpp` — addon 实现（unix socket 服务 + commitString）
  - `textrelay.conf` — fcitx5 addon 描述文件
  - `compat/` — 手工合成的 export 头文件（替代 CMake 生成物）
  - `fcitx5-textrelay.spec` — RPM spec
  - `build-rpm.sh` — 一键构建 RPM

## Todo

1. ~~能否不使用 dbus~~ — 0.2.0 起数据面走 unix socket，0.3.0 起 addon 完全移除 DBus
2. ~~server 使用 Rust 实现~~
3. 蓝牙支持（`Transport` trait 已预留接口，追加 `BleTransport` 即可）