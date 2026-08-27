const chatBox = document.getElementById('chat-box');
const userInput = document.getElementById('user-input');
const sendBtn = document.getElementById('send-btn');
const modelSelect = document.getElementById('model-select');
const welcomeScreen = document.getElementById('welcome-screen');

// テキストエリアの高さを自動調整
userInput.addEventListener('input', () => {
  userInput.style.height = 'auto';
  userInput.style.height = `${Math.min(userInput.scrollHeight, 150)}px`;
});

// モデル一覧を取得
async function loadModels() {
  try {
    const res = await fetch('/api/models');
    const data = await res.json();
    
    modelSelect.innerHTML = '';
    if (data.models && data.models.length > 0) {
      data.models.forEach(model => {
        const option = document.createElement('option');
        option.value = model;
        option.textContent = model;
        modelSelect.appendChild(option);
      });
    } else {
      modelSelect.innerHTML = '<option value="">モデルが見つかりません</option>';
    }
  } catch (err) {
    modelSelect.innerHTML = '<option value="">接続エラー</option>';
  }
}

// メッセージ要素を追加
function appendMessage(role, text) {
  if (welcomeScreen) {
    welcomeScreen.style.display = 'none';
  }

  const row = document.createElement('div');
  row.classList.add('message-row', role);

  if (role === 'assistant') {
    const avatar = document.createElement('div');
    avatar.classList.add('avatar');
    avatar.innerHTML = '<span class="material-symbols-outlined">sparkles</span>';
    row.appendChild(avatar);
  }

  const content = document.createElement('div');
  content.classList.add('message-content');
  
  // 簡易コードブロック変換（```で囲まれた部分を<pre><code>に変換）
  if (role === 'assistant' && text.includes('```')) {
    content.innerHTML = formatMarkdownCode(text);
  } else {
    content.textContent = text;
  }

  row.appendChild(content);
  chatBox.appendChild(row);
  chatBox.scrollTop = chatBox.scrollHeight;

  return content;
}

// 簡易Markdown（コードブロック）フォーマッタ
function formatMarkdownCode(text) {
  const parts = text.split(/(```[\s\S]*?```)/g);
  return parts.map(part => {
    if (part.startsWith('```') && part.endsWith('```')) {
      const codeContent = part.slice(3, -3).replace(/^[a-zA-Z]+\n/, ''); // 言語名をスキップ
      return `<pre><code>${escapeHtml(codeContent.trim())}</code></pre>`;
    }
    return escapeHtml(part);
  }).join('');
}

function escapeHtml(str) {
  return str.replace(/[&<>"']/g, match => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'
  }[match]));
}

async function sendMessage() {
  const text = userInput.value.trim();
  const selectedModel = modelSelect.value;

  if (!text || !selectedModel) return;

  appendMessage('user', text);
  userInput.value = '';
  userInput.style.height = 'auto';

  // ローディング表示を追加
  const loadingContent = appendMessage('assistant', '思考中...');

  try {
    const response = await fetch('/api/chat', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ model: selectedModel, message: text })
    });

    const data = await response.json();
    
    if (data.reply) {
      if (data.reply.includes('```')) {
        loadingContent.innerHTML = formatMarkdownCode(data.reply);
      } else {
        loadingContent.textContent = data.reply;
      }
    } else {
      loadingContent.textContent = 'エラー: ' + (data.error || '回答を取得できませんでした。');
    }
  } catch (err) {
    loadingContent.textContent = 'エラー: サーバーとの通信に失敗しました。';
  }
}

sendBtn.addEventListener('click', sendMessage);
userInput.addEventListener('keydown', (e) => {
  // Enterキーで送信 (Shift+Enterは改行)
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    sendMessage();
  }
});

loadModels();