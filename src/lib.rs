//! The simplest file server for web development purposes.
//!
//! ```no_run
//! use env_logger::{Builder, Env};
//! use tiny_file_server::FileServer;
//!
//! fn main() {
//!     Builder::from_env(Env::default().default_filter_or("debug")).init();
//!
//!     FileServer::http("127.0.0.1:9080")
//!         .expect("Server should be created")
//!         .run("path/to/static/files")
//!         .expect("Server should start");
//! }
//! ```

use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::File;
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::str::FromStr;
use std::{error, io};

use log::{debug, info};
use tiny_http::{Header, Response, Server, StatusCode};

pub type Error = Box<dyn error::Error + Send + Sync + 'static>;

pub struct FileServer {
    server: Server,
    default_file: Cow<'static, str>,
    default_content_type: Cow<'static, str>,
    content_type_by_extension: HashMap<&'static str, &'static str>,
    url_to_file: fn(&str, PathBuf, &str) -> PathBuf,
}

impl FileServer {
    pub fn http(addr: impl ToSocketAddrs + Display) -> Result<Self, Error> {
        info!("Starting file server on http://{addr}");
        let server = Server::http(addr)?;

        let content_type_by_extension = [
            ("js", "application/javascript"),
            ("wasm", "application/wasm"),
            ("html", "text/html"),
            ("css", "text/css"),
        ]
        .into_iter()
        .collect();

        Ok(Self {
            server,
            default_file: "index.html".into(),
            default_content_type: "text/plain".into(),
            content_type_by_extension,
            url_to_file,
        })
    }

    pub fn with_default_file(mut self, file_name: impl Into<Cow<'static, str>>) -> Self {
        self.set_default_file(file_name);
        self
    }

    pub fn with_default_content_type(mut self, content_type: impl Into<Cow<'static, str>>) -> Self {
        self.set_default_content_type(content_type);
        self
    }

    pub fn with_content_type_by_extension(
        mut self,
        content_types: impl IntoIterator<Item = (&'static str, &'static str)>,
    ) -> Self {
        self.content_type_by_extension.extend(content_types);
        self
    }

    pub fn with_url_to_file(mut self, url_to_file: fn(&str, PathBuf, &str) -> PathBuf) -> Self {
        self.url_to_file = url_to_file;
        self
    }

    pub fn set_default_file(&mut self, file_name: impl Into<Cow<'static, str>>) {
        self.default_file = file_name.into();
    }

    pub fn set_default_content_type(&mut self, content_type: impl Into<Cow<'static, str>>) {
        self.default_content_type = content_type.into();
    }

    pub fn set_url_to_file(&mut self, url_to_file: fn(&str, PathBuf, &str) -> PathBuf) {
        self.url_to_file = url_to_file;
    }

    pub fn content_type_by_extension(&self) -> &HashMap<&'static str, &'static str> {
        &self.content_type_by_extension
    }

    pub fn content_type_by_extension_mut(&mut self) -> &mut HashMap<&'static str, &'static str> {
        &mut self.content_type_by_extension
    }

    pub fn unblock(&self) {
        self.server.unblock();
    }

    pub fn run(&self, statics_path: impl Into<PathBuf>) -> Result<(), io::Error> {
        let statics_path = statics_path.into();
        info!("Listen incoming requests to {}", statics_path.display());
        for request in self.server.incoming_requests() {
            debug!(
                "Received request. Method: {:?}, url: {:?}, headers: {:?}",
                request.method(),
                request.url(),
                request.headers()
            );

            let file_path = (self.url_to_file)(request.url(), statics_path.clone(), self.default_file.borrow());

            debug!("Requested file: {}", file_path.display());

            if !file_path.exists() {
                let status = StatusCode(404);
                debug!("Status: {} ({})", status.default_reason_phrase(), status.0);
                request.respond(Response::empty(status))?;
            } else {
                match File::open(&file_path) {
                    Ok(file) => {
                        let mut response = Response::from_file(file);
                        let content_type = file_path
                            .extension()
                            .and_then(OsStr::to_str)
                            .and_then(|ext| self.content_type_by_extension.get(ext).copied())
                            .unwrap_or(&self.default_content_type);
                        response.add_header(
                            Header::from_str(&format!("Content-Type: {}", content_type))
                                .map_err(|_| io::Error::from(io::ErrorKind::Other))?,
                        );
                        request.respond(response)?;
                    },
                    Err(err) => {
                        let status = StatusCode(500);
                        debug!("Status: {} ({})", status.default_reason_phrase(), status.0);
                        debug!("Error: {err:?}");
                        request.respond(Response::empty(status))?;
                    },
                }
            };
        }
        info!("File server socket is shutdown");
        Ok(())
    }
}

pub fn url_to_file(url: &str, statics_path: PathBuf, default_file: &str) -> PathBuf {
    let mut file_path = statics_path;
    if url.len() > 1 {
        for chunk in url.trim_start_matches('/').split('/') {
            file_path.push(chunk);
        }
    } else {
        file_path.push(default_file);
    };
    file_path
}
