use super::combinators;
use crate::factory::{self, LatexComponentId};
use futures_boxed::boxed;
use itertools::Itertools;
use texlab_protocol::*;
use texlab_syntax::*;
use texlab_workspace::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct LatexUserCommandCompletionProvider;

impl FeatureProvider for LatexUserCommandCompletionProvider {
    type Params = CompletionParams;
    type Output = Vec<CompletionItem>;

    #[boxed]
    async fn execute<'a>(&'a self, request: &'a FeatureRequest<Self::Params>) -> Self::Output {
        combinators::command(request, |current_command| {
            async move {
                let mut items = Vec::new();
                for document in request.related_documents() {
                    if let SyntaxTree::Latex(tree) = &document.tree {
                        tree.commands
                            .iter()
                            .filter(|command| command.range() != current_command.range())
                            .map(|command| &command.name.text()[1..])
                            .unique()
                            .map(|command| {
                                let text_edit = TextEdit::new(
                                    current_command.short_name_range(),
                                    command.to_owned(),
                                );
                                factory::command(
                                    request,
                                    command.to_owned(),
                                    None,
                                    None,
                                    text_edit,
                                    &LatexComponentId::User,
                                )
                            })
                            .for_each(|item| items.push(item));
                    }
                }
                items
            }
        })
        .await
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct LatexUserEnvironmentCompletionProvider;

impl FeatureProvider for LatexUserEnvironmentCompletionProvider {
    type Params = CompletionParams;
    type Output = Vec<CompletionItem>;

    #[boxed]
    async fn execute<'a>(&'a self, request: &'a FeatureRequest<Self::Params>) -> Self::Output {
        combinators::environment(request, |context| {
            async move {
                let mut items = Vec::new();
                for document in request.related_documents() {
                    if let SyntaxTree::Latex(tree) = &document.tree {
                        for environment in &tree.env.environments {
                            if environment.left.command == context.command
                                || environment.right.command == context.command
                            {
                                continue;
                            }

                            if let Some(item) =
                                Self::make_item(request, &environment.left, context.range)
                            {
                                items.push(item);
                            }

                            if let Some(item) =
                                Self::make_item(request, &environment.right, context.range)
                            {
                                items.push(item);
                            }
                        }
                    }
                }
                items
            }
        })
        .await
    }
}

impl LatexUserEnvironmentCompletionProvider {
    fn make_item(
        request: &FeatureRequest<CompletionParams>,
        delimiter: &LatexEnvironmentDelimiter,
        name_range: Range,
    ) -> Option<CompletionItem> {
        if let Some(name) = delimiter.name() {
            let text = name.text().to_owned();
            let text_edit = TextEdit::new(name_range, text.clone());
            let item = factory::environment(request, text, text_edit, &LatexComponentId::User);
            return Some(item);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command() {
        let items = test_feature(
            LatexUserCommandCompletionProvider,
            FeatureSpec {
                files: vec![
                    FeatureSpec::file("foo.tex", "\\include{bar.tex}\n\\foo"),
                    FeatureSpec::file("bar.tex", "\\bar"),
                    FeatureSpec::file("baz.tex", "\\baz"),
                ],
                main_file: "foo.tex",
                position: Position::new(1, 2),
                ..FeatureSpec::default()
            },
        );
        let labels: Vec<&str> = items.iter().map(|item| item.label.as_ref()).collect();
        assert_eq!(labels, vec!["include", "bar"]);
    }

    #[test]
    fn environment() {
        let items = test_feature(
            LatexUserEnvironmentCompletionProvider,
            FeatureSpec {
                files: vec![
                    FeatureSpec::file("foo.tex", "\\include{bar.tex}\n\\begin{foo}"),
                    FeatureSpec::file("bar.tex", "\\begin{bar}\\end{bar}"),
                    FeatureSpec::file("baz.tex", "\\begin{baz}\\end{baz}"),
                ],
                main_file: "foo.tex",
                position: Position::new(1, 9),
                ..FeatureSpec::default()
            },
        );
        let labels: Vec<&str> = items
            .iter()
            .map(|item| item.label.as_ref())
            .unique()
            .collect();
        assert_eq!(labels, vec!["bar"]);
    }
}
