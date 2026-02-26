#[derive(Debug, Clone)]
pub struct Message {
    pub message_type: MessageType,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Hello,
    Validation,
    Propose,
    Transaction,
    StatusChange,
    HaveTransactionSet,
    GetLedger,
    LedgerData,
}

impl Message {
    pub fn new(message_type: MessageType, payload: Vec<u8>) -> Self {
        Self {
            message_type,
            payload,
        }
    }

    pub fn get_type(&self) -> MessageType {
        self.message_type
    }

    pub fn get_payload(&self) -> &[u8] {
        &self.payload
    }
}
