use crate::factory::{self, LatexComponentId};
use futures_boxed::boxed;
use texlab_protocol::*;
use texlab_syntax::*;
use texlab_workspace::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct BibtexCommandCompletionProvider;

impl FeatureProvider for BibtexCommandCompletionProvider {
    type Params = CompletionParams;
    type Output = Vec<CompletionItem>;

    #[boxed]
    async fn execute<'a>(&'a self, request: &'a FeatureRequest<Self::Params>) -> Self::Output {
        let mut items = Vec::new();
        if let SyntaxTree::Bibtex(tree) = &request.document().tree {
            let position = request.params.text_document_position.position;
            if let Some(BibtexNode::Command(command)) = tree.find(position).last() {
                if command.token.range().contains(position)
                    && command.token.start().character != position.character
                {
                    let mut range = command.range();
                    range.start.character += 1;

                    let component = LatexComponentId::kernel();
                    for command in &COMPONENT_DATABASE.kernel().commands {
                        let text_edit = TextEdit::new(range, (&command.name).into());
                        let item = factory::command(
                            request,
                            (&command.name).into(),
                            command.image.as_ref().map(AsRef::as_ref),
                            command.glyph.as_ref().map(AsRef::as_ref),
                            text_edit,
                            &component,
                        );
                        items.push(item);
                    }
                }
            }
        }
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inside_command() {
        let items = test_feature(
            BibtexCommandCompletionProvider,
            FeatureSpec {
                files: vec![FeatureSpec::file("foo.bib", "@article{foo, bar=\n\\}")],
                main_file: "foo.bib",
                position: Position::new(1, 1),
                ..FeatureSpec::default()
            },
        );
        assert!(!items.is_empty());
        assert_eq!(
            items[0].text_edit.as_ref().map(|edit| edit.range),
            Some(Range::new_simple(1, 1, 1, 2))
        );
    }

    #[test]
    fn start_of_command() {
        let items = test_feature(
            BibtexCommandCompletionProvider,
            FeatureSpec {
                files: vec![FeatureSpec::file("foo.bib", "@article{foo, bar=\n\\}")],
                main_file: "foo.bib",
                position: Position::new(1, 0),
                ..FeatureSpec::default()
            },
        );
        assert!(items.is_empty());
    }

    #[test]
    fn inside_text() {
        let items = test_feature(
            BibtexCommandCompletionProvider,
            FeatureSpec {
                files: vec![FeatureSpec::file("foo.bib", "@article{foo, bar=\n}")],
                main_file: "foo.bib",
                position: Position::new(1, 0),
                ..FeatureSpec::default()
            },
        );
        assert!(items.is_empty());
    }

    #[test]
    fn latex() {
        let items = test_feature(
            BibtexCommandCompletionProvider,
            FeatureSpec {
                files: vec![FeatureSpec::file("foo.tex", "\\")],
                main_file: "foo.tex",
                position: Position::new(0, 1),
                ..FeatureSpec::default()
            },
        );
        assert!(items.is_empty());
    }
}
