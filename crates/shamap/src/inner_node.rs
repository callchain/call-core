use crypto::HashPrefix;
use primitives::UInt256;
use serialization::{Serializer, SerialIter};

#[derive(Debug, Clone)]
pub struct SHAMapInnerNode {
    hashes: [UInt256; 16],
    children: [Option<Box<crate::SHAMapAbstractNode>>; 16],
    is_branch: u16,
    seq: u32,
}

impl SHAMapInnerNode {
    pub fn new(seq: u32) -> Self {
        Self {
            hashes: [UInt256::zero(); 16],
            children: [
                None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
            ],
            is_branch: 0,
            seq,
        }
    }

    pub fn get_child(&self, branch: usize) -> Option<&crate::SHAMapAbstractNode> {
        self.children[branch].as_ref().map(|b| b.as_ref())
    }

    pub fn get_child_mut(&mut self, branch: usize) -> Option<&mut crate::SHAMapAbstractNode> {
        self.children[branch].as_mut().map(|b| b.as_mut())
    }

    pub fn set_child(&mut self, branch: usize, child: crate::SHAMapAbstractNode) {
        self.children[branch] = Some(Box::new(child));
        self.is_branch |= 1 << branch;
    }

    pub fn remove_child(&mut self, branch: usize) {
        self.children[branch] = None;
        self.is_branch &= !(1 << branch);
        self.hashes[branch] = UInt256::zero();
    }

    pub fn get_hash(&self, branch: usize) -> UInt256 {
        self.hashes[branch]
    }

    pub fn set_hash(&mut self, branch: usize, hash: UInt256) {
        self.hashes[branch] = hash;
    }

    pub fn is_branch_set(&self, branch: usize) -> bool {
        (self.is_branch >> branch) & 1 == 1
    }

    pub fn get_branch_count(&self) -> u32 {
        self.is_branch.count_ones()
    }

    pub fn get_depth(&self) -> usize {
        0
    }

    pub fn compute_hash(&mut self) {
        if self.is_branch == 0 {
            return;
        }

        let mut serializer = Serializer::with_capacity(516);
        serializer.add32(HashPrefix::InnerNode.as_u32());

        for i in 0..16 {
            serializer.add256(self.hashes[i]);
        }

        let _hash = crypto::sha512_half(serializer.as_slice());
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut serializer = Serializer::with_capacity(516);
        serializer.add32(HashPrefix::InnerNode.as_u32());

        for i in 0..16 {
            serializer.add256(self.hashes[i]);
        }

        serializer.finish()
    }

    pub fn deserialize(data: &[u8], _seq: u32) -> Option<Self> {
        if data.len() < 516 {
            return None;
        }

        let mut iter = SerialIter::new(data);
        let _prefix = iter.get32().ok()?;

        let mut node = Self::new(0);
        for i in 0..16 {
            node.hashes[i] = iter.get256().ok()?;
            if node.hashes[i] != UInt256::zero() {
                node.is_branch |= 1 << i;
            }
        }

        Some(node)
    }
}

#[derive(Debug, Clone)]
pub struct SHAMapInnerNodeV2 {
    pub inner: SHAMapInnerNode,
    common: UInt256,
    depth: u8,
}

impl SHAMapInnerNodeV2 {
    pub fn new(seq: u32, depth: u8, common: UInt256) -> Self {
        Self {
            inner: SHAMapInnerNode::new(seq),
            common,
            depth,
        }
    }

    pub fn get_child(&self, branch: usize) -> Option<&crate::SHAMapAbstractNode> {
        self.inner.get_child(branch)
    }

    pub fn get_child_mut(&mut self, branch: usize) -> Option<&mut crate::SHAMapAbstractNode> {
        self.inner.get_child_mut(branch)
    }

    pub fn set_child(&mut self, branch: usize, child: crate::SHAMapAbstractNode) {
        self.inner.set_child(branch, child);
    }

    pub fn get_depth(&self) -> usize {
        self.depth as usize
    }

    pub fn get_common(&self) -> UInt256 {
        self.common
    }

    pub fn compute_hash(&mut self) {
        if self.inner.is_branch == 0 {
            return;
        }

        let mut serializer = Serializer::with_capacity(580);
        serializer.add32(HashPrefix::InnerNodeV2.as_u32());

        for i in 0..16 {
            serializer.add256(self.inner.hashes[i]);
        }

        serializer.add8(self.depth);

        let common_len = (self.depth as usize + 1) / 2;
        let common_bytes = &self.common.as_bytes()[..common_len];
        serializer.add_vl(common_bytes);

        let _hash = crypto::sha512_half(serializer.as_slice());
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut serializer = Serializer::with_capacity(580);
        serializer.add32(HashPrefix::InnerNodeV2.as_u32());

        for i in 0..16 {
            serializer.add256(self.inner.hashes[i]);
        }

        serializer.add8(self.depth);

        let common_len = (self.depth as usize + 1) / 2;
        let common_bytes = &self.common.as_bytes()[..common_len];
        serializer.add_vl(common_bytes);

        serializer.finish()
    }
}
