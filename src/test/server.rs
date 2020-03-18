use crate::{
    jsonrpc::server::{Middleware, Result},
    protocol::*,
};
use aovec::Aovec;
use chashmap::CHashMap;
use futures::lock::Mutex;
use futures_boxed::boxed;
use jsonrpc_derive::{jsonrpc_method, jsonrpc_server};

pub struct TestLatexLspServer {
    pub options: Mutex<Options>,
    pub show_message_buf: Aovec<ShowMessageParams>,
    pub register_capability_buf: Aovec<RegistrationParams>,
    pub diagnostics_by_uri: CHashMap<Uri, Vec<Diagnostic>>,
    pub progress_buf: Aovec<ProgressParams>,
    pub work_done_progress_create_buf: Aovec<WorkDoneProgressCreateParams>,
    pub log_message_buf: Aovec<LogMessageParams>,
}

#[jsonrpc_server]
impl TestLatexLspServer {
    pub fn new(options: Options) -> Self {
        let base = 16;
        Self {
            options: Mutex::new(options),
            show_message_buf: Aovec::new(base),
            register_capability_buf: Aovec::new(base),
            diagnostics_by_uri: CHashMap::new(),
            progress_buf: Aovec::new(base),
            work_done_progress_create_buf: Aovec::new(base),
            log_message_buf: Aovec::new(base),
        }
    }

    #[jsonrpc_method("workspace/configuration", kind = "request")]
    pub async fn configuration(&self, params: ConfigurationParams) -> Result<serde_json::Value> {
        let options = self.options.lock().await;
        if params.items[0].section.as_ref().unwrap() == "latex" {
            Ok(serde_json::to_value(vec![options.latex.clone().unwrap_or_default()]).unwrap())
        } else {
            Ok(serde_json::to_value(vec![options.bibtex.clone().unwrap_or_default()]).unwrap())
        }
    }

    #[jsonrpc_method("window/showMessage", kind = "notification")]
    pub async fn show_message(&self, params: ShowMessageParams) {
        self.show_message_buf.push(params);
    }

    #[jsonrpc_method("client/registerCapability", kind = "request")]
    pub async fn register_capability(&self, params: RegistrationParams) -> Result<()> {
        self.register_capability_buf.push(params);
        Ok(())
    }

    #[jsonrpc_method("textDocument/publishDiagnostics", kind = "notification")]
    #[boxed]
    pub async fn publish_diagnostics(&self, params: PublishDiagnosticsParams) {
        let _ = self
            .diagnostics_by_uri
            .insert(params.uri.into(), params.diagnostics);
    }

    #[jsonrpc_method("$/progress", kind = "notification")]
    #[boxed]
    pub async fn progress(&self, params: ProgressParams) {
        self.progress_buf.push(params);
    }

    #[jsonrpc_method("window/workDoneProgress/create", kind = "request")]
    #[boxed]
    pub async fn work_done_progress_create(
        &self,
        params: WorkDoneProgressCreateParams,
    ) -> Result<()> {
        self.work_done_progress_create_buf.push(params);
        Ok(())
    }

    #[jsonrpc_method("window/logMessage", kind = "notification")]
    #[boxed]
    pub async fn log_message(&self, params: LogMessageParams) {
        self.log_message_buf.push(params);
    }
}

impl Middleware for TestLatexLspServer {
    #[boxed]
    async fn before_message(&self) {}

    #[boxed]
    fn after_message(&self) {}
}