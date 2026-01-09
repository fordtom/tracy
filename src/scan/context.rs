//! Context extraction for requirement markers.
//!
//! This module extracts context around comments by walking up and down
//! to find adjacent comments and the surrounding code.

use ast_grep_core::{Doc, Node};
use serde::Serialize;
use std::collections::HashMap;

/// Represents code context found near a comment.
#[derive(Debug, Clone, Serialize)]
pub struct CodeContext {
    /// The AST node kind (e.g., "function_item", "let_declaration")
    pub kind: String,
    /// Extracted name if applicable (function name, variable name, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The text content (first line for multiline nodes)
    pub text: String,
    /// Line number (1-indexed)
    pub line: usize,
}

/// Represents a scope item in the hierarchy chain.
#[derive(Debug, Clone, Serialize)]
pub struct ScopeItem {
    /// The AST node kind (e.g., "function_item", "impl_item", "mod_item")
    pub kind: String,
    /// Extracted name if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The line where this scope starts (1-indexed)
    pub line: usize,
}

/// Context extracted for a comment block.
#[derive(Debug, Clone)]
pub struct BlockContext {
    /// Code found above the comment block
    pub above: Option<CodeContext>,
    /// Code found below the comment block
    pub below: Option<CodeContext>,
    /// Code on the same line (for inline comments)
    pub inline: Option<CodeContext>,
}

/// Node kinds that represent "interesting" code constructs.
const INTERESTING_KINDS: &[&str] = &[
    // Functions
    "function_item",
    "function_definition",
    "function_declaration",
    "method_definition",
    "method_declaration",
    "arrow_function",
    "function_expression",
    "lambda_expression",
    // Variables/Constants
    "let_declaration",
    "const_declaration",
    "const_item",
    "static_item",
    "variable_declaration",
    "variable_declarator",
    "lexical_declaration",
    "assignment_expression",
    "assignment_statement",
    "short_var_declaration",
    "var_declaration",
    // Types/Classes/Structs
    "struct_item",
    "struct_definition",
    "class_declaration",
    "class_definition",
    "interface_declaration",
    "type_alias",
    "type_declaration",
    "type_item",
    "enum_item",
    "enum_declaration",
    "trait_item",
    "trait_definition",
    // Impl/Methods
    "impl_item",
    // Calls
    "call_expression",
    "method_call_expression",
    // Macros
    "macro_invocation",
    "macro_definition",
    // Statements
    "expression_statement",
    "return_statement",
    // Module/Package
    "mod_item",
    "module_declaration",
    "package_clause",
    "namespace_definition",
    // Use/Import
    "use_declaration",
    "import_statement",
    "import_declaration",
    // Python specific
    "assignment",
    "decorated_definition",
    // Java specific
    "field_declaration",
    "local_variable_declaration",
    // Python docstrings (string as first statement)
    "expression_statement",
];

/// Node kinds that represent scope containers.
const SCOPE_KINDS: &[&str] = &[
    "function_item",
    "function_definition",
    "function_declaration",
    "method_definition",
    "method_declaration",
    "arrow_function",
    "lambda_expression",
    "closure_expression",
    "struct_item",
    "class_declaration",
    "class_definition",
    "interface_declaration",
    "enum_item",
    "enum_declaration",
    "trait_item",
    "impl_item",
    "mod_item",
    "module_declaration",
    "namespace_definition",
    "decorated_definition",
];

/// Extract context for a comment at the given line.
///
/// This function:
/// 1. Finds all adjacent comments (the "block")
/// 2. Looks for code above the block
/// 3. Looks for code below the block
/// 4. Looks for code on the same line (inline)
pub fn extract_block_context<D: Doc>(
    root: &Node<D>,
    comment_line: usize,
    source_lines: &[&str],
) -> BlockContext {
    // Build a map of line -> nodes on that line (excluding comments)
    let mut line_to_nodes: HashMap<usize, Vec<NodeInfo>> = HashMap::new();
    // Also track which lines have comments
    let mut comment_lines: HashMap<usize, Vec<String>> = HashMap::new();

    for node in root.dfs() {
        let kind = node.kind();
        let kind_str: &str = &kind;
        let start_line = node.start_pos().line();

        if kind_str.contains("comment") {
            let text = node.text().to_string();
            comment_lines.entry(start_line).or_default().push(text);
        } else if is_interesting_kind(kind_str) {
            let name = extract_name(&node, kind_str);
            let text = first_line(node.text());
            line_to_nodes.entry(start_line).or_default().push(NodeInfo {
                kind: kind_str.to_string(),
                name,
                text,
                priority: kind_priority(kind_str),
            });
        }
    }

    // Find the comment block boundaries by walking up and down
    let (block_start, block_end) = find_comment_block_bounds(comment_line, &comment_lines, source_lines);

    // Look for code ABOVE the block (first line with non-comment content)
    let above = find_context_above(block_start, &line_to_nodes, source_lines);

    // Look for code BELOW the block (first line with non-comment content)
    let below = find_context_below(block_end, &line_to_nodes, source_lines);

    // Look for code on the same line as the comment (inline)
    let inline = find_inline_context(comment_line, &line_to_nodes);

    BlockContext {
        above,
        below,
        inline,
    }
}

#[derive(Debug)]
struct NodeInfo {
    kind: String,
    name: Option<String>,
    text: String,
    priority: i32,
}

/// Find the boundaries of a comment block by walking up and down.
fn find_comment_block_bounds(
    start_line: usize,
    comment_lines: &HashMap<usize, Vec<String>>,
    source_lines: &[&str],
) -> (usize, usize) {
    let mut block_start = start_line;
    let mut block_end = start_line;

    // Walk up to find the start of the comment block
    if start_line > 0 {
        let mut line = start_line - 1;
        loop {
            if line < source_lines.len() && is_comment_only_line(line, comment_lines, source_lines) {
                block_start = line;
            } else {
                break;
            }
            if line == 0 {
                break;
            }
            line -= 1;
        }
    }

    // Walk down to find the end of the comment block
    let mut line = start_line + 1;
    while line < source_lines.len() {
        if is_comment_only_line(line, comment_lines, source_lines) {
            block_end = line;
            line += 1;
        } else {
            break;
        }
    }

    (block_start, block_end)
}

/// Check if a line contains only comment(s) and whitespace.
fn is_comment_only_line(
    line: usize,
    comment_lines: &HashMap<usize, Vec<String>>,
    source_lines: &[&str],
) -> bool {
    // If we have comments on this line
    if comment_lines.contains_key(&line) {
        // Check if the source line is all whitespace except for the comment
        if let Some(src) = source_lines.get(line) {
            let trimmed = src.trim();
            // Common comment starters
            trimmed.starts_with("//")
                || trimmed.starts_with("#")
                || trimmed.starts_with("/*")
                || trimmed.starts_with("*")
                || trimmed.starts_with("'''")
                || trimmed.starts_with("\"\"\"")
                || trimmed.is_empty()
        } else {
            false
        }
    } else {
        false
    }
}

/// Find context on lines above the comment block.
fn find_context_above(
    block_start: usize,
    line_to_nodes: &HashMap<usize, Vec<NodeInfo>>,
    source_lines: &[&str],
) -> Option<CodeContext> {
    if block_start == 0 {
        return None;
    }

    // Walk up to find the first line with non-comment, non-empty content
    let mut line = block_start - 1;
    loop {
        if let Some(nodes) = line_to_nodes.get(&line) {
            // Pick the highest priority node on this line
            if let Some(best) = nodes.iter().max_by_key(|n| n.priority) {
                return Some(CodeContext {
                    kind: best.kind.clone(),
                    name: best.name.clone(),
                    text: best.text.clone(),
                    line: line + 1, // 1-indexed
                });
            }
        }

        // Check if line has any content (even if not "interesting")
        if let Some(src) = source_lines.get(line) {
            let trimmed = src.trim();
            if !trimmed.is_empty()
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("#")
                && !trimmed.starts_with("/*")
                && !trimmed.starts_with("*")
            {
                // There's code here but we didn't capture it - return generic context
                return Some(CodeContext {
                    kind: "unknown".to_string(),
                    name: None,
                    text: trimmed.to_string(),
                    line: line + 1,
                });
            }
        }

        if line == 0 {
            break;
        }
        line -= 1;
    }

    None
}

/// Find context on lines below the comment block.
fn find_context_below(
    block_end: usize,
    line_to_nodes: &HashMap<usize, Vec<NodeInfo>>,
    source_lines: &[&str],
) -> Option<CodeContext> {
    // Walk down to find the first line with non-comment, non-empty content
    let mut line = block_end + 1;
    while line < source_lines.len() {
        if let Some(nodes) = line_to_nodes.get(&line) {
            if let Some(best) = nodes.iter().max_by_key(|n| n.priority) {
                return Some(CodeContext {
                    kind: best.kind.clone(),
                    name: best.name.clone(),
                    text: best.text.clone(),
                    line: line + 1, // 1-indexed
                });
            }
        }

        // Check if line has any content
        if let Some(src) = source_lines.get(line) {
            let trimmed = src.trim();
            if !trimmed.is_empty()
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("#")
                && !trimmed.starts_with("/*")
                && !trimmed.starts_with("*")
                && !trimmed.starts_with("'''")
                && !trimmed.starts_with("\"\"\"")
            {
                return Some(CodeContext {
                    kind: "unknown".to_string(),
                    name: None,
                    text: trimmed.to_string(),
                    line: line + 1,
                });
            }
        }

        line += 1;
    }

    None
}

/// Find context on the same line as the comment (inline).
fn find_inline_context(
    comment_line: usize,
    line_to_nodes: &HashMap<usize, Vec<NodeInfo>>,
) -> Option<CodeContext> {
    if let Some(nodes) = line_to_nodes.get(&comment_line) {
        if let Some(best) = nodes.iter().max_by_key(|n| n.priority) {
            return Some(CodeContext {
                kind: best.kind.clone(),
                name: best.name.clone(),
                text: best.text.clone(),
                line: comment_line + 1, // 1-indexed
            });
        }
    }
    None
}

/// Extract the scope hierarchy by finding all containers that encompass the target line.
pub fn extract_hierarchy<D: Doc>(root: &Node<D>, target_line: usize) -> Vec<ScopeItem> {
    let mut scopes = Vec::new();

    for node in root.dfs() {
        let kind = node.kind();
        let kind_str: &str = &kind;

        if !is_scope_kind(kind_str) {
            continue;
        }

        let start_line = node.start_pos().line();
        let end_line = node.end_pos().line();

        if start_line <= target_line && target_line <= end_line {
            let name = extract_name(&node, kind_str);

            scopes.push(ScopeItem {
                kind: kind_str.to_string(),
                name,
                line: start_line + 1,
            });
        }
    }

    // Sort by line number descending (innermost first)
    scopes.sort_by(|a, b| b.line.cmp(&a.line));
    scopes
}

fn is_interesting_kind(kind: &str) -> bool {
    INTERESTING_KINDS.contains(&kind)
}

fn is_scope_kind(kind: &str) -> bool {
    SCOPE_KINDS.contains(&kind)
}

fn kind_priority(kind: &str) -> i32 {
    match kind {
        "function_item" | "function_definition" | "function_declaration" | "method_definition"
        | "method_declaration" => 100,

        "struct_item" | "struct_definition" | "class_declaration" | "class_definition"
        | "interface_declaration" | "enum_item" | "enum_declaration" | "trait_item"
        | "type_alias" | "type_item" => 90,

        "impl_item" => 85,

        "let_declaration" | "const_declaration" | "const_item" | "static_item"
        | "variable_declaration" | "lexical_declaration" | "short_var_declaration"
        | "var_declaration" | "field_declaration" | "local_variable_declaration" => 80,

        "assignment_expression" | "assignment_statement" | "assignment" => 70,

        "call_expression" | "method_call_expression" | "macro_invocation" => 60,

        "return_statement" => 50,

        "use_declaration" | "import_statement" | "import_declaration" => 45,

        "expression_statement" => 30,

        "decorated_definition" => 25,

        _ => 10,
    }
}

/// Extract a name from a node based on its kind.
fn extract_name<D: Doc>(node: &Node<D>, kind: &str) -> Option<String> {
    match kind {
        // Rust
        "function_item" | "struct_item" | "enum_item" | "trait_item" | "mod_item"
        | "type_alias" | "type_item" | "const_item" | "static_item" | "macro_definition" => {
            node.field("name").map(|n| n.text().to_string())
        }

        "impl_item" => {
            node.field("type")
                .or_else(|| node.field("trait"))
                .map(|n| first_line(n.text()))
        }

        "let_declaration" => node.field("pattern").map(|n| n.text().to_string()),

        "use_declaration" => node.field("argument").map(|n| first_line(n.text())),

        // JavaScript/TypeScript
        "function_declaration" | "class_declaration" | "interface_declaration"
        | "method_definition" => node.field("name").map(|n| n.text().to_string()),

        "variable_declaration" | "lexical_declaration" => {
            for child in node.children() {
                let child_kind: &str = &child.kind();
                if child_kind == "variable_declarator" {
                    if let Some(name) = child.field("name") {
                        return Some(name.text().to_string());
                    }
                }
            }
            None
        }

        "variable_declarator" => node.field("name").map(|n| n.text().to_string()),

        // Python
        "function_definition" | "class_definition" => {
            node.field("name").map(|n| n.text().to_string())
        }

        "assignment" => node.field("left").map(|n| first_line(n.text())),

        "decorated_definition" => {
            node.field("definition")
                .and_then(|def| def.field("name").map(|n| n.text().to_string()))
        }

        // Go
        "type_declaration" => {
            for child in node.children() {
                let child_kind: &str = &child.kind();
                if child_kind == "type_spec" {
                    if let Some(name) = child.field("name") {
                        return Some(name.text().to_string());
                    }
                }
            }
            None
        }

        "short_var_declaration" | "var_declaration" => {
            node.field("left").map(|n| first_line(n.text()))
        }

        // Java
        "method_declaration" => node.field("name").map(|n| n.text().to_string()),

        "field_declaration" | "local_variable_declaration" => {
            node.field("declarator")
                .and_then(|d| d.field("name").map(|n| n.text().to_string()))
        }

        // Call expressions
        "call_expression" => node
            .field("function")
            .or_else(|| node.field("callee"))
            .map(|n| first_line(n.text())),

        "method_call_expression" => node.field("name").map(|n| n.text().to_string()),

        "macro_invocation" => node.field("macro").map(|n| n.text().to_string()),

        "import_statement" | "import_declaration" => node
            .field("source")
            .or_else(|| node.field("module_name"))
            .map(|n| n.text().to_string()),

        _ => None,
    }
}

fn first_line(s: impl AsRef<str>) -> String {
    s.as_ref().lines().next().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kind_priority() {
        assert!(kind_priority("function_item") > kind_priority("let_declaration"));
        assert!(kind_priority("let_declaration") > kind_priority("call_expression"));
    }

    #[test]
    fn test_is_interesting_kind() {
        assert!(is_interesting_kind("function_item"));
        assert!(is_interesting_kind("let_declaration"));
        assert!(!is_interesting_kind("source_file"));
    }

    #[test]
    fn test_is_scope_kind() {
        assert!(is_scope_kind("function_item"));
        assert!(is_scope_kind("impl_item"));
        assert!(!is_scope_kind("let_declaration"));
    }
}
