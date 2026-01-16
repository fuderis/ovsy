use crate::{ prelude::*, Manifest };
use super::Tools;
use std::{ fs, process::Stdio };
use tokio::process::Command;

/// The tool structure
#[derive(Default, Clone, Debug)]
pub struct Tool {
    pub dir: PathBuf,
    pub manifest: Config<Manifest>,
    pub docs: Vec<String>,
    pub last_update: Option<SystemTime>,
}

impl Tool {
    /// Reads a tool server & runs it
    pub(super) async fn run<P>(tool_dir: P) -> Result<Option<()>>
    where
        P: AsRef<Path>
    {
        let tool_dir = tool_dir.as_ref();
        
        // read manifest:
        let manifest_path = tool_dir.join("Ovsy.toml");
        let manifest = match Config::<Manifest>::new(&manifest_path) {
            Ok(r) => r,
            Err(e) => {
                warn!("Read manifest '{}' error: {e}", manifest_path.display());
                return Ok(None);
            }
        };
        let name = &manifest.tool.name;

        // check if tool already exists:
        if Tools::has(&name).await {
            trace!("Tool '{name}' already running, skipping..");
            return Ok(None);
        }

        // remove tool if disabled:
        if !manifest.tool.enable {
            Tools::stop(name).await?;
            return Ok(None);
        }

        // get actual mimetype: 
        let exec_path = tool_dir.join(&manifest.tool.exec);
        let mut last_update: Option<SystemTime> = if manifest_path.exists() {
            Some(fs::metadata(&manifest_path)?.modified()?.into())
        } else {
            None
        };
        if exec_path.exists() {
            let exec_mtime = fs::metadata(&exec_path)?.modified()?.into();
            match last_update {
                Some(current) => last_update = Some(current.max(exec_mtime)),
                None => last_update = Some(exec_mtime),
            }
        }

        // generate prompt-doc:
        let mut docs = vec![];
        for (action_name, action) in &manifest.actions {
            let doc = fmt!(r#"* "{name}/{action_name}":\n  * description: {}\n  * arguments: {}"#,
                &action.descr,
                {
                    let mut args = vec![];
                    for (name, arg) in &action.args {
                        args.push(fmt!(r#"    * {}: format {}{}{}."#,
                            name,
                            arg.format,
                            if let Some(vars) = &arg.variants { 
                                fmt!(", variants {:?}", vars) 
                            } else { 
                                String::new() 
                            },
                            if arg.optional { ", optional" } else { "" },
                        ));
                    }
                    args.join("\n")
                }
            );
            docs.push(doc);
        }

        // exec file path:
        let orig_exec = exec_path.clone();
        let ovsy_exec_name = fmt!("ovsy-{}",
            manifest.tool.exec.file_name()
                .map(|s: &std::ffi::OsStr| str!(s.to_string_lossy()))
                .unwrap_or(name.clone())
        );
        let ovsy_exec_path = tool_dir.join(&ovsy_exec_name);

        // check metadata:
        let needs_copy = if ovsy_exec_path.exists() {
            let ovsy_mtime = fs::metadata(&ovsy_exec_path)?.modified()?.into();
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
        if manifest.server.is_some() {
            // create run command:
            let mut cmd = Command::new(ovsy_exec_path);
                cmd.stdout(Stdio::null());
                cmd.stderr(Stdio::null());
                cmd.kill_on_drop(false);
        
            // spawn process child:
            let _ = cmd.spawn()?;
        }
        
        // register tool instance:
        Tools::add(
            Tool {
                dir: tool_dir.to_path_buf(),
                manifest,
                last_update,
                docs,
            }
        ).await;

        Ok(Some(()))
    }

    /// Checks & reruns tool if needs
    pub(super) async fn check(&self) -> Result<()> {
        let tool_dir = &self.dir;
        let manifest_path = tool_dir.join("Ovsy.toml");
        let name = &self.manifest.tool.name;
        
        // check manifest for exists:
        if !manifest_path.exists() {
            warn!("Manifest '{}' not found, stopping tool '{name}'..", manifest_path.display());
            Tools::stop(&self.manifest.tool.name).await?;
            return Ok(());
        }

        // check if still enabled:
        match Config::<Manifest>::new(&manifest_path) {
            Ok(new_manifest) => {
                if !new_manifest.tool.enable {
                    warn!("Tool '{name}' disabled in manifest, stopping..");
                    Tools::stop(name).await?;
                    return Ok(());
                }
            }
            Err(e) => {
                warn!("Fail with read manifest '{}': {e}..", manifest_path.display());
                Tools::stop(name).await?;
                return Ok(());
            }
        }

        // get actual mimetype: 
        let exec_path = tool_dir.join(&self.manifest.tool.exec);
        let mut new_update: Option<SystemTime> = if manifest_path.exists() {
            Some(fs::metadata(&manifest_path)?.modified()?.into())
        } else {
            None
        };
        if exec_path.exists() {
            let exec_mtime = fs::metadata(&exec_path)?.modified()?.into();
            match new_update {
                Some(current) => new_update = Some(current.max(exec_mtime)),
                None => new_update = Some(exec_mtime),
            }
        }

        // rerun tool server (if outdated):
        match (self.last_update, new_update) {
            (Some(old), Some(new)) if new > old => {
                info!("Tool '{name}' outdated, restarting..");
                Tools::stop(name).await?;

                if let Some(()) = Self::run(tool_dir).await? {
                    info!("Tool '{name}' successfully restarted");
                }
            }
            _ => {
                // up to date
                trace!("Tool '{name}' is up to date");
            }
        }

        Ok(())
    }

    /// Stops the tool server
    pub(super) async fn stop(self) -> Result<()> {
        // get server port:
        if self.manifest.server.is_none() { return Ok(()); }
        let server = self.manifest.server.as_ref().unwrap();
        let port = server.port;
        info!("Trying to kill server on {port} port..");

        // kill tool server by port:
        #[cfg(unix)]
        {
            // find server process by port:
            let output = Command::new("lsof")
                .args([
                    "-t", 
                    "-i", &fmt!("TCP:{port}"),
                    "-i", &fmt!("UDP:{port}")
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()
                .await?;
    
            if !output.status.success() {
                info!("Port {port} already is free");
                return Ok(());
            }

            // parse output PIDs: 
            let pids: Vec<i32> = String::from_utf8_lossy(&output.stdout)
                .trim()
                .split('\n')
                .filter_map(|line| {
                    line.trim().parse().ok()
                        .filter(|&pid| pid > 1)  // exclude system ones
                })
                .collect();

            if pids.is_empty() {
                info!("No processes found on port {port}");
                return Ok(());
            }

            info!("Found {} processes on port {port}: {pids:?}", pids.len());

            // stop all PIDs: 
            for &pid in &pids {
                let pid_str = pid.to_string();
                if Command::new("kill").args(["-TERM", &pid_str]).status().await?.success() {
                    info!("Graceful stop PID {pid}");
                } else {
                    let _ = Command::new("kill").args(["-9", &pid_str]).status();
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

        Ok(())
    }
}
