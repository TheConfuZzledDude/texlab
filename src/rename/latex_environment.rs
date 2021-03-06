use futures_boxed::boxed;
use std::collections::HashMap;
use texlab_protocol::RangeExt;
use texlab_protocol::*;
use texlab_syntax::*;
use texlab_workspace::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct LatexEnvironmentPrepareRenameProvider;

impl FeatureProvider for LatexEnvironmentPrepareRenameProvider {
    type Params = TextDocumentPositionParams;
    type Output = Option<Range>;

    #[boxed]
    async fn execute<'a>(
        &'a self,
        request: &'a FeatureRequest<TextDocumentPositionParams>,
    ) -> Option<Range> {
        let position = request.params.position;
        let environment = find_environment(&request.document().tree, position)?;
        let left_range = environment.left.name().unwrap().range();
        let right_range = environment.right.name().unwrap().range();
        if left_range.contains(position) {
            Some(left_range)
        } else {
            Some(right_range)
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct LatexEnvironmentRenameProvider;

impl FeatureProvider for LatexEnvironmentRenameProvider {
    type Params = RenameParams;
    type Output = Option<WorkspaceEdit>;

    #[boxed]
    async fn execute<'a>(
        &'a self,
        request: &'a FeatureRequest<RenameParams>,
    ) -> Option<WorkspaceEdit> {
        let environment = find_environment(
            &request.document().tree,
            request.params.text_document_position.position,
        )?;
        let edits = vec![
            TextEdit::new(
                environment.left.name().unwrap().range(),
                request.params.new_name.clone(),
            ),
            TextEdit::new(
                environment.right.name().unwrap().range(),
                request.params.new_name.clone(),
            ),
        ];
        let mut changes = HashMap::new();
        changes.insert(request.document().uri.clone().into(), edits);
        Some(WorkspaceEdit::new(changes))
    }
}

fn find_environment(tree: &SyntaxTree, position: Position) -> Option<&LatexEnvironment> {
    if let SyntaxTree::Latex(tree) = &tree {
        for environment in &tree.env.environments {
            if let Some(left_name) = environment.left.name() {
                if let Some(right_name) = environment.right.name() {
                    if left_name.range().contains(position) || right_name.range().contains(position)
                    {
                        return Some(environment);
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use texlab_protocol::{Position, Range};

    #[test]
    fn environment() {
        let edit = test_feature(
            LatexEnvironmentRenameProvider,
            FeatureSpec {
                files: vec![FeatureSpec::file("foo.tex", "\\begin{foo}\n\\end{bar}")],
                main_file: "foo.tex",
                position: Position::new(0, 8),
                new_name: "baz",
                ..FeatureSpec::default()
            },
        );
        let mut changes = HashMap::new();
        changes.insert(
            FeatureSpec::uri("foo.tex"),
            vec![
                TextEdit::new(Range::new_simple(0, 7, 0, 10), "baz".into()),
                TextEdit::new(Range::new_simple(1, 5, 1, 8), "baz".into()),
            ],
        );
        assert_eq!(edit, Some(WorkspaceEdit::new(changes)));
    }

    #[test]
    fn command() {
        let edit = test_feature(
            LatexEnvironmentRenameProvider,
            FeatureSpec {
                files: vec![FeatureSpec::file("foo.tex", "\\begin{foo}\n\\end{bar}")],
                main_file: "foo.tex",
                position: Position::new(0, 5),
                new_name: "baz",
                ..FeatureSpec::default()
            },
        );
        assert_eq!(edit, None);
    }

    #[test]
    fn bibtex() {
        let edit = test_feature(
            LatexEnvironmentRenameProvider,
            FeatureSpec {
                files: vec![FeatureSpec::file("foo.bib", "")],
                main_file: "foo.bib",
                position: Position::new(0, 0),
                new_name: "baz",
                ..FeatureSpec::default()
            },
        );
        assert_eq!(edit, None);
    }
}
