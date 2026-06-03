use super::Manager;
use crate::prelude::*;

use ovsy_share::AgentInfo;
use reqwest::Client;
use std::{net::SocketAddr, process::Stdio, time::SystemTime};
use tokio::{
    net::TcpStream,
    process::{Child, Command},
    time,
};

/// The AI agent
#[derive(Default, Debug, Clone)]
pub struct Agent {
    pub dir: PathBuf,
    pub exec_path: PathBuf,
    pub port: u16,
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
        let port = crate::free_port().await?;

        // build command:
        let mut cmd = Command::new(&exec_path);
        cmd.args(&["--port", &str!(port)])
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

        // get agent info:
        let mut attempts = 0;
        let client = Client::new();
        let info_url = str!("http://127.0.0.1:{port}/info");

        let info = loop {
            attempts += 1;

            let request_result =
                time::timeout(Duration::from_millis(100), client.post(&info_url).send()).await;

            match request_result {
                Ok(Ok(response)) => {
                    break response.json::<AgentInfo>().await;
                }
                _ => {
                    if attempts >= 50 {
                        return Err(Error::AgentStartFailed { name, port }.into());
                    }

                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            }
        }?;

        let agent = Self {
            dir,
            exec_path,
            port,
            info,
            _started: Some(SystemTime::now()),
            _child: arc_mutex!(Some(child)),
        };

        Ok(Some(agent))
    }

    /// Returns true if needs to be updated
    pub async fn check(&self) -> Result<bool> {
        // check for alive:
        let addr: SocketAddr = ([127, 0, 0, 1], self.port).into();

        let is_alive = time::timeout(Duration::from_millis(100), TcpStream::connect(addr)).await;
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
