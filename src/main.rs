use std::sync::Arc;

use anyhow::Result;
use bytes::Bytes;
use clap::Parser;
use log::{debug, error, info, warn};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use url::Url;
use web_transport_quinn::{Client, ClientBuilder, Session};

#[derive(Parser, Debug)]
#[command(
    name = "webtranscat",
    about = "WebTransport client equivalent to websocat",
    version = "0.1.0"
)]
struct Args {
    /// WebTransport URL to connect to
    url: Url,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

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

async fn create_client(args: &Args) -> Result<Client> {
    if args.insecure {
        warn!("Certificate verification disabled (--insecure)");
        // SAFETY: This is intentionally insecure for testing purposes
        Ok(unsafe { ClientBuilder::new().with_no_certificate_verification()? })
    } else {
        // Use default secure configuration - let's try the basic new() method
        Ok(ClientBuilder::new().with_system_roots()?)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Enable logging
    let env =
        env_logger::Env::default().default_filter_or(if args.verbose { "debug" } else { "info" });
    env_logger::init_from_env(env);

    if args.verbose {
        info!("webtranscat starting");
        debug!("Arguments: {:?}", args);
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
        let verbose = args.verbose;
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
                        error!("Datagram error: {}", e);
                        break;
                    }
                }
            }
        }));
    }

    // Handle unidirectional streams
    {
        let session = session.clone();
        let verbose = args.verbose;
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
                                error!("Stream read error: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Stream accept error: {}", e);
                        break;
                    }
                }
            }
        }));
    }

    // Handle stdin input (if not unidirectional)
    if !args.unidirectional {
        let session = session.clone();
        let verbose = args.verbose;

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
                            error!("Failed to send datagram: {}", e);
                        } else if verbose {
                            debug!("Datagram sent successfully");
                        }
                    }
                    Err(e) => {
                        error!("Error reading from stdin: {}", e);
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
