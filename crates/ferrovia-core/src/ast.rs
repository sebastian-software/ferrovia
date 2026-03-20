/// Stable node identifier into the arena.
pub type NodeId = usize;

/// Arena-backed SVG document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Document {
    pub nodes: Vec<Node>,
}

impl Document {
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: vec![Node::document()],
        }
    }

    #[must_use]
    pub const fn root_id(&self) -> NodeId {
        0
    }

    pub fn append_child(&mut self, parent: NodeId, kind: NodeKind) -> NodeId {
        let id = self.nodes.len();
        let mut node = Node::new(kind);
        node.parent = Some(parent);

        let last_child = self.nodes[parent].last_child;
        if let Some(last_child) = last_child {
            self.nodes[last_child].next_sibling = Some(id);
        } else {
            self.nodes[parent].first_child = Some(id);
        }
        self.nodes[parent].last_child = Some(id);
        self.nodes.push(node);
        id
    }

    #[must_use]
    pub fn children(&self, parent: NodeId) -> Children<'_> {
        Children {
            doc: self,
            next: self.nodes[parent].first_child,
        }
    }

    pub fn reorder_children(&mut self, parent: NodeId, ordered_children: &[NodeId]) {
        if ordered_children.is_empty() {
            self.nodes[parent].first_child = None;
            self.nodes[parent].last_child = None;
            return;
        }

        self.nodes[parent].first_child = Some(ordered_children[0]);
        self.nodes[parent].last_child = ordered_children.last().copied();

        for (index, child) in ordered_children.iter().copied().enumerate() {
            self.nodes[child].parent = Some(parent);
            self.nodes[child].next_sibling = ordered_children.get(index + 1).copied();
        }
    }

    #[must_use]
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }

    pub fn node_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id]
    }
}

/// Child iterator over arena links.
pub struct Children<'a> {
    doc: &'a Document,
    next: Option<NodeId>,
}

impl Iterator for Children<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next?;
        self.next = self.doc.nodes[current].next_sibling;
        Some(current)
    }
}

/// Arena node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub kind: NodeKind,
    pub parent: Option<NodeId>,
    pub first_child: Option<NodeId>,
    pub last_child: Option<NodeId>,
    pub next_sibling: Option<NodeId>,
}

impl Node {
    const fn document() -> Self {
        Self::new(NodeKind::Document)
    }

    const fn new(kind: NodeKind) -> Self {
        Self {
            kind,
            parent: None,
            first_child: None,
            last_child: None,
            next_sibling: None,
        }
    }
}

/// Node payloads supported by the parser/serializer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Document,
    XmlDecl(XmlDecl),
    Doctype(String),
    Comment(String),
    Text(String),
    Cdata(String),
    Element(Element),
}

impl NodeKind {
    #[must_use]
    pub fn element_attributes(&self) -> Vec<Attribute> {
        match self {
            Self::Element(element) => element.attributes.clone(),
            _ => Vec::new(),
        }
    }
}

/// XML declaration state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlDecl {
    pub attributes: Vec<Attribute>,
}

/// SVG/XML element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub self_closing: bool,
}

/// Attribute preserving quote style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub value: String,
    pub quote: QuoteStyle,
}

/// Quote style used in serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteStyle {
    Double,
    Single,
}
