use primitives::{AccountID, UInt256};
use serialization::STObject;

/// Transaction type enum matching calld values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum TxType {
    Invalid = -1,
    Payment = 0,
    IssueSet = 1,
    TrustSet = 2,
    OfferCreate = 3,
    OfferCancel = 4,
    AccountSet = 5,
    SetRegularKey = 6,
    SignerListSet = 7,
}

impl TxType {
    pub fn as_i16(&self) -> i16 {
        *self as i16
    }

    pub fn from_i16(v: i16) -> Option<Self> {
        match v {
            -1 => Some(Self::Invalid),
            0 => Some(Self::Payment),
            1 => Some(Self::IssueSet),
            2 => Some(Self::TrustSet),
            3 => Some(Self::OfferCreate),
            4 => Some(Self::OfferCancel),
            5 => Some(Self::AccountSet),
            6 => Some(Self::SetRegularKey),
            7 => Some(Self::SignerListSet),
            _ => None,
        }
    }
}

/// Transaction Engine Result (TER) codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum TER {
    // Success
    tesSUCCESS = 0,

    // Claimed fee failures (100-199)
    tecCLAIMED = 100,
    tecPATH_PARTIAL = 101,
    tecUNFUNDED_ADD = 102,
    tecUNFUNDED_OFFER = 103,
    tecUNFUNDED_PAYMENT = 104,
    tecFAILED_PROCESSING = 105,
    tecDIR_FULL = 121,
    tecINSUF_RESERVE_LINE = 122,
    tecINSUF_RESERVE_OFFER = 123,
    tecNO_DST = 124,
    tecNO_DST_INSUF_CALL = 125,
    tecNO_LINE_INSUF_RESERVE = 126,
    tecNO_LINE = 127,
    tecOWNERS = 128,
    tecPATH_DRY = 129,
    tecUNFUNDED = 130,
    tecNO_ALTERNATIVE_KEY = 131,
    tecNO_REGULAR_KEY = 132,
    tecDUPLICATE = 149,

    // Malformed transactions (-199 to -100)
    temMALFORMED = -299,
    temBAD_AMOUNT = -298,
    temBAD_AUTH_MASTER = -297,
    temBAD_CURRENCY = -296,
    temBAD_EXPIRATION = -295,
    temBAD_FEE = -294,
    temBAD_ISSUER = -293,
    temBAD_LIMIT = -292,
    temBAD_OFFER = -291,
    temBAD_PATH = -290,
    temBAD_PATH_LOOP = -289,
    temBAD_SEND_CALL_LIMIT = -288,
    temBAD_SEQUENCE = -287,
    temBAD_SIGNATURE = -286,
    temBAD_SIGNER = -285,
    temBAD_TRANSFER_RATE = -284,
    temDST_IS_SRC = -279,
    temINVALID = -271,
    temINVALID_FLAG = -270,
    temREDUNDANT = -264,
    temREDUNDANT_SEND_MAX = -263,
    temBAD_QUORUM = -254,
    temBAD_WEIGHT = -253,

    // Failed preflight check (-399 to -300)
    terRETRY = -99,
    terNO_ACCOUNT = -69,
    terNO_AUTH = -68,
    terNO_LINE = -64,
    terOWNERS = -63,
    terPRE_SEQ = -61,
    terLAST = -60,
    terNO_CALL = -59,
    terNO_CALL_ISSUER = -58,
    terDUPLICATE = -39,
    terFAILED = -16,
}

impl TER {
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    pub fn is_success(&self) -> bool {
        *self == TER::tesSUCCESS
    }

    pub fn is_claimed(&self) -> bool {
        let code = *self as i32;
        code >= 100 && code < 200
    }

    pub fn is_malformed(&self) -> bool {
        let code = *self as i32;
        code <= -100 && code > -400
    }

    pub fn is_preclaim_failure(&self) -> bool {
        let code = *self as i32;
        code < 0 && code >= -100
    }
}

/// Transaction structure
#[derive(Debug, Clone)]
pub struct Transaction {
    pub tx_type: TxType,
    pub account: AccountID,
    pub sequence: u32,
    pub fee: u64,
    pub signing_pub_key: Vec<u8>,
    pub txn_signature: Vec<u8>,
    pub hash: UInt256,
    pub data: STObject,
}

impl Transaction {
    pub fn new(tx_type: TxType, account: AccountID, sequence: u32) -> Self {
        Self {
            tx_type,
            account,
            sequence,
            fee: 10,
            signing_pub_key: Vec::new(),
            txn_signature: Vec::new(),
            hash: UInt256::zero(),
            data: STObject::new(),
        }
    }

    pub fn get_hash(&self) -> UInt256 {
        self.hash
    }

    pub fn get_account(&self) -> AccountID {
        self.account
    }

    pub fn get_sequence(&self) -> u32 {
        self.sequence
    }

    pub fn get_fee(&self) -> u64 {
        self.fee
    }

    pub fn get_tx_type(&self) -> TxType {
        self.tx_type
    }

    pub fn set_fee(&mut self, fee: u64) {
        self.fee = fee;
    }

    pub fn set_signing_pub_key(&mut self, key: Vec<u8>) {
        self.signing_pub_key = key;
    }

    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.txn_signature = signature;
    }

    pub fn set_hash(&mut self, hash: UInt256) {
        self.hash = hash;
    }
}

/// Transaction metadata
#[derive(Debug, Clone)]
pub struct TransactionMetadata {
    pub transaction_hash: UInt256,
    pub affected_nodes: Vec<AffectedNode>,
    pub delivered_amount: Option<u64>,
}

impl TransactionMetadata {
    pub fn new(transaction_hash: UInt256) -> Self {
        Self {
            transaction_hash,
            affected_nodes: Vec::new(),
            delivered_amount: None,
        }
    }

    pub fn add_affected_node(&mut self, node: AffectedNode) {
        self.affected_nodes.push(node);
    }
}

/// Affected node in transaction metadata
#[derive(Debug, Clone)]
pub struct AffectedNode {
    pub ledger_entry_type: u16,
    pub ledger_index: UInt256,
    pub previous_fields: Option<STObject>,
    pub final_fields: Option<STObject>,
    pub created: bool,
    pub deleted: bool,
}

impl AffectedNode {
    pub fn new(ledger_index: UInt256, ledger_entry_type: u16) -> Self {
        Self {
            ledger_entry_type,
            ledger_index,
            previous_fields: None,
            final_fields: None,
            created: false,
            deleted: false,
        }
    }

    pub fn set_created(&mut self) {
        self.created = true;
    }

    pub fn set_deleted(&mut self) {
        self.deleted = true;
    }

    pub fn set_previous_fields(&mut self, fields: STObject) {
        self.previous_fields = Some(fields);
    }

    pub fn set_final_fields(&mut self, fields: STObject) {
        self.final_fields = Some(fields);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ter_success() {
        assert!(TER::tesSUCCESS.is_success());
        assert!(!TER::tecCLAIMED.is_success());
        assert!(!TER::temMALFORMED.is_success());
        assert!(!TER::terNO_ACCOUNT.is_success());
    }

    #[test]
    fn test_ter_claimed() {
        assert!(!TER::tesSUCCESS.is_claimed());
        assert!(TER::tecCLAIMED.is_claimed());
        assert!(TER::tecPATH_PARTIAL.is_claimed());
        assert!(!TER::temMALFORMED.is_claimed());
    }

    #[test]
    fn test_tx_type_roundtrip() {
        assert_eq!(TxType::Payment.as_i16(), 0);
        assert_eq!(TxType::IssueSet.as_i16(), 1);
        assert_eq!(TxType::from_i16(0), Some(TxType::Payment));
        assert_eq!(TxType::from_i16(1), Some(TxType::IssueSet));
        assert_eq!(TxType::from_i16(999), None);
    }

    #[test]
    fn test_transaction_basic() {
        let account = AccountID::new([0u8; 20]);
        let tx = Transaction::new(TxType::Payment, account, 1);
        assert_eq!(tx.get_sequence(), 1);
        assert_eq!(tx.get_fee(), 10);
        assert_eq!(tx.get_tx_type(), TxType::Payment);
    }
}
