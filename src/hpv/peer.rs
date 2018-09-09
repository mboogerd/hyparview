use super::HpvRecipient;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::*;

#[derive(Eq, PartialEq, Hash, Clone)]
pub struct Peer {
    pub recipient: HpvRecipient,
}

impl Into<Peer> for HpvRecipient {
    fn into(self) -> Peer {
        Peer { recipient: self }
    }
}

impl fmt::Debug for Peer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}
impl fmt::Display for Peer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut hasher = DefaultHasher::new();
        self.recipient.hash(&mut hasher);
        write!(f, "Peer {}", hasher.finish())
    }
}
