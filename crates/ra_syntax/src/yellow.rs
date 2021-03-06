mod builder;
pub mod syntax_error;
mod syntax_text;

use self::syntax_text::SyntaxText;
use crate::{SmolStr, SyntaxKind, TextRange};
use rowan::{Types, TransparentNewType};
use std::fmt;

pub(crate) use self::builder::GreenBuilder;
pub use self::syntax_error::{SyntaxError, SyntaxErrorKind, Location};
pub use rowan::WalkEvent;

#[derive(Debug, Clone, Copy)]
pub enum RaTypes {}
impl Types for RaTypes {
    type Kind = SyntaxKind;
    type RootData = Vec<SyntaxError>;
}

pub type GreenNode = rowan::GreenNode<RaTypes>;

#[derive(PartialEq, Eq, Hash)]
pub struct TreeArc<T: TransparentNewType<Repr = rowan::SyntaxNode<RaTypes>>>(
    pub(crate) rowan::TreeArc<RaTypes, T>,
);

impl<T> TreeArc<T>
where
    T: TransparentNewType<Repr = rowan::SyntaxNode<RaTypes>>,
{
    pub(crate) fn cast<U>(this: TreeArc<T>) -> TreeArc<U>
    where
        U: TransparentNewType<Repr = rowan::SyntaxNode<RaTypes>>,
    {
        TreeArc(rowan::TreeArc::cast(this.0))
    }
}

impl<T> std::ops::Deref for TreeArc<T>
where
    T: TransparentNewType<Repr = rowan::SyntaxNode<RaTypes>>,
{
    type Target = T;
    fn deref(&self) -> &T {
        self.0.deref()
    }
}

impl<T> PartialEq<T> for TreeArc<T>
where
    T: TransparentNewType<Repr = rowan::SyntaxNode<RaTypes>>,
    T: PartialEq<T>,
{
    fn eq(&self, other: &T) -> bool {
        let t: &T = self;
        t == other
    }
}

impl<T> Clone for TreeArc<T>
where
    T: TransparentNewType<Repr = rowan::SyntaxNode<RaTypes>>,
{
    fn clone(&self) -> TreeArc<T> {
        TreeArc(self.0.clone())
    }
}

impl<T> fmt::Debug for TreeArc<T>
where
    T: TransparentNewType<Repr = rowan::SyntaxNode<RaTypes>>,
    T: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0, fmt)
    }
}

#[derive(PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SyntaxNode(pub(crate) rowan::SyntaxNode<RaTypes>);
unsafe impl TransparentNewType for SyntaxNode {
    type Repr = rowan::SyntaxNode<RaTypes>;
}

impl SyntaxNode {
    pub(crate) fn new(green: GreenNode, errors: Vec<SyntaxError>) -> TreeArc<SyntaxNode> {
        let ptr = TreeArc(rowan::SyntaxNode::new(green, errors));
        TreeArc::cast(ptr)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Next,
    Prev,
}

impl SyntaxNode {
    pub fn leaf_text(&self) -> Option<&SmolStr> {
        self.0.leaf_text()
    }
    pub fn ancestors(&self) -> impl Iterator<Item = &SyntaxNode> {
        crate::algo::generate(Some(self), |&node| node.parent())
    }
    pub fn descendants(&self) -> impl Iterator<Item = &SyntaxNode> {
        self.preorder().filter_map(|event| match event {
            WalkEvent::Enter(node) => Some(node),
            WalkEvent::Leave(_) => None,
        })
    }
    pub fn siblings(&self, direction: Direction) -> impl Iterator<Item = &SyntaxNode> {
        crate::algo::generate(Some(self), move |&node| match direction {
            Direction::Next => node.next_sibling(),
            Direction::Prev => node.prev_sibling(),
        })
    }
    pub fn preorder(&self) -> impl Iterator<Item = WalkEvent<&SyntaxNode>> {
        self.0.preorder().map(|event| match event {
            WalkEvent::Enter(n) => WalkEvent::Enter(SyntaxNode::from_repr(n)),
            WalkEvent::Leave(n) => WalkEvent::Leave(SyntaxNode::from_repr(n)),
        })
    }
}

impl SyntaxNode {
    pub(crate) fn root_data(&self) -> &Vec<SyntaxError> {
        self.0.root_data()
    }

    pub(crate) fn replace_with(&self, replacement: GreenNode) -> GreenNode {
        self.0.replace_self(replacement)
    }

    pub fn to_owned(&self) -> TreeArc<SyntaxNode> {
        let ptr = TreeArc(self.0.to_owned());
        TreeArc::cast(ptr)
    }

    pub fn kind(&self) -> SyntaxKind {
        self.0.kind()
    }

    pub fn range(&self) -> TextRange {
        self.0.range()
    }

    pub fn text(&self) -> SyntaxText {
        SyntaxText::new(self)
    }

    pub fn is_leaf(&self) -> bool {
        self.0.is_leaf()
    }

    pub fn parent(&self) -> Option<&SyntaxNode> {
        self.0.parent().map(SyntaxNode::from_repr)
    }

    pub fn first_child(&self) -> Option<&SyntaxNode> {
        self.0.first_child().map(SyntaxNode::from_repr)
    }

    pub fn last_child(&self) -> Option<&SyntaxNode> {
        self.0.last_child().map(SyntaxNode::from_repr)
    }

    pub fn next_sibling(&self) -> Option<&SyntaxNode> {
        self.0.next_sibling().map(SyntaxNode::from_repr)
    }

    pub fn prev_sibling(&self) -> Option<&SyntaxNode> {
        self.0.prev_sibling().map(SyntaxNode::from_repr)
    }

    pub fn children(&self) -> SyntaxNodeChildren {
        SyntaxNodeChildren(self.0.children())
    }
}

impl fmt::Debug for SyntaxNode {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{:?}@{:?}", self.kind(), self.range())?;
        if has_short_text(self.kind()) {
            write!(fmt, " \"{}\"", self.text())?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct SyntaxNodeChildren<'a>(rowan::SyntaxNodeChildren<'a, RaTypes>);

impl<'a> Iterator for SyntaxNodeChildren<'a> {
    type Item = &'a SyntaxNode;

    fn next(&mut self) -> Option<&'a SyntaxNode> {
        self.0.next().map(SyntaxNode::from_repr)
    }
}

fn has_short_text(kind: SyntaxKind) -> bool {
    use crate::SyntaxKind::*;
    match kind {
        IDENT | LIFETIME | INT_NUMBER | FLOAT_NUMBER => true,
        _ => false,
    }
}
