use crate::{ prelude::*, remote::Bind };
use tokio::io::{ AsyncBufReadExt, AsyncReadExt, BufReader };
use tokio_serial::{ SerialStream, SerialPortBuilderExt };
use tokio::fs as tfs;

/// The serial port controller
pub struct IRController;

impl IRController {
    pub async fn listen() -> Result<()> {
        tokio::spawn(async move {
            loop {
                let cfg = Settings::get_updated().unwrap().arduino.clone();
                let port = cfg.port;
                let rate = cfg.rate;

                // init COM port:
                let path = fmt!("COM{}", port);
                let serial = tokio_serial::new(&path, rate)
                    .baud_rate(rate as u32)
                    .timeout(Duration::from_millis(100))
                    .open_native_async()
                    .map_err(|e| { err!("Failed to get COM{port} port: {e}"); e });
   
                if let Ok(serial) = serial {
                    if let Err(e) = Self::handle_port(serial, port, rate).await {
                        err!("COM-port reader panicked with error: {e}");
                    } else {
                        break;
                    }
                } else {
                    sleep(Duration::from_millis(200)).await;
                }
            }
        });

        Ok(())
    }

    async fn handle_port(serial: SerialStream, port: u16, rate: u32) -> Result<()> {
        // init reader:
        let mut com_reader = BufReader::new(serial);
        let mut buffer = str!("");
        
        // init help vars:
        let mut last_code = String::new();
        let mut last_action = Instant::now();
        let mut last_millis = 0i64;
        let repeat_timeout_min = Duration::from_millis(200);
        let repeat_timeout_max = Duration::from_millis(800);

        info!("Listening on COM{port} at {rate} baud..");
        
        loop {
            // reading remote code:
            match com_reader.read_line(&mut buffer).await {
                // Ok(0) => continue,
                Ok(n) => {
                    dbg!(n);
                    
                    let json_doc: JsonValue = json::from_str(buffer.trim())?;
                    let code = if let Some(code) = json_doc.get("code").and_then(|s| s.as_str()) { code.to_owned() }else{ continue };
                    let millis = if let Some(millis) = json_doc.get("millis").and_then(|v| v.as_i64()) { millis }else{ continue };

                    // get millis:
                    let millis_diff = millis - last_millis;
                    last_millis = millis;

                    // validate remote code:
                    if !code.starts_with("0x") || code.len() < 8 { continue }
                    info!("Pressed button '{code}'..");
                    
                    // handle last bind repeat:
                    if code == "0xFFFFFFFF" {
                        if last_code.is_empty()
                        || last_action.elapsed() < repeat_timeout_min
                        || last_action.elapsed() > repeat_timeout_max
                        {
                            continue
                        }

                        if let Err(e) = Self::handle_code(&last_code, true, millis_diff).await {
                            err!("Error with handling code: {e}");
                        }
                    }
                    // handle new code:
                    else {                        
                        if code != last_code {
                            last_code = code;
                        }
                        if let Err(e) = Self::handle_code(&last_code, false, millis_diff).await {
                            err!("Error with handling bind: {e}");
                        }
                    }

                    last_action = Instant::now();
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
                Err(e) => return Err(fmt!("Error with reading 'COM{port}' port: {e}").into())
            }
        }
    }

    /// Handles IR-code
    async fn handle_code(code: &str, repeat: bool, millis_diff: i64) -> Result<()> {
        let code = code.to_string();
       
        // read binds config:
        let binds_path = app_data().join("config/binds.json");
        let json_str = tfs::read_to_string(binds_path).await?;
        let binds: Vec<Bind> = json::from_str(&json_str)?;


        // handle IR-code:
        for bind in binds {
            if bind.codes.contains(&code) {
                if let Err(e) = bind.handle(&code, repeat, millis_diff).await {
                    err!("Handle IR-code '{code}' error: {e}");
                };
                // return Ok(());
            }
        }
       
        Ok(())
    }
}
