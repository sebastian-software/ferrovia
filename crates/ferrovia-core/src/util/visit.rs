use crate::types::{
    XastCdata, XastChild, XastComment, XastDoctype, XastElement, XastInstruction, XastRoot,
    XastText,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisitOutcome {
    Continue,
    Skip,
}

pub trait Visitor {
    fn root_enter(&mut self, _node: &mut XastRoot) -> VisitOutcome {
        VisitOutcome::Continue
    }

    fn root_exit(&mut self, _node: &mut XastRoot) {}

    fn element_enter(&mut self, _node: &mut XastElement) -> VisitOutcome {
        VisitOutcome::Continue
    }

    fn element_exit(&mut self, _node: &mut XastElement) {}

    fn text_enter(&mut self, _node: &mut XastText) -> VisitOutcome {
        VisitOutcome::Continue
    }

    fn text_exit(&mut self, _node: &mut XastText) {}

    fn doctype_enter(&mut self, _node: &mut XastDoctype) -> VisitOutcome {
        VisitOutcome::Continue
    }

    fn doctype_exit(&mut self, _node: &mut XastDoctype) {}

    fn instruction_enter(&mut self, _node: &mut XastInstruction) -> VisitOutcome {
        VisitOutcome::Continue
    }

    fn instruction_exit(&mut self, _node: &mut XastInstruction) {}

    fn comment_enter(&mut self, _node: &mut XastComment) -> VisitOutcome {
        VisitOutcome::Continue
    }

    fn comment_exit(&mut self, _node: &mut XastComment) {}

    fn cdata_enter(&mut self, _node: &mut XastCdata) -> VisitOutcome {
        VisitOutcome::Continue
    }

    fn cdata_exit(&mut self, _node: &mut XastCdata) {}
}

pub fn visit(root: &mut XastRoot, visitor: &mut impl Visitor) {
    if visitor.root_enter(root) == VisitOutcome::Skip {
        return;
    }
    visit_children(&mut root.children, visitor);
    visitor.root_exit(root);
}

#[expect(
    clippy::ptr_arg,
    reason = "The direct-port rewrite keeps mutable child vectors to preserve JS-like splice semantics"
)]
fn visit_children(children: &mut Vec<XastChild>, visitor: &mut impl Visitor) {
    let mut index = 0usize;
    while index < children.len() {
        let original_len = children.len();
        match &mut children[index] {
            XastChild::Element(node) => {
                if visitor.element_enter(node) != VisitOutcome::Skip {
                    visit_children(&mut node.children, visitor);
                }
                visitor.element_exit(node);
            }
            XastChild::Text(node) => {
                visitor.text_enter(node);
                visitor.text_exit(node);
            }
            XastChild::Doctype(node) => {
                visitor.doctype_enter(node);
                visitor.doctype_exit(node);
            }
            XastChild::Instruction(node) => {
                visitor.instruction_enter(node);
                visitor.instruction_exit(node);
            }
            XastChild::Comment(node) => {
                visitor.comment_enter(node);
                visitor.comment_exit(node);
            }
            XastChild::Cdata(node) => {
                visitor.cdata_enter(node);
                visitor.cdata_exit(node);
            }
        }
        if children.len() == original_len {
            index += 1;
        }
    }
}
