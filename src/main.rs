use axum::{
    body::Body,
    extract::{Json, Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use clap::{Parser, Subcommand};
use reqwest::Client;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path as StdPath, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

// --- 静的ファイルの埋め込み設定 ---
// 実行ファイル生成時、プロジェクト直下の "static" フォルダの内容をバイナリへ埋め込む
#[derive(RustEmbed)]
#[folder = "static/"]
struct Assets;

// --- 構造体定義 ---

#[derive(Parser)]
#[command(name = "lustui")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    prj_file: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    New { name: String },
}

#[derive(Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct LustuiProject {
    name: String,
    work_dir: String,
    history: Vec<ChatMessage>,
}

#[derive(Deserialize)]
struct ChatRequest {
    model: String,
    message: String,
}

struct AppState {
    project_path: PathBuf,
    project_data: Mutex<LustuiProject>,
}

// --- メイン処理 ---

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Some(Commands::New { name }) = cli.command {
        create_new_project(&name);
        return;
    }

    let prj_path = match cli.prj_file {
        Some(path) => path,
        None => {
            println!("使用法: lustui <project.lustuiprj> または lustui new <name>");
            return;
        }
    };

    if !prj_path.exists() {
        eprintln!("エラー: 指定されたファイルが存在しません: {:?}", prj_path);
        return;
    }

    let file_content = fs::read_to_string(&prj_path).expect("プロジェクト読み込み失敗");
    let project_data: LustuiProject = serde_json::from_str(&file_content).expect("JSONエラー");

    let shared_state = Arc::new(AppState {
        project_path: prj_path,
        project_data: Mutex::new(project_data),
    });

    // 埋め込みアセット配信ルートを登録
    let app = Router::new()
        .route("/api/models", get(get_models_handler))
        .route("/api/chat", post(chat_agent_handler))
        .route("/", get(static_handler))
        .route("/*file", get(static_handler))
        .with_state(shared_state);

    let addr = "127.0.0.1:3000";
    println!("Server running on http://{}", addr);

    let _ = open::that(format!("http://{}", addr));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// --- 埋め込みアセットハンドラー ---

async fn static_handler(path: Option<Path<String>>) -> impl IntoResponse {
    let path = path.map(|p| p.0).unwrap_or_else(|| "index.html".to_string());
    let target_path = if path.is_empty() { "index.html" } else { &path };

    match Assets::get(target_path) {
        Some(content) => {
            let mime = mime_guess::from_path(target_path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, HeaderValue::from_str(mime.as_ref()).unwrap())
                .body(Body::from(content.data))
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("404 Not Found"))
            .unwrap(),
    }
}

// --- Ollamaの自動起動ロジック ---

async fn ensure_ollama_model_running(client: &Client, model_name: &str) -> Result<(), String> {
    // 1. Ollamaサービスが疎通可能かチェック
    let is_running = client.get("http://localhost:11434/api/tags").send().await.is_ok();

    if !is_running {
        println!("[Ollama] Ollamaプロセスが検知されません。モデル '{}' をバックグラウンドで起動します...", model_name);

        // OSごとに `ollama run <model>` をバックグラウンド起動
        let status = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", &format!("start /B ollama run {}", model_name)])
                .spawn()
        } else {
            Command::new("sh")
                .args(["-c", &format!("ollama run {} > /dev/null 2>&1 &", model_name)])
                .spawn()
        };

        if let Err(e) = status {
            return Err(format!("Ollamaの起動に失敗しました: {}", e));
        }

        // 起動待ち (最大15秒待機)
        for _ in 0..15 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            if client.get("http://localhost:11434/api/tags").send().await.is_ok() {
                println!("[Ollama] Ollamaの起動を確認しました");
                return Ok(());
            }
        }
        return Err("Ollamaの起動タイムアウトエラー".to_string());
    }

    Ok(())
}

// --- API ハンドラー ---

async fn get_models_handler() -> impl IntoResponse {
    let client = Client::new();

    // 疎通確認（落ちていれば基本モデルの起動を試行）
    let _ = ensure_ollama_model_running(&client, "codellama").await;

    let res = client.get("http://localhost:11434/api/tags").send().await;

    if let Ok(response) = res {
        if let Ok(json) = response.json::<serde_json::Value>().await {
            let models: Vec<String> = json
                .get("models")
                .and_then(|m| m.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|i| i.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            return Json(serde_json::json!({ "models": models }));
        }
    }
    Json(serde_json::json!({ "models": [] }))
}

async fn chat_agent_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ChatRequest>,
) -> impl IntoResponse {
    let client = Client::new();

    // ★ チャットリクエスト受信時、指定されたモデルの起動を確認・実行
    if let Err(err_msg) = ensure_ollama_model_running(&client, &payload.model).await {
        return Json(serde_json::json!({ "error": err_msg }));
    }

    let (messages, work_dir) = {
        let mut prj = state.project_data.lock().unwrap();
        prj.history.push(ChatMessage {
            role: "user".to_string(),
            content: payload.message,
        });
        (prj.history.clone(), prj.work_dir.clone())
    };

    let ollama_payload = serde_json::json!({
        "model": payload.model,
        "messages": messages,
        "stream": false
    });

    let res = client
        .post("http://localhost:11434/api/chat")
        .json(&ollama_payload)
        .send()
        .await;

    if let Ok(response) = res {
        if let Ok(json) = response.json::<serde_json::Value>().await {
            if let Some(reply) = json.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                let mut final_reply = reply.to_string();

                if reply.contains("```json") {
                    if let Some(start) = reply.find("```json") {
                        if let Some(end) = reply[start..].find("```\n").or_else(|| reply[start..].find("\n```")) {
                            let json_str = reply[start + 7..start + end].trim();
                            if let Ok(tool_val) = serde_json::from_str::<serde_json::Value>(json_str) {
                                let tool_result = execute_agent_tool(&work_dir, &tool_val);
                                final_reply.push_str(&format!("\n\n**[ツール実行結果]**\n```\n{}\n```", tool_result));
                            }
                        }
                    }
                }

                let mut prj = state.project_data.lock().unwrap();
                prj.history.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: final_reply.clone(),
                });

                let updated_json = serde_json::to_string_pretty(&*prj).unwrap();
                let _ = fs::write(&state.project_path, updated_json);

                return Json(serde_json::json!({ "reply": final_reply }));
            }
        }
    }

    Json(serde_json::json!({ "error": "LLM通信エラーが発生しました" }))
}

// ツール実行関数の追加省略（前述と同じ）
fn execute_agent_tool(work_dir: &str, tool_call: &serde_json::Value) -> String {
    let command_type = tool_call.get("tool").and_then(|t| t.as_str()).unwrap_or("");
    match command_type {
        "run_cmd" => {
            let cmd = tool_call.get("cmd").and_then(|c| c.as_str()).unwrap_or("");
            let output = if cfg!(target_os = "windows") {
                Command::new("cmd").args(["/C", cmd]).current_dir(work_dir).output()
            } else {
                Command::new("sh").args(["-c", cmd]).current_dir(work_dir).output()
            };
            match output {
                Ok(out) => format!("STDOUT:\n{}\nSTDERR:\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)),
                Err(e) => format!("実行エラー: {}", e),
            }
        }
        "create_file" => {
            let path_str = tool_call.get("path").and_then(|p| p.as_str()).unwrap_or("");
            let content = tool_call.get("content").and_then(|c| c.as_str()).unwrap_or("");
            let target_path = StdPath::new(work_dir).join(path_str);
            if let Some(parent) = target_path.parent() { let _ = fs::create_dir_all(parent); }
            match fs::write(&target_path, content) {
                Ok(_) => format!("作成成功: {:?}", target_path),
                Err(e) => format!("エラー: {}", e),
            }
        }
        _ => "未知のツール呼び出しです".to_string(),
    }
}

fn create_new_project(name: &str) {
    let dir_name = format!("{}.lustuiprj_dir", name);
    let prj_file_name = format!("{}.lustuiprj", name);

    fs::create_dir_all(&dir_name).expect("作業ディレクトリの作成に失敗しました");

    let abs_work_dir = fs::canonicalize(&dir_name)
        .expect("絶対パスの取得に失敗しました")
        .to_str()
        .expect("パスの文字列変換に失敗しました")
        .to_string();

    let system_prompt = r#"あなたは指示に従って自律的に操作を行う開発自動化エージェントです。
ユーザーから「〜を作成して」「〜をビルドして」「実行して」などの指示を受けたら、文章で手順を説明するのではなく、必ず以下のJSONフォーマットを返答に含めて実際に操作を行ってください。

【ツール1: ファイル作成・編集】
```json
{
  "tool": "create_file",
  "path": "相対ファイルパス (例: main.c)",
  "content": "ファイル内に書き込むコード"
}
【ツール2: コマンド実行（ビルド・ファイル操作・品質確認など）】

JSON
{
  "tool": "run_cmd",
  "cmd": "実行するシェルコマンド (例: gcc -o main main.c && main.exe)"
}
注意点:
・説明文や補足は最小限にし、可能な限り迅速にツール（JSON）を発行してください。
・ツール実行結果はシステムから自動フィードバックされます。エラーが発生した場合はエラーログを確認して自律的に修正を行ってください。"#;

let initial_prj = LustuiProject {
    name: name.to_string(),
    work_dir: abs_work_dir,
    history: vec![ChatMessage {
        role: "system".to_string(),
        content: system_prompt.to_string(),
    }],
};

let json = serde_json::to_string_pretty(&initial_prj).expect("JSONシリアライズに失敗しました");
let mut file = File::create(&prj_file_name).expect("プロジェクトファイルの作成に失敗しました");
file.write_all(json.as_bytes()).expect("プロジェクトファイルへの書き込みに失敗しました");

println!("新規プロジェクトを作成しました: {}", prj_file_name);
}