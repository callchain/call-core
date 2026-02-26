use crate::{SHAMapAbstractNode, SHAMapInnerNode, SHAMapInnerNodeV2};
use crate::leaf_node::{SHAMapItem, SHAMapTreeNode, SHAMapTreeNodeType};
use crypto::sha512_half;
use primitives::UInt256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SHAMapType {
    Free = 0,
    Transaction = 1,
    State = 2,
}

pub struct SHAMap {
    map_type: SHAMapType,
    root: Option<Box<SHAMapAbstractNode>>,
    seq: u32,
}

impl SHAMap {
    pub fn new(map_type: SHAMapType) -> Self {
        Self {
            map_type,
            root: None,
            seq: 1,
        }
    }

    pub fn with_root(root: SHAMapAbstractNode, seq: u32) -> Self {
        Self {
            map_type: SHAMapType::State,
            root: Some(Box::new(root)),
            seq,
        }
    }

    pub fn get_type(&self) -> SHAMapType {
        self.map_type
    }

    fn get_branch_index(key: &UInt256, depth: usize) -> usize {
        let byte_index = depth / 2;
        let nibble_index = depth % 2;
        let byte = key.as_bytes()[byte_index];
        if nibble_index == 0 {
            (byte >> 4) as usize
        } else {
            (byte & 0x0f) as usize
        }
    }

    pub fn add_item(&mut self, key: UInt256, item: SHAMapItem) -> bool {
        let leaf = SHAMapTreeNode::new(SHAMapTreeNodeType::AccountState, item);
        self.add_node(key, SHAMapAbstractNode::Leaf(leaf))
    }

    fn add_node(&mut self, key: UInt256, node: SHAMapAbstractNode) -> bool {
        match &mut self.root {
            None => {
                self.root = Some(Box::new(node));
                true
            }
            Some(root) => Self::insert_recursive(root, key, node, 0),
        }
    }

    fn insert_recursive(
        current: &mut SHAMapAbstractNode,
        key: UInt256,
        node: SHAMapAbstractNode,
        depth: usize,
    ) -> bool {
        if depth >= 64 {
            return false;
        }

        match current {
            SHAMapAbstractNode::Leaf(_) => {
                let old_leaf = std::mem::replace(current, node);
                *current = old_leaf;
                true
            }
            SHAMapAbstractNode::Inner(inner) => {
                let branch = Self::get_branch_index(&key, depth);

                if !inner.is_branch_set(branch) {
                    inner.set_child(branch, node);
                    true
                } else if let Some(child) = inner.get_child_mut(branch) {
                    Self::insert_recursive(child, key, node, depth + 1)
                } else {
                    false
                }
            }
            SHAMapAbstractNode::InnerV2(inner_v2) => {
                let branch = Self::get_branch_index(&key, depth);

                if !inner_v2.inner.is_branch_set(branch) {
                    inner_v2.set_child(branch, node);
                    true
                } else if let Some(child) = inner_v2.get_child_mut(branch) {
                    Self::insert_recursive(child, key, node, depth + 1)
                } else {
                    false
                }
            }
        }
    }

    pub fn get_item(&self, key: &UInt256) -> Option<&SHAMapItem> {
        self.get_node(key).and_then(|n| n.as_leaf().map(|l| l.item()))
    }

    fn get_node(&self, _key: &UInt256) -> Option<&SHAMapAbstractNode> {
        self.root.as_ref().map(|r| r.as_ref())
    }

    pub fn get_root_hash(&self) -> UInt256 {
        match &self.root {
            None => sha512_half(b""),
            Some(root) => root.hash(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    pub fn clear(&mut self) {
        self.root = None;
        self.seq += 1;
    }

    pub fn iter(&self) -> SHAMapIterator {
        SHAMapIterator::new(self.root.as_deref())
    }
}

pub struct SHAMapIterator<'a> {
    stack: Vec<&'a SHAMapAbstractNode>,
}

impl<'a> SHAMapIterator<'a> {
    fn new(root: Option<&'a SHAMapAbstractNode>) -> Self {
        let mut stack = Vec::new();
        if let Some(r) = root {
            stack.push(r);
        }
        Self { stack }
    }
}

impl<'a> Iterator for SHAMapIterator<'a> {
    type Item = &'a SHAMapItem;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop() {
            match node {
                SHAMapAbstractNode::Leaf(leaf) => return Some(leaf.item()),
                SHAMapAbstractNode::Inner(inner) => {
                    for i in (0..16).rev() {
                        if let Some(child) = inner.get_child(i) {
                            self.stack.push(child);
                        }
                    }
                }
                SHAMapAbstractNode::InnerV2(inner_v2) => {
                    for i in (0..16).rev() {
                        if let Some(child) = inner_v2.get_child(i) {
                            self.stack.push(child);
                        }
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_map() {
        let map = SHAMap::new(SHAMapType::State);
        assert!(map.is_empty());
    }

    #[test]
    fn test_add_item() {
        let mut map = SHAMap::new(SHAMapType::State);
        let key = UInt256::new([0u8; 32]);
        let item = SHAMapItem::new(key, vec![1, 2, 3, 4]);

        assert!(map.add_item(key, item));
        assert!(!map.is_empty());
    }

    #[test]
    fn test_get_item() {
        let mut map = SHAMap::new(SHAMapType::State);
        let key = UInt256::new([0u8; 32]);
        let data = vec![1, 2, 3, 4];
        let item = SHAMapItem::new(key, data.clone());

        map.add_item(key, item);

        let retrieved = map.get_item(&key);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().data(), &data[..]);
    }
}
