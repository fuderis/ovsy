use super::Agents;
use crate::{Manifest, prelude::*};
use anylm::{Schema, Tool};
use std::{fs, process::Stdio, time::SystemTime};
use tokio::{fs as tfs, process::Command};

/// Time range to find recent file
const RECENT_FILE_TIME_RANGE: u64 = 5;

/// The agent structure
#[derive(Default, Clone, Debug)]
pub struct Agent {
    pub dir: PathBuf,
    pub manifest: Config<Manifest>,
    pub examples: Vec<String>,
    pub tools: Vec<Tool>,
    pub last_update: Option<SystemTime>,
    pub trace: Option<Trace>,
}

impl Agent {
    /// Reads an agent server & runs it
    pub(super) async fn run<P>(dir: P) -> Result<Option<()>>
    where
        P: AsRef<Path>,
    {
        let agent_dir = dir.as_ref();

        // read manifest:
        let manifest_path = agent_dir.join("Ovsy.toml");
        let manifest = match Config::<Manifest>::new(&manifest_path) {
            Ok(r) => r,
            Err(e) => {
                warn!("Read manifest '{}' error: {e}", manifest_path.display());
                return Ok(None);
            }
        };
        let agent_name = &manifest.agent.name;

        // check if agent already is running:
        if Agents::has(agent_name).await {
            trace!("Agent '{agent_name}' already running, skipping..");
            return Ok(None);
        }

        // remove tool if disabled:
        if !manifest.agent.enable {
            Agents::stop(agent_name).await?;
            return Ok(None);
        }

        // get actual mimetype:
        let exec_path = agent_dir.join(&manifest.agent.exec);
        let mut last_update: Option<SystemTime> = if manifest_path.exists() {
            Some(fs::metadata(&manifest_path)?.modified()?)
        } else {
            None
        };
        if exec_path.exists() {
            let exec_mtime = fs::metadata(&exec_path)?.modified()?;
            match last_update {
                Some(current) => last_update = Some(current.max(exec_mtime)),
                None => last_update = Some(exec_mtime),
            }
        }

        // collect examples & tools:
        let mut examples = vec![];
        let mut tools = vec![];

        for (name, action) in manifest.actions.iter() {
            // gen examples:
            for exmpl in action.examples.iter() {
                examples.push(fmt!(
                    r#"* query: "{query}", result: {name} {data}"#,
                    query = exmpl.query,
                    data = json::to_string(&exmpl.data).unwrap(),
                ))
            }

            // create tool & push:
            let mut schema = Schema::object("");
            for (name, arg) in action.arguments.iter() {
                let value = json::to_value(arg).unwrap();
                schema.set_property(
                    name.clone(),
                    json::from_value(value).unwrap(),
                    !arg.optional,
                );
            }
            tools.push(Tool::new(name.clone(), action.description.clone(), schema));
        }

        // exec file path:
        let orig_exec = exec_path.clone();
        let ovsy_exec_name = fmt!(
            "ovsy-{}",
            manifest
                .agent
                .exec
                .file_name()
                .map(|s: &std::ffi::OsStr| str!(s.to_string_lossy()))
                .unwrap_or(agent_name.clone())
        );
        let ovsy_exec_path = agent_dir.join(&ovsy_exec_name);

        // check metadata:
        let needs_copy = if ovsy_exec_path.exists() {
            let ovsy_mtime = fs::metadata(&ovsy_exec_path)?.modified()?;
            if let Some(orig_mtime) = fs::metadata(&orig_exec)?.modified()?.into() {
                orig_mtime > ovsy_mtime
            } else {
                true
            }
        } else {
            true
        };

        // copy exec file (if needs):
        if needs_copy {
            if ovsy_exec_path.exists() {
                let _ = fs::remove_file(&ovsy_exec_path);
            }
            fs::copy(&orig_exec, &ovsy_exec_path)?;
        }

        // run tool server (if exists):
        if let Some(server) = &manifest.server {
            use tokio::net::TcpStream;

            // check port for available:
            let addr = fmt!("127.0.0.1:{}", server.port);
            if TcpStream::connect(&addr).await.is_err() {
                // create agent run command:
                let mut cmd = Command::new(ovsy_exec_path);
                cmd.stdout(Stdio::null());
                cmd.stderr(Stdio::null());
                cmd.kill_on_drop(false);

                // spawn process child:
                if let Err(e) = cmd.spawn() {
                    error!("Failed start '{}' agent server: {e}", manifest.agent.name);
                    return Err(Error::FailedRunTool(manifest.agent.name.clone(), e).into());
                };
            }
        }

        // wait for run server:
        sleep(Duration::from_millis(500)).await;

        // trace a new created log file:
        let mut retryes = RECENT_FILE_TIME_RANGE * 2;
        let mut trace = None;
        while retryes > 0 {
            let logs_dir = app_data()
                .join("agents")
                .join(&manifest.agent.name)
                .join("logs");

            if let Some(log_file) =
                Self::find_recent_file(&logs_dir, RECENT_FILE_TIME_RANGE).await?
            {
                let timeout = Settings::get().agents.trace_timeout;
                trace.replace(Trace::open(log_file, Duration::from_millis(timeout), false).await?);
                break;
            }

            retryes -= 1;
            sleep(Duration::from_millis(500)).await;
        }

        // check trace status:
        if trace.is_none() {
            warn!("Failed to catch log file for tracing");
        }

        // register tool instance:
        Agents::add(Agent {
            dir: agent_dir.to_path_buf(),
            manifest,
            examples,
            tools,
            last_update,
            trace,
        })
        .await;

        Ok(Some(()))
    }

    /// Finds and returns the most recent file in dir
    async fn find_recent_file<P>(dir: &P, time_range: u64) -> Result<Option<PathBuf>>
    where
        P: AsRef<Path>,
    {
        let dir = dir.as_ref();
        // time range:
        let now = SystemTime::now();
        let time_start = now.checked_sub(Duration::from_secs(time_range)).unwrap();
        let time_end = now.checked_add(Duration::from_secs(time_range)).unwrap();

        let mut newest_file: Option<(PathBuf, SystemTime)> = None;
        let mut reader = tfs::read_dir(dir).await?;

        // read dir files:
        while let Some(entry) = reader.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                let metadata = tfs::metadata(&path).await?;
                if let Ok(created) = metadata.created()
                    && created >= time_start
                    && created <= time_end
                {
                    // compare with last recent file:
                    if let Some((_, ref newest_time)) = newest_file {
                        if created > *newest_time {
                            newest_file = Some((path, created));
                        }
                    } else {
                        newest_file = Some((path, created));
                    }
                }
            }
        }

        Ok(newest_file.map(|(path, _)| path))
    }

    /// Checks & reruns agent if needs
    pub(super) async fn check(&self) -> Result<()> {
        let agent_dir = &self.dir;
        let manifest_path = agent_dir.join("Ovsy.toml");
        let name = &self.manifest.agent.name;

        // check manifest for exists:
        if !manifest_path.exists() {
            warn!(
                "Manifest '{}' not found, stopping agent '{name}'..",
                manifest_path.display()
            );
            Agents::stop(&self.manifest.agent.name).await?;
            return Ok(());
        }

        // check if still enabled:
        match Config::<Manifest>::new(&manifest_path) {
            Ok(new_manifest) => {
                if !new_manifest.agent.enable {
                    warn!("Agent '{name}' disabled in manifest, stopping..");
                    Agents::stop(name).await?;
                    return Ok(());
                }
            }
            Err(e) => {
                warn!(
                    "Fail with read manifest '{}': {e}..",
                    manifest_path.display()
                );
                Agents::stop(name).await?;
                return Ok(());
            }
        }

        // get actual mimetype:
        let exec_path = agent_dir.join(&self.manifest.agent.exec);
        let mut new_update: Option<SystemTime> = if manifest_path.exists() {
            Some(fs::metadata(&manifest_path)?.modified()?)
        } else {
            None
        };
        if exec_path.exists() {
            let exec_mtime = fs::metadata(&exec_path)?.modified()?;
            match new_update {
                Some(current) => new_update = Some(current.max(exec_mtime)),
                None => new_update = Some(exec_mtime),
            }
        }

        // rerun tool server (if outdated):
        match (self.last_update, new_update) {
            (Some(old), Some(new)) if new > old => {
                info!("Agent '{name}' outdated, restarting..");

                // stop server:
                Agents::stop(name).await?;

                // re-run server:
                if let Some(()) = Self::run(agent_dir).await? {
                    info!("Agent '{name}' successfully restarted");
                }
            }
            _ => {
                // up to date
                trace!("Agent '{name}' is up to date");
            }
        }

        Ok(())
    }

    /// Stops the agent server
    pub(super) async fn stop(&self) -> Result<()> {
        let port = if let Some(server) = &self.manifest.server {
            server.port
        } else {
            return Ok(());
        };
        info!(
            "Trying to kill '{}' agent server on {port} port..",
            &self.manifest.agent.name
        );

        // kill tool server by port:
        #[cfg(unix)]
        {
            // find server process by port:
            let output = Command::new("lsof")
                .args(["-i", &fmt!("TCP:{port}"), "-i", &fmt!("UDP:{port}")])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await?;

            // parse output:
            let s = String::from_utf8_lossy(&output.stdout);
            let mut lines = s.lines();
            if lines.next().is_none() {
                info!("Port {port} already is free");
                return Ok(());
            }

            // parse PIDs:
            let pids: Vec<i32> = lines
                .map(|l| {
                    let mut spl = l.split_whitespace();
                    (spl.next(), spl.next())
                })
                .filter_map(
                    |(name, pid)| {
                        if name.unwrap_or("").starts_with("ovsy-") {
                            pid.unwrap_or("").trim().parse().ok().filter(|&pid| pid > 1)
                        } else {
                            None
                        }
                    }, // exclude system ones
                )
                .collect();

            if !pids.is_empty() {
                info!("Found {} processes on port {port}: {pids:?}", pids.len());
            } else {
                info!("No processes found on port {port}");
                return Ok(());
            }

            // stop all PIDs:
            for pid in pids {
                let pid_str = pid.to_string();
                if Command::new("kill")
                    .args(["-TERM", &pid_str])
                    .status()
                    .await?
                    .success()
                {
                    info!("Graceful stop PID {pid}");
                } else {
                    let _ = Command::new("kill").args(["-9", &pid_str]).status().await;
                    info!("Force kill PID {pid}");
                }
            }
        }

        /* TODO: kill server process on Windows OS..
        #[cfg(windows)]
        {
            // netstat -ano | findstr :PORT
            let output = Command::new("netstat")
                .args(["-ano", &format!("|findstr :{}", port)])
                .stdout(Stdio::piped())
                .output()?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut pids = Vec::new();

            for line in stdout.lines() {
                if let Some(pid_str) = line.split_whitespace().last() {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        pids.push(pid);
                    }
                }
            }

            for pid in pids {
                // taskkill /PID <pid> /F
                let status = Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/F"])
                    .status()?;

                if status.success() {
                    info!("Killed PID {} on port {}", pid, port);
                }
            }
        }
        */

        // wait for stop server:
        sleep(Duration::from_millis(1000)).await;

        Ok(())
    }
}
