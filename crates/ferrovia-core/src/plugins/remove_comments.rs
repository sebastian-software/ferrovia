use serde_json::Value;

use crate::svgo::tools::preserve_comment;
use crate::types::{XastChild, XastRoot};

/// Apply the `removeComments` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot, params: Option<&Value>) -> crate::error::Result<()> {
    let preserve_patterns = params
        .and_then(|value| value.get("preservePatterns"))
        .and_then(Value::as_array)
        .map_or_else(|| vec!["^!".to_string()], |items| {
            items.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        });
    remove_comments(&mut root.children, &preserve_patterns);
    Ok(())
}

fn remove_comments(children: &mut Vec<XastChild>, preserve_patterns: &[String]) {
    let mut index = 0usize;
    while index < children.len() {
        let mut removed = false;
        match &mut children[index] {
            XastChild::Comment(comment) => {
                if !preserve_comment(comment.value.as_str(), preserve_patterns) {
                    children.remove(index);
                    removed = true;
                }
            }
            XastChild::Element(element) => remove_comments(&mut element.children, preserve_patterns),
            _ => {}
        }
        if !removed {
            index += 1;
        }
    }
}
