const demo = document.querySelector('.demo');
if (demo) {
  for (const button of document.querySelectorAll('[data-set-mode]')) {
    button.addEventListener('click', () => {
      const chinese = button.dataset.setMode === 'zh';
      demo.dataset.mode = chinese ? 'zh' : 'en';
      for (const option of document.querySelectorAll('[data-set-mode]')) {
        option.setAttribute('aria-pressed', String(option === button));
      }
      document.querySelector('.cursor-tag').textContent = chinese ? '中' : 'EN';
      document.querySelector('.keyboard-state').textContent = chinese ? '中' : 'A';
      document.querySelector('#demo-status').textContent = chinese
        ? '珊瑚色指针 · 中文状态' : '白色指针 · 英文状态';
    });
  }
}

for (const button of document.querySelectorAll('[data-command]')) {
  button.addEventListener('click', async () => {
    const status = document.querySelector('#copy-status');
    try {
      await navigator.clipboard.writeText(button.dataset.command);
      status.textContent = '退出命令已复制，请在程序目录运行。';
    } catch {
      status.textContent = '浏览器未允许复制，请手动选择并复制上方命令。';
    }
  });
}
