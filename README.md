# webtranscat

[![Crates.io](https://img.shields.io/crates/v/webtranscat.svg)](https://crates.io/crates/webtranscat)
[![Documentation](https://docs.rs/webtranscat/badge.svg)](https://docs.rs/webtranscat)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/securityunion/webtranscat#license)

A WebTransport equivalent of `websocat` - a command-line WebTransport client for debugging and testing.

**webtranscat** was originally developed to debug [videocall.rs](https://videocall.rs), a WebRTC video calling application that uses WebTransport for data channels.

## Features

- 🔗 **WebTransport Client**: Connect to WebTransport servers using QUIC
- 📡 **Datagram Support**: Send and receive unreliable datagrams  
- 🌊 **Stream Support**: Handle reliable unidirectional streams
- 🔄 **Bidirectional**: Echo incoming data and send stdin input
- 🔧 **websocat-like CLI**: Familiar interface for websocat users
- 🛡️ **Security Options**: Support for both secure and insecure connections
- 📊 **Verbose Logging**: Multiple verbosity levels for debugging

## Installation

### From crates.io

```bash
cargo install webtranscat
```

### From source

```bash
git clone https://github.com/securityunion/webtranscat.git
cd webtranscat
cargo install --path .
```

## Usage

### Basic Examples

```bash
# Connect to a WebTransport server (interactive mode)
webtranscat wss://example.com:4443

# Connect with verbose logging
webtranscat -v wss://example.com:4443

# Connect with insecure certificate verification (for testing)
webtranscat --insecure wss://localhost:4443

# Listen-only mode (don't send stdin input)
webtranscat -u wss://example.com:4443

# Exit after receiving one message
webtranscat -1 wss://example.com:4443

# Multiple verbosity levels (like websocat)
webtranscat -vv wss://example.com:4443   # Debug level
webtranscat -vvv wss://example.com:4443  # Trace level

# Quiet mode (suppress diagnostic messages)
webtranscat -q wss://example.com:4443
```

### Environment Variables

```bash
# Override logging with RUST_LOG
RUST_LOG=debug webtranscat wss://example.com:4443

# Early startup debugging
WEBTRANSCAT_EARLY_LOG=1 webtranscat wss://example.com:4443
```

## Command Line Options

```
Usage: webtranscat [OPTIONS] <URL>

Arguments:
  <URL>  WebTransport URL to connect to

Options:
  -v...                 Increase verbosity level to info or further
  -q                    Suppress all diagnostic messages, except of startup errors
      --insecure        Skip certificate verification (insecure)
  -u, --unidirectional  Only listen for incoming data, don't send from stdin
  -1, --one-message     Exit after receiving one message
  -h, --help            Print help
  -V, --version         Print version
```

## How it Works

webtranscat establishes a WebTransport connection and concurrently:

1. **Listens for incoming datagrams** - unreliable, unordered packets (UDP-like but encrypted)
2. **Accepts unidirectional streams** - reliable, ordered byte streams (TCP-like but multiplexed)  
3. **Sends stdin input as datagrams** - user input becomes outgoing datagrams (unless `-u` is used)

All received data is echoed to stdout with newlines, making it easy to see what's being received.

## Logging Levels

Following websocat's logging approach:

- **No flags**: `WARN` level (warnings + errors only)
- **`-v`**: `INFO` level (connection info + warnings + errors)  
- **`-vv`**: `DEBUG` level (detailed debugging info)
- **`-vvv+`**: `TRACE` level (everything)

## Development Origin

This tool was created to debug [videocall.rs](https://videocall.rs), a WebRTC video calling application that leverages WebTransport for efficient data channel communication. During development, we needed a simple way to test WebTransport connections, inspect datagrams and streams, and debug connectivity issues.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

## Disclaimer

This tool is primarily intended for debugging and testing. While it supports secure connections, please review the security implications before using in production environments. 