use super::combinators::{self, Parameter};
use crate::factory;
use futures_boxed::boxed;
use std::iter;
use texlab_protocol::*;
use texlab_workspace::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct LatexArgumentCompletionProvider;

impl FeatureProvider for LatexArgumentCompletionProvider {
    type Params = CompletionParams;
    type Output = Vec<CompletionItem>;

    #[boxed]
    async fn execute<'a>(&'a self, request: &'a FeatureRequest<Self::Params>) -> Self::Output {
        let mut all_items = Vec::new();
        for component in COMPONENT_DATABASE.related_components(request.related_documents()) {
            for command in &component.commands {
                let name = format!("\\{}", command.name);
                for (i, parameter) in command.parameters.iter().enumerate() {
                    let mut items = combinators::argument(
                        request,
                        iter::once(Parameter::new(&name, i)),
                        |context| {
                            async move {
                                let mut items = Vec::new();
                                for argument in &parameter.0 {
                                    let text_edit =
                                        TextEdit::new(context.range, (&argument.name).into());
                                    let item = factory::argument(
                                        request,
                                        &argument.name,
                                        text_edit,
                                        argument.image.as_ref().map(AsRef::as_ref),
                                    );
                                    items.push(item);
                                }
                                items
                            }
                        },
                    )
                    .await;
                    all_items.append(&mut items);
                }
            }
        }
        all_items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inside_mathbb_empty() {
        let items = test_feature(
            LatexArgumentCompletionProvider,
            FeatureSpec {
                files: vec![FeatureSpec::file(
                    "foo.tex",
                    "\\usepackage{amsfonts}\n\\mathbb{}",
                )],
                main_file: "foo.tex",
                position: Position::new(1, 8),
                ..FeatureSpec::default()
            },
        );
        assert!(!items.is_empty());
        assert_eq!(
            items[0].text_edit.as_ref().map(|edit| edit.range),
            Some(Range::new_simple(1, 8, 1, 8))
        );
    }

    #[test]
    fn inside_mathbb_non_empty() {
        let items = test_feature(
            LatexArgumentCompletionProvider,
            FeatureSpec {
                files: vec![FeatureSpec::file(
                    "foo.tex",
                    "\\usepackage{amsfonts}\n\\mathbb{foo}",
                )],
                main_file: "foo.tex",
                position: Position::new(1, 8),
                ..FeatureSpec::default()
            },
        );
        assert!(!items.is_empty());
        assert_eq!(
            items[0].text_edit.as_ref().map(|edit| edit.range),
            Some(Range::new_simple(1, 8, 1, 11))
        );
    }

    #[test]
    fn outside_mathbb() {
        let items = test_feature(
            LatexArgumentCompletionProvider,
            FeatureSpec {
                files: vec![FeatureSpec::file(
                    "foo.tex",
                    "\\usepackage{amsfonts}\n\\mathbb{}",
                )],
                main_file: "foo.tex",
                position: Position::new(1, 9),
                ..FeatureSpec::default()
            },
        );
        assert!(items.is_empty());
    }
}
