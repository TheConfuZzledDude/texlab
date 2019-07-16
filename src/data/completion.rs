use crate::syntax::SyntaxTree;
use crate::workspace::Document;
use itertools::Itertools;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Database {
    pub components: Vec<Component>,
}

impl Database {
    pub fn find(&self, name: &str) -> Option<&Component> {
        self.components.iter().find(|component| {
            component
                .file_names
                .iter()
                .any(|file_name| file_name == name)
        })
    }

    pub fn kernel(&self) -> &Component {
        self.components
            .iter()
            .find(|component| component.file_names.is_empty())
            .unwrap()
    }

    pub fn related_components(&self, documents: &[Arc<Document>]) -> Vec<&Component> {
        let mut start_components = vec![self.kernel()];
        for document in documents {
            if let SyntaxTree::Latex(tree) = &document.tree {
                tree.components
                    .iter()
                    .flat_map(|file| self.find(file))
                    .for_each(|component| start_components.push(component))
            }
        }

        let mut all_components = Vec::new();
        for component in start_components {
            all_components.push(component);
            component
                .references
                .iter()
                .flat_map(|file| self.find(&file))
                .for_each(|component| all_components.push(component))
        }

        log::info!("Components = {:?}", all_components.len());

        all_components
            .into_iter()
            .unique_by(|component| &component.file_names)
            .collect()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    pub file_names: Vec<String>,
    pub references: Vec<String>,
    pub commands: Vec<Command>,
    pub environments: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub name: String,
    pub image: Option<String>,
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameter(Vec<Argument>);

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Argument {
    pub name: String,
    pub image: Option<String>,
}

const JSON: &str = include_str!("completion.json");

pub static DATABASE: Lazy<Database> = Lazy::new(|| serde_json::from_str(JSON).unwrap());
