<h1 align="center">umami-alerts</h1>

A fast, efficient daily analytics report generator for [Umami Analytics](https://umami.is/). This tool fetches your analytics data and sends simple, detailed email reports including:

- Pageviews and visitor statistics
- Engagement metrics (bounce rates, time spent)
- Top referrers and traffic sources
- Geographic distribution of visitors
- Browser and device breakdowns

## Installation

### Using Cargo

```bash
# Install directly from git
$ cargo install --git https://github.com/Thunderbottom/umami-alerts

# Or clone and build
$ git clone https://github.com/Thunderbottom/umami-alerts
$ cd umami-alerts
$ cargo build --release
```

### Using Nix

```bash
$ nix build github:Thunderbottom/umami-alerts
$ ./results/bin/umami-alerts -c config.toml
```

## Configuration

Create a `config.toml` file:

```toml
[app]
debug = false
dry_run = false
max_concurrent_jobs = 4
report_type = "weekly"

[smtp]
host = "smtp.example.com"
port = 587
username = "your-username"
password = "your-password"
from = "reports@example.com"
skip_tls_verify = false
tls = true

[websites.example]
disabled = true
base_url = "https://analytics.example.com"
id = "e97f683e-12e8-4fb5-970b-f5171804fe21"
name = "Example Website"
username = "your-username"
password = "your-password"
recipients = ["user@example.com"]
timezone = "UTC"

[websites.example-io]
base_url = "https://umami.example.com"
id = "e4de62a3-d40a-40da-b900-3ea016893f38"
name = "example.io"
username = "umami-user"
password = "hunter2"
recipients = [
    "user2@example.com",
    "user3@example.com",
]
timezone = "Asia/Kolkata"

```

You may add multiple such websites under `[websites]` as `[websites.new-example]` with the site's configuration.

## Usage

```bash
# Run with default config path
$ umami-alerts

# Specify config path
$ umami-alerts --config /path/to/config.toml
```
### Crontab Configuration

`umami-alerts` is meant to be run as an everyday-cron to send daily reports.

```bash
# Add an entry to crontab to run at 8am daily
0 8 * * * /path/to/umami-alerts --config /path/to/config.toml
```

## Development

### Prerequisites

- Rust 1.70 or higher
- OpenSSL development libraries
- GCC or compatible C compiler
- pkg-config

### Building from Source

```bash
# Using cargo
$ cargo build --release

# Using nix develop
$ nix develop
(nix shell) $ cargo build --release
```

### Running Tests

```bash
cargo test
```

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

MIT License

```
Copyright (c) 2025 Chinmay D. Pai

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
