use crate::Result;

pub struct IPFSNode;

impl IPFSNode {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
    
    pub async fn start_private_node(&self) -> Result<()> {
        // TODO: Implement private IPFS node
        Ok(())
    }
}
