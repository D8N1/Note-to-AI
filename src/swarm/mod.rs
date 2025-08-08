pub mod discovery;
pub mod ipfs;
pub mod sync;

use crate::Result;

pub struct Swarm;

impl Swarm {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
    
    pub async fn start(&self) -> Result<()> {
        // TODO: Implement IPFS private swarm
        Ok(())
    }
}// TODO: implement this file
