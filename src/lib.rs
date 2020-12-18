use std::{
    borrow::Cow, collections::HashMap, error, ffi::OsStr, fmt::Display, fs::File, io, net::ToSocketAddrs,
    path::PathBuf, str::FromStr,
};

use log::{debug, info};
use std::borrow::Borrow;
use tiny_http::{Header, Response, Server, StatusCode};

pub type Error = Box<dyn error::Error + Send + Sync + 'static>;

pub struct FileServer {
    server: Server,
    default_file: Cow<'static, str>,
    default_content_type: Cow<'static, str>,
    content_type_by_extension: HashMap<&'static str, &'static str>,
}

impl FileServer {
    pub fn new(addr: impl ToSocketAddrs + Display) -> Result<Self, Error> {
        info!("Starting file server on {}", addr);
        let server = Server::http(addr)?;

        let content_type_by_extension = [
            ("js", "application/javascript"),
            ("wasm", "application/wasm"),
            ("html", "text/html"),
            ("css", "text/css"),
        ]
        .iter()
        .cloned()
        .collect();

        Ok(Self {
            server,
            default_file: "index.html".into(),
            default_content_type: "text/plain".into(),
            content_type_by_extension,
        })
    }

    pub fn set_default_file(&mut self, file_name: impl Into<Cow<'static, str>>) {
        self.default_file = file_name.into();
    }

    pub fn set_default_content_type(&mut self, content_type: impl Into<Cow<'static, str>>) {
        self.default_content_type = content_type.into();
    }

    pub fn content_type_by_extension(&self) -> &HashMap<&'static str, &'static str> {
        &self.content_type_by_extension
    }

    pub fn content_type_by_extension_mut(&mut self) -> &mut HashMap<&'static str, &'static str> {
        &mut self.content_type_by_extension
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

            let mut file_path = statics_path.clone();
            if request.url().len() > 1 {
                for chunk in request.url().trim_start_matches('/').split('/') {
                    file_path.push(chunk);
                }
            } else {
                let default_file: &str = self.default_file.borrow();
                file_path.push(default_file);
            };

            debug!("    Requested file: {}", file_path.display());

            if !file_path.exists() {
                let status = StatusCode(404);
                debug!("    Status: {} ({})", status.default_reason_phrase(), status.0);
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
                    }
                    Err(err) => {
                        let status = StatusCode(500);
                        debug!("    Status: {} ({})", status.default_reason_phrase(), status.0);
                        debug!("    Error: {:?}", err);
                        request.respond(Response::empty(status))?;
                    }
                }
            };
        }
        info!("File server socket is shutdown");
        Ok(())
    }
}
