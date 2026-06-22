use super::Manager;
use crate::prelude::*;

use ovsy_share::AgentInfo;
use pearce::Client;
use std::{
    process::Stdio,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::{
    net::UnixStream,
    process::{Child, Command},
    sync::Mutex,
    time,
};

/// The AI agent
#[derive(Default, Debug, Clone)]
pub struct Agent {
    pub dir: PathBuf,
    pub exec_path: PathBuf,
    pub sock_path: PathBuf,
    pub info: AgentInfo,
    _started: Option<SystemTime>,
    _child: Arc<Mutex<Option<Child>>>,
}

impl Agent {
    /// Runs the agent server
    pub async fn run(dir: impl Into<PathBuf>) -> Result<Option<Self>> {
        let dir = dir.into();
        let name = dir
            .file_name()
            .ok_or(Error::FailedGetAgentName)?
            .to_string_lossy()
            .to_string();

        // check agent for already running:
        if Manager::contains(&arc!(name.clone())).await {
            return Ok(None);
        }

        // run agent server:
        let exec_path = dir.join(&str!(
            "{name}-agent{exe}",
            exe = if cfg!(windows) { ".exe" } else { "" }
        ));

        // Формируем путь к UDS сокету: app_data().join("uds/{agent_name}.sock")
        // Предполагается, что функция app_data() возвращает PathBuf и доступна в контексте
        let sock_path = app_data().join(format!("uds/{}.sock", name));

        // Убедимся, что директория для сокетов существует (актуально для Linux/macOS/Windows)
        if let Some(parent) = sock_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        // Удаляем старый файл сокета, если он остался после прошлого падения
        if sock_path.exists() {
            let _ = tokio::fs::remove_file(&sock_path).await;
        }

        // build command:
        let mut cmd = Command::new(&exec_path);
        // Передаем путь к сокету в аргументы агента (убедитесь, что бинарник принимает --socket)
        cmd.args(&["--socket", &sock_path.to_string_lossy()])
            .args(&["--max-logs", &str!(Settings::get().server.max_logs)])
            .stdin(Stdio::piped())
            .kill_on_drop(true);

        #[cfg(target_os = "linux")]
        {
            unsafe {
                cmd.pre_exec(|| {
                    if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }

        let child = {
            #[cfg(windows)]
            {
                cmd.spawn_group()?
            }

            #[cfg(not(windows))]
            {
                cmd.spawn()?
            }
        };

        // Инициализируем ваш IPC клиент из pearce
        let client = Client::ipc(&sock_path.to_string_lossy());
        let mut attempts = 0;

        let info = loop {
            attempts += 1;

            // Используем .post("/info") — ваш клиент сам подставит http://localhost/info
            // и направит запрос напрямую в Unix Domain Socket
            let request_result =
                time::timeout(Duration::from_millis(100), client.post("/info").send()).await;

            match request_result {
                Ok(Ok(response)) => {
                    break response.json::<AgentInfo>().await;
                }
                _ => {
                    if attempts >= 50 {
                        // Не забудьте обновить определение вашей ошибки, чтобы она принимала sock_path
                        return Err(Error::AgentStartFailed {
                            name,
                            sock_path: sock_path.to_string_lossy().to_string(),
                        }
                        .into());
                    }

                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            }
        }?;

        let agent = Self {
            dir,
            exec_path,
            sock_path,
            info,
            _started: Some(SystemTime::now()),
            _child: arc_mutex!(Some(child)),
        };

        Ok(Some(agent))
    }

    /// Returns true if needs to be updated
    pub async fn check(&self) -> Result<bool> {
        // Проверка живости агента через UnixStream
        let is_alive = time::timeout(
            Duration::from_millis(100),
            UnixStream::connect(&self.sock_path),
        )
        .await;

        if is_alive.is_err() || is_alive.unwrap().is_err() {
            // agent not responding..
            return Ok(true);
        }

        // check file metadata:
        let metadata = tokio::fs::metadata(&self.exec_path).await?;

        if let Ok(modified_at) = metadata.modified()
            && let Some(started_at) = self._started
        {
            Ok(modified_at > started_at)
        } else {
            Ok(false)
        }
    }
}
