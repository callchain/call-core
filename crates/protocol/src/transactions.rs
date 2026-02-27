use primitives::{AccountID, UInt256};
use serialization::{Amount, STObject};

/// Transaction type enum matching calld values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum TxType {
    Invalid = -1,
    Payment = 0,
    AccountSet = 3,
    SetRegularKey = 5,
    OfferCreate = 7,
    OfferCancel = 8,
    SignerListSet = 12,
    IssueSet = 16,
    TrustSet = 20,
}

impl TxType {
    pub fn as_i16(&self) -> i16 {
        *self as i16
    }

    pub fn from_i16(v: i16) -> Option<Self> {
        match v {
            -1 => Some(Self::Invalid),
            0 => Some(Self::Payment),
            3 => Some(Self::AccountSet),
            5 => Some(Self::SetRegularKey),
            7 => Some(Self::OfferCreate),
            8 => Some(Self::OfferCancel),
            12 => Some(Self::SignerListSet),
            16 => Some(Self::IssueSet),
            20 => Some(Self::TrustSet),
            _ => None,
        }
    }
}

/// Signer entry for SignerListSet transaction
#[derive(Debug, Clone)]
pub struct SignerEntry {
    pub account: AccountID,
    pub weight: u8,
}

/// Transaction Engine Result (TER) codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
#[allow(non_camel_case_types)]
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
    terNO_OFFER = -57,
    terINSUFF_FEE = -56,
    terDUPLICATE = -39,
    terFAILED = -16,

    // Additional TER codes needed by engine
    temINVALID_TRANSACTION_TYPE = -269,
    temEMPTY_SIGNER = -260,
    temDST_NEEDED = -278,
    temBAD_TICK_SIZE = -255,
    temBAD_REGULAR_KEY = -265,
    temBAD_SIGNER_LIST = -252,
    telINSUFFICIENT_FEE = -394,
    tecINTERNAL = 199,
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
    pub signing_pub_key: Option<Vec<u8>>,
    pub txn_signature: Option<Vec<u8>>,
    pub hash: UInt256,
    pub data: STObject,
    // Payment fields
    pub destination: Option<AccountID>,
    pub amount: Option<Amount>,
    pub destination_tag: Option<u32>,
    pub send_max: Option<Amount>,
    // TrustSet fields
    pub limit_amount: Option<Amount>,
    pub issuer: Option<AccountID>,
    pub quality_in: Option<u32>,
    pub quality_out: Option<u32>,
    // OfferCreate/Cancel fields
    pub taker_pays: Option<Amount>,
    pub taker_gets: Option<Amount>,
    pub offer_sequence: u32,
    pub expiration: Option<u32>,
    // AccountSet fields
    pub domain: Option<Vec<u8>>,
    pub email_hash: Option<UInt256>,
    pub message_key: Option<Vec<u8>>,
    pub transfer_rate: Option<u32>,
    pub tick_size: Option<u8>,
    pub set_flag: Option<u32>,
    pub clear_flag: Option<u32>,
    // SetRegularKey fields
    pub regular_key: Option<AccountID>,
    // SignerListSet fields
    pub signer_quorum: u32,
    pub signers: Vec<SignerEntry>,
    // IssueSet fields
    pub total_supply: Option<Amount>,
}

impl Transaction {
    pub fn new(tx_type: TxType, account: AccountID, sequence: u32) -> Self {
        Self {
            tx_type,
            account,
            sequence,
            fee: 10,
            signing_pub_key: None,
            txn_signature: None,
            hash: UInt256::zero(),
            data: STObject::new(),
            destination: None,
            amount: None,
            destination_tag: None,
            send_max: None,
            limit_amount: None,
            issuer: None,
            quality_in: None,
            quality_out: None,
            taker_pays: None,
            taker_gets: None,
            offer_sequence: 0,
            expiration: None,
            domain: None,
            email_hash: None,
            message_key: None,
            transfer_rate: None,
            tick_size: None,
            set_flag: None,
            clear_flag: None,
            regular_key: None,
            signer_quorum: 0,
            signers: Vec::new(),
            total_supply: None,
        }
    }

    /// Create a new payment transaction
    pub fn new_payment(account: AccountID, destination: AccountID, amount: Amount) -> Self {
        let mut tx = Self::new(TxType::Payment, account, 1);
        tx.destination = Some(destination);
        tx.amount = Some(amount);
        tx
    }

    /// Create a new trust set transaction
    pub fn new_trust_set(account: AccountID, limit_amount: Amount, issuer: AccountID) -> Self {
        let mut tx = Self::new(TxType::TrustSet, account, 1);
        tx.limit_amount = Some(limit_amount);
        tx.issuer = Some(issuer);
        tx
    }

    /// Create a new offer create transaction
    pub fn new_offer_create(
        account: AccountID,
        taker_pays: Amount,
        taker_gets: Amount,
        sequence: u32,
    ) -> Self {
        let mut tx = Self::new(TxType::OfferCreate, account, sequence);
        tx.taker_pays = Some(taker_pays);
        tx.taker_gets = Some(taker_gets);
        tx
    }

    /// Create a new offer cancel transaction
    pub fn new_offer_cancel(account: AccountID, offer_sequence: u32, sequence: u32) -> Self {
        let mut tx = Self::new(TxType::OfferCancel, account, sequence);
        tx.offer_sequence = offer_sequence;
        tx
    }

    /// Create a new account set transaction
    pub fn new_account_set(account: AccountID, sequence: u32) -> Self {
        Self::new(TxType::AccountSet, account, sequence)
    }

    /// Create a new set regular key transaction
    pub fn new_set_regular_key(account: AccountID, regular_key: AccountID, sequence: u32) -> Self {
        let mut tx = Self::new(TxType::SetRegularKey, account, sequence);
        tx.regular_key = Some(regular_key);
        tx
    }

    /// Create a new signer list set transaction
    pub fn new_signer_list_set(account: AccountID, quorum: u32, sequence: u32) -> Self {
        let mut tx = Self::new(TxType::SignerListSet, account, sequence);
        tx.signer_quorum = quorum;
        tx
    }

    /// Create a new issue set transaction
    pub fn new_issue_set(account: AccountID, amount: Amount, sequence: u32) -> Self {
        let mut tx = Self::new(TxType::IssueSet, account, sequence);
        tx.amount = Some(amount);
        tx
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
        self.signing_pub_key = Some(key);
    }

    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.txn_signature = Some(signature);
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
        // Values must match calld specification
        assert_eq!(TxType::Payment.as_i16(), 0);
        assert_eq!(TxType::AccountSet.as_i16(), 3);
        assert_eq!(TxType::SetRegularKey.as_i16(), 5);
        assert_eq!(TxType::OfferCreate.as_i16(), 7);
        assert_eq!(TxType::OfferCancel.as_i16(), 8);
        assert_eq!(TxType::SignerListSet.as_i16(), 12);
        assert_eq!(TxType::IssueSet.as_i16(), 16);
        assert_eq!(TxType::TrustSet.as_i16(), 20);

        // Test from_i16 roundtrip
        assert_eq!(TxType::from_i16(0), Some(TxType::Payment));
        assert_eq!(TxType::from_i16(3), Some(TxType::AccountSet));
        assert_eq!(TxType::from_i16(5), Some(TxType::SetRegularKey));
        assert_eq!(TxType::from_i16(7), Some(TxType::OfferCreate));
        assert_eq!(TxType::from_i16(8), Some(TxType::OfferCancel));
        assert_eq!(TxType::from_i16(12), Some(TxType::SignerListSet));
        assert_eq!(TxType::from_i16(16), Some(TxType::IssueSet));
        assert_eq!(TxType::from_i16(20), Some(TxType::TrustSet));
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
