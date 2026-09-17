use super::{AigGraph, LabelKind, NodeId};

impl AigGraph {
    /// Render this graph as Graphviz DOT text.
    ///
    /// The returned string can be written to a `.dot` file and rendered with
    /// Graphviz tools such as `dot`.
    pub fn to_dot(&self) -> String {
        let mut dot = String::new();

        dot.push_str(
            "digraph AIG {\n\
        \trankdir=BT;\n\
        \tordering=out;\n\
        \tnode [fontname=\"Helvetica\"];\n\
        \tedge [fontname=\"Helvetica\"];\n\n\
        \tconst_false [label=\"false\", shape=box];\n\n",
        );

        for (input_index, input_id) in self.inputs.iter().enumerate() {
            let node_name = Self::dot_name(input_id.regular());
            let input_label = Self::input_label(input_index);

            dot.push_str(&format!(
                "\t{} [label=\"{}\", shape=box];\n",
                node_name, input_label
            ));
        }

        dot.push('\n');

        for (latch_index, latch_id) in self.latches.iter().enumerate() {
            let node_name = Self::dot_name(latch_id.regular());

            dot.push_str(&format!(
                "\t{} [label=\"l{}\", shape=box, style=rounded];\n",
                node_name, latch_index
            ));
        }

        dot.push('\n');

        for (index, node) in self.nodes.iter().enumerate() {
            if node.is_and() {
                let node_id: NodeId = index.into();
                let node_name = Self::dot_name(node_id);

                dot.push_str(&format!("\t{} [label=\"AND\", shape=circle];\n", node_name));
            }
        }

        dot.push('\n');

        for (index, node) in self.nodes.iter().enumerate() {
            if node.is_and() {
                let parent_id: NodeId = index.into();
                let parent_name = Self::dot_name(parent_id);

                Self::write_dot_edge(&mut dot, node.left(), &parent_name, "left");
                Self::write_dot_edge(&mut dot, node.right(), &parent_name, "right");
            }
        }

        dot.push('\n');

        for latch_id in &self.latches {
            let latch = &self[*latch_id];
            let latch_name = Self::dot_name(latch_id.regular());

            Self::write_dot_edge(&mut dot, latch.right(), &latch_name, "next");
        }

        dot.push('\n');

        for (kind, index, expr) in self.labels() {
            let prefix = match kind {
                LabelKind::Output => "out",
                LabelKind::BadState => "bad",
                LabelKind::Constraints => "inv",
                LabelKind::Justice => "just",
                LabelKind::Fairness => "fair",
            };
            let name = format!("{prefix}{index}");

            dot.push_str(&format!("\t{name} [label=\"{name}\", shape=box];\n",));

            Self::write_dot_edge(&mut dot, expr, &name, "");
        }

        dot.push_str("}\n");

        dot
    }

    fn write_dot_edge(dot: &mut String, child: NodeId, parent_name: &str, edge_label: &str) {
        let child_name = Self::dot_name(child.regular());

        if edge_label.is_empty() {
            if child.is_inverted() {
                dot.push_str(&format!(
                    "\t{} -> {} [style=dashed];\n",
                    child_name, parent_name
                ));
            } else {
                dot.push_str(&format!("\t{} -> {};\n", child_name, parent_name));
            }
        } else if child.is_inverted() {
            dot.push_str(&format!(
                "\t{} -> {} [label=\"{}\", style=dashed];\n",
                child_name, parent_name, edge_label
            ));
        } else {
            dot.push_str(&format!(
                "\t{} -> {} [label=\"{}\"];\n",
                child_name, parent_name, edge_label
            ));
        }
    }

    fn dot_name(id: NodeId) -> String {
        let regular_id = id.regular();

        if regular_id.is_false() {
            String::from("const_false")
        } else {
            let index =
                usize::try_from(regular_id).expect("NodeId does not correspond to a graph index");
            format!("n{}", index)
        }
    }

    // Represents inputs as letters instead of numbers:
    // 0 -> a
    // 1 -> b
    // ...
    // 25 -> z
    // 26 -> aa
    // 27 -> ab
    fn input_label(index: usize) -> String {
        if index < 26 {
            ((b'a' + index as u8) as char).to_string()
        } else {
            let prefix = Self::input_label((index / 26) - 1);
            let suffix = (b'a' + (index % 26) as u8) as char;

            format!("{}{}", prefix, suffix)
        }
    }
}
