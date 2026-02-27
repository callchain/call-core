pub mod inner_node;
pub mod leaf_node;
pub mod map;

pub use inner_node::{SHAMapInnerNode, SHAMapInnerNodeV2};
pub use leaf_node::{SHAMapItem, SHAMapTreeNode, SHAMapTreeNodeType};
pub use map::{SHAMap, SHAMapType};

use primitives::UInt256;

#[derive(Debug, Clone)]
pub enum SHAMapAbstractNode {
    Inner(SHAMapInnerNode),
    InnerV2(SHAMapInnerNodeV2),
    Leaf(SHAMapTreeNode),
}

impl SHAMapAbstractNode {
    pub fn is_inner(&self) -> bool {
        matches!(self, Self::Inner(_) | Self::InnerV2(_))
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf(_))
    }

    pub fn as_inner(&self) -> Option<&SHAMapInnerNode> {
        match self {
            Self::Inner(inner) => Some(inner),
            _ => None,
        }
    }

    pub fn as_inner_mut(&mut self) -> Option<&mut SHAMapInnerNode> {
        match self {
            Self::Inner(inner) => Some(inner),
            _ => None,
        }
    }

    pub fn as_inner_v2(&self) -> Option<&SHAMapInnerNodeV2> {
        match self {
            Self::InnerV2(inner) => Some(inner),
            _ => None,
        }
    }

    pub fn as_inner_v2_mut(&mut self) -> Option<&mut SHAMapInnerNodeV2> {
        match self {
            Self::InnerV2(inner) => Some(inner),
            _ => None,
        }
    }

    pub fn as_leaf(&self) -> Option<&SHAMapTreeNode> {
        match self {
            Self::Leaf(leaf) => Some(leaf),
            _ => None,
        }
    }

    pub fn as_leaf_mut(&mut self) -> Option<&mut SHAMapTreeNode> {
        match self {
            Self::Leaf(leaf) => Some(leaf),
            _ => None,
        }
    }

    pub fn hash(&self) -> UInt256 {
        match self {
            Self::Inner(inner) => inner.get_node_hash(),
            Self::InnerV2(inner_v2) => inner_v2.get_node_hash(),
            Self::Leaf(leaf) => leaf.hash(),
        }
    }
}
