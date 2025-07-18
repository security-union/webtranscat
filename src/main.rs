use anyhow::Result;
use bytes::Bytes;
use clap::Parser;
use log::{debug, error, info, warn};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use url::Url;
use web_transport_quinn::{Client, ClientBuilder};

#[derive(Parser, Debug)]
#[command(
    name = "webtranscat",
    about = "WebTransport client equivalent to websocat",
    version = "0.1.0"
)]
struct Args {
    /// WebTransport URL to connect to
    url: Url,

    /// Increase verbosity level to info or further
    #[arg(
        short = 'v',
        action = clap::ArgAction::Count,
        help = "Increase verbosity level to info or further"
    )]
    verbosity: u8,

    /// Suppress all diagnostic messages, except of startup errors
    #[arg(
        short = 'q',
        help = "Suppress all diagnostic messages, except of startup errors"
    )]
    quiet: bool,

    /// Skip certificate verification (insecure)
    #[arg(long)]
    insecure: bool,

    /// Only listen for incoming data, don't send from stdin
    #[arg(short = 'u', long)]
    unidirectional: bool,

    /// Exit after receiving one message
    #[arg(short = '1', long)]
    one_message: bool,
}

// Based on websocat's logging approach
mod logging {
    use anyhow::Result;
    use log::Level;

    pub fn setup_env_logger(ll: u8) -> Result<()> {
        if std::env::var("RUST_LOG").is_ok() {
            if ll > 0 {
                eprintln!("webtranscat: RUST_LOG environment variable overrides any -v");
            }
            env_logger::init();
            return Ok(());
        }

        let lf = match ll {
            0 => Level::Warn,  // Default: warnings and errors only
            1 => Level::Info,  // -v: info, warnings, errors
            2 => Level::Debug, // -vv: debug, info, warnings, errors
            _ => Level::Trace, // -vvv+: trace (everything)
        }
        .to_level_filter();

        env_logger::Builder::new()
            .filter(Some("webtranscat"), lf)
            .filter(None, Level::Warn.to_level_filter())
            .try_init()?;
        Ok(())
    }
}

async fn create_client(args: &Args) -> Result<Client> {
    if args.insecure {
        warn!("Certificate verification disabled (--insecure)");
        // SAFETY: This is intentionally insecure for testing purposes
        Ok(unsafe { ClientBuilder::new().with_no_certificate_verification()? })
    } else {
        // Use default secure configuration with system certificates
        Ok(ClientBuilder::new().with_system_roots()?)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Handle early logging like websocat
    let mut logging_already_set = false;
    if std::env::var("WEBTRANSCAT_EARLY_LOG").is_ok() {
        logging::setup_env_logger(0)?;
        logging_already_set = true;
    }

    let args = Args::parse();
    let quiet = args.quiet;

    // Setup logging (if not already done and not in quiet mode)
    if !quiet && !logging_already_set {
        logging::setup_env_logger(args.verbosity)?;
    }

    if args.verbosity > 0 {
        info!("webtranscat starting");
        debug!("Arguments: {args:?}");
    }

    // Create client and connect
    let client = create_client(&args).await?;
    info!("connecting to {}", args.url);
    let session = client.connect(args.url.clone()).await?;
    info!("connected");

    // Run the echo logic
    let mut handles = Vec::new();

    // Handle datagrams
    {
        let session = session.clone();
        let verbose = args.verbosity > 0;
        let one_message = args.one_message;

        handles.push(tokio::spawn(async move {
            loop {
                match session.read_datagram().await {
                    Ok(data) => {
                        if verbose {
                            info!("Received datagram: {} bytes", data.len());
                        }
                        let _ = io::stdout().write_all(&data).await;
                        let _ = io::stdout().write_all(b"\n").await;
                        let _ = io::stdout().flush().await;

                        if one_message {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Datagram error: {e}");
                        break;
                    }
                }
            }
        }));
    }

    // Handle unidirectional streams
    {
        let session = session.clone();
        let verbose = args.verbosity > 0;
        let one_message = args.one_message;

        handles.push(tokio::spawn(async move {
            loop {
                match session.accept_uni().await {
                    Ok(mut stream) => {
                        if verbose {
                            info!("Accepted unidirectional stream");
                        }

                        match stream.read_to_end(usize::MAX).await {
                            Ok(data) => {
                                if verbose {
                                    info!("Read {} bytes from stream", data.len());
                                }
                                let _ = io::stdout().write_all(&data).await;
                                let _ = io::stdout().write_all(b"\n").await;
                                let _ = io::stdout().flush().await;

                                if one_message {
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Stream read error: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        error!("Stream accept error: {e}");
                        break;
                    }
                }
            }
        }));
    }

    // Handle stdin input (if not unidirectional)
    if !args.unidirectional {
        let session = session.clone();
        let verbose = args.verbosity > 0;

        handles.push(tokio::spawn(async move {
            let stdin = io::stdin();
            let mut reader = BufReader::new(stdin);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        if verbose {
                            info!("EOF on stdin");
                        }
                        break;
                    }
                    Ok(_) => {
                        let data = line.trim_end().as_bytes();

                        if verbose {
                            info!("Sending {} bytes as datagram", data.len());
                        }

                        if let Err(e) = session.send_datagram(Bytes::from(data.to_vec())) {
                            error!("Failed to send datagram: {e}");
                        } else if verbose {
                            debug!("Datagram sent successfully");
                        }
                    }
                    Err(e) => {
                        error!("Error reading from stdin: {e}");
                        break;
                    }
                }
            }
        }));
    }

    // Wait for any task to complete
    let (result, _index, _remaining) = futures::future::select_all(handles).await;
    result?;

    Ok(())
}
