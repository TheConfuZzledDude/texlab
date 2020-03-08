use crate::{
    protocol::*,
    tex::{DynamicDistribution, Language},
    workspace::{Document, DocumentParams, Snapshot},
};
use futures_boxed::boxed;
use std::{
    env,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DocumentView {
    pub snapshot: Arc<Snapshot>,
    pub current: Arc<Document>,
    pub related: Vec<Arc<Document>>,
}

impl DocumentView {
    pub fn analyze(
        snapshot: Arc<Snapshot>,
        current: Arc<Document>,
        options: &Options,
        current_dir: &Path,
    ) -> Self {
        let related = snapshot.relations(&current.uri, options, current_dir);
        Self {
            snapshot,
            current,
            related,
        }
    }
}

#[derive(Clone)]
pub struct FeatureRequest<P> {
    pub params: P,
    pub view: DocumentView,
    pub distro: DynamicDistribution,
    pub client_capabilities: Arc<ClientCapabilities>,
    pub options: Options,
    pub current_dir: Arc<PathBuf>,
}

impl<P> FeatureRequest<P> {
    pub fn snapshot(&self) -> &Snapshot {
        &self.view.snapshot
    }

    pub fn current(&self) -> &Document {
        &self.view.current
    }

    pub fn related(&self) -> &[Arc<Document>] {
        &self.view.related
    }
}

pub trait FeatureProvider {
    type Params;
    type Output;

    #[boxed]
    async fn execute<'a>(&'a self, req: &'a FeatureRequest<Self::Params>) -> Self::Output;
}

type ListProvider<P, O> = Box<dyn FeatureProvider<Params = P, Output = Vec<O>> + Send + Sync>;

#[derive(Default)]
pub struct ConcatProvider<P, O> {
    providers: Vec<ListProvider<P, O>>,
}

impl<P, O> ConcatProvider<P, O> {
    pub fn new(providers: Vec<ListProvider<P, O>>) -> Self {
        Self { providers }
    }
}

impl<P, O> FeatureProvider for ConcatProvider<P, O>
where
    P: Send + Sync,
    O: Send + Sync,
{
    type Params = P;
    type Output = Vec<O>;

    #[boxed]
    async fn execute<'a>(&'a self, req: &'a FeatureRequest<P>) -> Vec<O> {
        let mut items = Vec::new();
        for provider in &self.providers {
            items.append(&mut provider.execute(req).await);
        }
        items
    }
}

type OptionProvider<P, O> = Box<dyn FeatureProvider<Params = P, Output = Option<O>> + Send + Sync>;

#[derive(Default)]
pub struct ChoiceProvider<P, O> {
    providers: Vec<OptionProvider<P, O>>,
}

impl<P, O> ChoiceProvider<P, O> {
    pub fn new(providers: Vec<OptionProvider<P, O>>) -> Self {
        Self { providers }
    }
}

impl<P, O> FeatureProvider for ChoiceProvider<P, O>
where
    P: Send + Sync,
    O: Send + Sync,
{
    type Params = P;
    type Output = Option<O>;

    #[boxed]
    async fn execute<'a>(&'a self, req: &'a FeatureRequest<P>) -> Option<O> {
        for provider in &self.providers {
            let item = provider.execute(req).await;
            if item.is_some() {
                return item;
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct FeatureTester {
    main: String,
    files: Vec<(String, String)>,
    distro: DynamicDistribution,
    position: Position,
    new_name: String,
    include_declaration: bool,
    client_capabilities: Arc<ClientCapabilities>,
    current_dir: Arc<PathBuf>,
    root_dir: Option<PathBuf>,
    output_dir: Option<PathBuf>,
}

impl FeatureTester {
    pub fn new() -> Self {
        Self {
            main: String::new(),
            files: Vec::new(),
            distro: DynamicDistribution::default(),
            position: Position::default(),
            new_name: String::new(),
            include_declaration: false,
            client_capabilities: Arc::default(),
            current_dir: Arc::new(env::temp_dir()),
            root_dir: None,
            output_dir: None,
        }
    }

    pub fn main<S: Into<String>>(&mut self, name: S) -> &mut Self {
        self.main = name.into();
        self
    }

    pub fn file<S, T>(&mut self, name: S, text: T) -> &mut Self
    where
        S: Into<String>,
        T: Into<String>,
    {
        self.files.push((name.into(), text.into()));
        self
    }

    pub fn position(&mut self, line: u64, character: u64) -> &mut Self {
        self.position = Position::new(line, character);
        self
    }

    pub fn new_name<S: Into<String>>(&mut self, value: S) -> &mut Self {
        self.new_name = value.into();
        self
    }

    pub fn include_declaration(&mut self) -> &mut Self {
        self.include_declaration = true;
        self
    }

    pub fn root_directory<P: Into<PathBuf>>(&mut self, path: P) -> &mut Self {
        self.root_dir = Some(path.into());
        self
    }

    pub fn output_directory<P: Into<PathBuf>>(&mut self, path: P) -> &mut Self {
        self.output_dir = Some(path.into());
        self
    }

    pub fn uri(name: &str) -> Uri {
        let path = env::temp_dir().join(name);
        Uri::from_file_path(path).unwrap()
    }

    fn identifier(&self) -> TextDocumentIdentifier {
        let uri = Self::uri(&self.main);
        TextDocumentIdentifier::new(uri.into())
    }

    fn options(&self) -> Options {
        Options {
            latex: Some(LatexOptions {
                build: Some(LatexBuildOptions {
                    output_directory: self.output_dir.clone(),
                    ..LatexBuildOptions::default()
                }),
                root_directory: self.root_dir.clone(),
                ..LatexOptions::default()
            }),
            ..Options::default()
        }
    }

    async fn view(&self) -> DocumentView {
        let mut snapshot = Snapshot::new();
        let resolver = self.distro.0.resolver().await;
        let options = self.options();
        for (name, text) in &self.files {
            let uri = Self::uri(name);
            let path = uri.to_file_path().unwrap();
            let language = path
                .extension()
                .and_then(|ext| ext.to_str())
                .and_then(Language::by_extension)
                .unwrap();
            let doc = Document::open(DocumentParams {
                uri,
                text: text.clone(),
                language,
                resolver: &resolver,
                options: &options,
                current_dir: &self.current_dir,
            });
            snapshot.push(doc);
        }
        let current = snapshot.find(&Self::uri(&self.main)).unwrap();
        DocumentView::analyze(Arc::new(snapshot), current, &options, &self.current_dir)
    }

    async fn request<P>(&self, params: P) -> FeatureRequest<P> {
        FeatureRequest {
            params,
            view: self.view().await,
            client_capabilities: Arc::clone(&self.client_capabilities),
            distro: self.distro.clone(),
            options: self.options(),
            current_dir: Arc::clone(&self.current_dir),
        }
    }

    pub async fn test_position<F, O>(&self, provider: F) -> O
    where
        F: FeatureProvider<Params = TextDocumentPositionParams, Output = O>,
    {
        let text_document = self.identifier();
        let params = TextDocumentPositionParams::new(text_document, self.position);
        let req = self.request(params).await;
        provider.execute(&req).await
    }

    pub async fn test_completion<F, O>(&self, provider: F) -> O
    where
        F: FeatureProvider<Params = CompletionParams, Output = O>,
    {
        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams::new(
                self.identifier(),
                self.position,
            ),
            context: None,
        };
        let req = self.request(params).await;
        provider.execute(&req).await
    }

    pub async fn test_folding<F, O>(&self, provider: F) -> O
    where
        F: FeatureProvider<Params = FoldingRangeParams, Output = O>,
    {
        let text_document = self.identifier();
        let params = FoldingRangeParams { text_document };
        let req = self.request(params).await;
        provider.execute(&req).await
    }

    pub async fn test_link<F, O>(&self, provider: F) -> O
    where
        F: FeatureProvider<Params = DocumentLinkParams, Output = O>,
    {
        let text_document = self.identifier();
        let params = DocumentLinkParams { text_document };
        let req = self.request(params).await;
        provider.execute(&req).await
    }

    pub async fn test_reference<F, O>(&self, provider: F) -> O
    where
        F: FeatureProvider<Params = ReferenceParams, Output = O>,
    {
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams::new(
                self.identifier(),
                self.position,
            ),
            context: ReferenceContext {
                include_declaration: self.include_declaration,
            },
        };
        let req = self.request(params).await;
        provider.execute(&req).await
    }

    pub async fn test_rename<F, O>(&self, provider: F) -> O
    where
        F: FeatureProvider<Params = RenameParams, Output = O>,
    {
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams::new(
                self.identifier(),
                self.position,
            ),
            new_name: self.new_name.clone(),
        };
        let req = self.request(params).await;
        provider.execute(&req).await
    }

    pub async fn test_symbol<F, O>(&self, provider: F) -> O
    where
        F: FeatureProvider<Params = DocumentSymbolParams, Output = O>,
    {
        let text_document = self.identifier();
        let params = DocumentSymbolParams { text_document };
        let req = self.request(params).await;
        provider.execute(&req).await
    }
}