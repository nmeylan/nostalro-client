use egui_ltreeview::{NodeBuilder, TreeViewBuilder};

#[derive(Clone)]
pub struct TreeNode {
    pub id: u32,
    pub name: String,
    pub full_path: String,
    pub is_dir: bool,
    pub children: Vec<TreeNode>,
    pub file_index: Option<usize>,
}

pub fn build_tree(file_names: &[(usize, &str)]) -> Vec<TreeNode> {
    let mut root_children: Vec<TreeNode> = Vec::new();
    let mut next_id: u32 = 1;

    for &(file_idx, path) in file_names {
        let parts: Vec<&str> = path.split('/').collect();
        insert_path(&mut root_children, &parts, 0, file_idx, &mut next_id);
    }

    sort_tree(&mut root_children);
    root_children
}

fn insert_path(
    children: &mut Vec<TreeNode>,
    parts: &[&str],
    depth: usize,
    file_index: usize,
    next_id: &mut u32,
) {
    if depth >= parts.len() {
        return;
    }

    let name = parts[depth];
    let is_leaf = depth == parts.len() - 1;

    let existing = children
        .iter()
        .position(|c| c.name == name && c.is_dir == !is_leaf);

    if is_leaf {
        let full_path = parts.join("/");
        let id = *next_id;
        *next_id += 1;
        children.push(TreeNode {
            id,
            name: name.to_string(),
            full_path,
            is_dir: false,
            children: Vec::new(),
            file_index: Some(file_index),
        });
    } else if let Some(pos) = existing {
        insert_path(
            &mut children[pos].children,
            parts,
            depth + 1,
            file_index,
            next_id,
        );
    } else {
        let full_path = parts[..=depth].join("/") + "/";
        let id = *next_id;
        *next_id += 1;
        let mut node = TreeNode {
            id,
            name: name.to_string(),
            full_path,
            is_dir: true,
            children: Vec::new(),
            file_index: None,
        };
        insert_path(&mut node.children, parts, depth + 1, file_index, next_id);
        children.push(node);
    }
}

fn sort_tree(children: &mut [TreeNode]) {
    children.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
    for child in children.iter_mut() {
        if child.is_dir {
            sort_tree(&mut child.children);
        }
    }
}

pub fn add_tree_node(node: &TreeNode, builder: &mut TreeViewBuilder<u32>) {
    if node.is_dir {
        let name = node.name.clone();
        builder.node(
            NodeBuilder::dir(node.id)
                .default_open(false)
                .label_ui(|ui| {
                    ui.add(eframe::egui::Label::new(&name).selectable(false));
                }),
        );
        for child in &node.children {
            add_tree_node(child, builder);
        }
        builder.close_dir();
    } else {
        builder.leaf(node.id, &node.name);
    }
}

pub fn find_node_by_id(nodes: &[TreeNode], id: u32) -> Option<&TreeNode> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let Some(found) = find_node_by_id(&node.children, id) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_tree_from_flat_paths() {
        let files = vec![
            (0, "data/texture/a.bmp"),
            (1, "data/texture/b.bmp"),
            (2, "data/model/c.rsm"),
            (3, "readme.txt"),
        ];
        let tree = build_tree(&files);

        assert_eq!(tree.len(), 2);
        // dirs first
        assert!(tree[0].is_dir);
        assert_eq!(tree[0].name, "data");
        // then files
        assert!(!tree[1].is_dir);
        assert_eq!(tree[1].name, "readme.txt");

        let data = &tree[0];
        assert_eq!(data.children.len(), 2);
        assert_eq!(data.children[0].name, "model");
        assert_eq!(data.children[1].name, "texture");

        let texture = &data.children[1];
        assert_eq!(texture.children.len(), 2);
        assert!(!texture.children[0].is_dir);
    }

    #[test]
    fn build_tree_empty_input() {
        let files: Vec<(usize, &str)> = vec![];
        let tree = build_tree(&files);
        assert!(tree.is_empty());
    }

    #[test]
    fn find_node_returns_correct_node() {
        let files = vec![(0, "data/a.txt"), (1, "data/b.txt")];
        let tree = build_tree(&files);
        let data_node = &tree[0];

        let found = find_node_by_id(&tree, data_node.children[0].id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "a.txt");
    }
}
