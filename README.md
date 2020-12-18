# Tiny file server

The simplest file server for web development purposes.

```rust
use env_logger::Env;
use tiny_file_server::FileServer;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    FileServer::http("127.0.0.1:9080")
        .expect("Can't create http server")
        .run("path/to/static/files")
        .expect("Server error");
}
```

## Development notes

To check the project, use the following command:

```shell script
cargo check --all-features --all-targets
```

To run all tests, use the following command:

```shell script
cargo test --all-features --all-targets
```

To check and perform formatting, use the following commands:

```shell script
cargo +nightly fmt -- --check
cargo +nightly fmt
```

To enable autoformatting for IntelliJ IDEA with the Rust plugin:

`Settings -> Languages and Frameworks -> Rust -> Rustfmt -> Run rustfmt on Save`

To run clippy, use the following command:

```shell script
cargo clippy --all-targets --all-features -- -D warnings
```

To setup git hook, use the following command:

```shell script
cp .git-pre-push.sh .git/hooks/pre-push
```