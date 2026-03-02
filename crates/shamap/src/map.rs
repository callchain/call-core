use crate::SHAMapAbstractNode;
use crate::inner_node::SHAMapInnerNode;
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
            SHAMapAbstractNode::Leaf(leaf) => {
                // Get the key of the existing leaf
                let existing_key = leaf.item().key();

                // If same key, replace (update)
                if existing_key == key {
                    *current = node;
                    true
                } else {
                    // Different keys - need to create inner node and add both leaves
                    let existing_branch = Self::get_branch_index(&existing_key, depth);
                    let new_branch = Self::get_branch_index(&key, depth);

                    if existing_branch == new_branch {
                        // Same branch - need to go deeper
                        // Create inner node and recursively insert both
                        let mut new_inner = SHAMapInnerNode::new(0);

                        // Take the old leaf out and add it to the new inner node
                        let old_leaf = std::mem::replace(current, SHAMapAbstractNode::Inner(SHAMapInnerNode::new(0)));
                        new_inner.set_child(existing_branch, old_leaf);

                        // Now recursively add the new node
                        if let Some(child) = new_inner.get_child_mut(new_branch) {
                            Self::insert_recursive(child, key, node, depth + 1);
                        }

                        *current = SHAMapAbstractNode::Inner(new_inner);
                        true
                    } else {
                        // Different branches - create inner node with both children
                        let mut new_inner = SHAMapInnerNode::new(0);
                        let old_leaf = std::mem::replace(current, SHAMapAbstractNode::Inner(new_inner));

                        // Set up new inner node with both children
                        if let SHAMapAbstractNode::Inner(inner) = current {
                            inner.set_child(existing_branch, old_leaf);
                            inner.set_child(new_branch, node);
                        }
                        true
                    }
                }
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

    /// Remove an item from the map by key
    /// Returns true if the item was found and removed, false otherwise
    pub fn remove_item(&mut self, key: &UInt256) -> bool {
        if self.root.is_none() {
            return false;
        }

        let (removed, new_root) = Self::remove_recursive(self.root.take(), key, 0);
        self.root = new_root;
        removed
    }

    /// Recursive helper for remove_item
    /// Returns (was_removed, new_node) where new_node may be None if this branch is now empty
    fn remove_recursive(
        node: Option<Box<SHAMapAbstractNode>>,
        key: &UInt256,
        depth: usize,
    ) -> (bool, Option<Box<SHAMapAbstractNode>>) {
        if depth >= 64 {
            return (false, node);
        }

        let mut node = match node {
            Some(n) => n,
            None => return (false, None),
        };

        match node.as_mut() {
            SHAMapAbstractNode::Leaf(leaf) => {
                if leaf.item().key() == *key {
                    // Found the leaf to remove
                    return (true, None);
                }
                // Key doesn't match, keep the leaf
                (false, Some(node))
            }
            SHAMapAbstractNode::Inner(inner) => {
                let branch = Self::get_branch_index(key, depth);

                if !inner.is_branch_set(branch) {
                    return (false, Some(node));
                }

                // Get the child and recursively remove
                let child = inner.take_child(branch);
                let (removed, new_child) = Self::remove_recursive(child, key, depth + 1);

                if !removed {
                    // Not found in this branch, restore child and return
                    if let Some(child) = new_child {
                        inner.set_child(branch, *child);
                    }
                    return (false, Some(node));
                }

                // Item was removed from child
                if let Some(child) = new_child {
                    // Child still exists, restore it
                    inner.set_child(branch, *child);
                    (true, Some(node))
                } else {
                    // Child is now empty, remove this branch
                    inner.remove_child(branch);

                    // Check if this inner node now has only one child
                    // If so, we can collapse it
                    let remaining_children: Vec<_> = (0..16)
                        .filter(|&i| inner.is_branch_set(i))
                        .collect();

                    if remaining_children.is_empty() {
                        // No children left, remove this node
                        (true, None)
                    } else if remaining_children.len() == 1 {
                        // Only one child - in some map implementations we could
                        // collapse, but for SHAMap we need to maintain the structure
                        // for hash consistency, so keep the inner node
                        (true, Some(node))
                    } else {
                        (true, Some(node))
                    }
                }
            }
            SHAMapAbstractNode::InnerV2(inner_v2) => {
                let branch = Self::get_branch_index(key, depth);

                if !inner_v2.is_branch_set(branch) {
                    return (false, Some(node));
                }

                // Get the child and recursively remove
                let child = inner_v2.take_child(branch);
                let (removed, new_child) = Self::remove_recursive(child, key, depth + 1);

                if !removed {
                    // Not found in this branch, restore child and return
                    if let Some(child) = new_child {
                        inner_v2.set_child(branch, *child);
                    }
                    return (false, Some(node));
                }

                // Item was removed from child
                if let Some(child) = new_child {
                    // Child still exists, restore it
                    inner_v2.set_child(branch, *child);
                    (true, Some(node))
                } else {
                    // Child is now empty, remove this branch
                    inner_v2.remove_child(branch);

                    if inner_v2.is_empty() {
                        (true, None)
                    } else {
                        (true, Some(node))
                    }
                }
            }
        }
    }

    fn get_node(&self, key: &UInt256) -> Option<&SHAMapAbstractNode> {
        let mut current = self.root.as_deref()?;
        let mut depth = 0;

        loop {
            match current {
                SHAMapAbstractNode::Leaf(leaf) => {
                    // Check if this leaf's key matches
                    if leaf.item().key() == *key {
                        return Some(current);
                    }
                    return None;
                }
                SHAMapAbstractNode::Inner(inner) => {
                    let branch = Self::get_branch_index(key, depth);
                    if let Some(child) = inner.get_child(branch) {
                        current = child;
                        depth += 1;
                    } else {
                        return None;
                    }
                }
                SHAMapAbstractNode::InnerV2(inner_v2) => {
                    let branch = Self::get_branch_index(key, depth);
                    if let Some(child) = inner_v2.get_child(branch) {
                        current = child;
                        depth += 1;
                    } else {
                        return None;
                    }
                }
            }
        }
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

    pub fn iter(&self) -> SHAMapIterator<'_> {
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
