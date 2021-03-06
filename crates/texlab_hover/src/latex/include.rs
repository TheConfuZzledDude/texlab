use futures_boxed::boxed;
use texlab_protocol::RangeExt;
use texlab_protocol::*;
use texlab_syntax::*;
use texlab_workspace::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct LatexIncludeHoverProvider;

impl FeatureProvider for LatexIncludeHoverProvider {
    type Params = TextDocumentPositionParams;
    type Output = Option<Hover>;

    #[boxed]
    async fn execute<'a>(
        &'a self,
        request: &'a FeatureRequest<TextDocumentPositionParams>,
    ) -> Option<Hover> {
        let (range, targets) = Self::find_include(request)?;
        for target in targets {
            if let Some(document) = request.workspace().find(&target) {
                let path = document.uri.to_file_path().ok()?;
                return Some(Hover {
                    range: Some(range),
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::PlainText,
                        value: path.to_string_lossy().into_owned(),
                    }),
                });
            }
        }
        None
    }
}

impl LatexIncludeHoverProvider {
    fn find_include(
        request: &FeatureRequest<TextDocumentPositionParams>,
    ) -> Option<(Range, &[Uri])> {
        if let SyntaxTree::Latex(tree) = &request.document().tree {
            for include in &tree.includes {
                for (i, path) in include.paths().iter().enumerate() {
                    let range = path.range();
                    if range.contains(request.params.position) {
                        return Some((range, &include.all_targets[i]));
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use texlab_protocol::RangeExt;

    #[test]
    fn multiple_paths() {
        let hover = test_feature(
            LatexIncludeHoverProvider,
            FeatureSpec {
                files: vec![
                    FeatureSpec::file("foo.tex", "\\include{bar, baz}"),
                    FeatureSpec::file("bar.tex", ""),
                    FeatureSpec::file("baz.tex", ""),
                ],
                main_file: "foo.tex",
                position: Position::new(0, 16),
                ..FeatureSpec::default()
            },
        );

        assert_eq!(
            hover,
            Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::PlainText,
                    value: FeatureSpec::uri("baz.tex")
                        .to_file_path()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                }),
                range: Some(Range::new_simple(0, 14, 0, 17)),
            })
        );
    }

    #[test]
    fn empty() {
        let hover = test_feature(
            LatexIncludeHoverProvider,
            FeatureSpec {
                files: vec![FeatureSpec::file("foo.tex", "")],
                main_file: "foo.tex",
                position: Position::new(0, 0),
                ..FeatureSpec::default()
            },
        );
        assert_eq!(hover, None);
    }
}
