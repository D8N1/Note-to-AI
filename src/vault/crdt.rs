use crate::Result;
use automerge::Automerge;

pub struct CRDT {
    doc: Automerge,
}

impl CRDT {
    pub fn new() -> Result<Self> {
        Ok(Self {
            doc: Automerge::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crdt_new() {
        let crdt = CRDT::new().unwrap();
        assert!(crdt.doc.is_empty());
    }

    #[test]
    fn test_crdt_creation_consistency() {
        let crdt1 = CRDT::new().unwrap();
        let crdt2 = CRDT::new().unwrap();
        assert_eq!(crdt1.doc.is_empty(), crdt2.doc.is_empty());
    }
}
