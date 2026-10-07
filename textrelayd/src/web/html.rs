/// 手机端 HTML 页面
///
/// 页面自带实时监听，手机输入完成后点击发送，也可以绑定回车自动发送。
pub const HTML: &str = r##"<!DOCTYPE html>
<html>
<head>
<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
<style>
  body { font-family: system-ui, sans-serif; padding: 20px; background: #f2f2f7; margin: 0; }
  textarea { width: 100%; height: 180px; font-size: 20px; border-radius: 12px; border: 1px solid #ccc; padding: 12px; box-sizing: border-box; }
  button { width: 100%; height: 60px; font-size: 20px; font-weight: bold; background: #007aff; color: white; border: none; border-radius: 12px; margin-top: 16px; cursor: pointer; }
  button:active { background: #0056b3; }
  #status { min-height: 24px; margin-top: 12px; text-align: center; font-size: 15px; color: #666; }
</style>
</head>
<body>
  <h3>手机远程文本输入 -> Linux</h3>
  <textarea id="text" placeholder="点击呼出输入法，使用语音输入..."></textarea>
  <button onclick="commit()">发送上屏并清空</button>
  <div id="status"></div>

  <script>
    async function commit() {
      const box = document.getElementById('text');
      const status = document.getElementById('status');
      const val = box.value.trim();
      if (!val) return;
      try {
        const res = await fetch('/commit', {
          method: 'POST',
          headers: {'Content-Type': 'text/plain; charset=utf-8'},
          body: val
        });
        const msg = await res.text();
        if (msg === 'OK') {
          box.value = '';
          box.focus();
          status.textContent = '已上屏 ✓';
        } else if (msg === 'NO_FOCUS') {
          status.textContent = '电脑上没有聚焦的输入框';
        } else {
          status.textContent = msg;
        }
      } catch (e) {
        status.textContent = '请求失败: ' + e;
      }
      setTimeout(() => status.textContent = '', 3000);
    }
  </script>
</body>
</html>"##;