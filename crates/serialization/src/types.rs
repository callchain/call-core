use primitives::{AccountID, Currency, UInt128, UInt160, UInt256};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(i16)]
pub enum SerializedTypeID {
    NotPresent = 0,
    UInt16 = 1,
    UInt32 = 2,
    UInt64 = 3,
    Hash128 = 4,
    Hash256 = 5,
    Amount = 6,
    VL = 7,
    Account = 8,
    Object = 14,
    Array = 15,
    UInt8 = 16,
    Hash160 = 17,
    PathSet = 18,
    Vector256 = 19,
    Transaction = 10001,
    LedgerEntry = 10002,
    Validation = 10003,
    Metadata = 10004,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SField {
    pub type_id: SerializedTypeID,
    pub field_code: u32,
    pub field_name: &'static str,
    pub is_signing_field: bool,
    pub is_serialized: bool,
}

impl SField {
    pub const fn new(
        type_id: SerializedTypeID,
        field_num: u16,
        name: &'static str,
        signing: bool,
        serialized: bool,
    ) -> Self {
        let type_val = type_id as i16;
        let field_code = ((type_val as u32) << 16) | (field_num as u32);
        Self {
            type_id,
            field_code,
            field_name: name,
            is_signing_field: signing,
            is_serialized: serialized,
        }
    }

    pub fn type_id(&self) -> SerializedTypeID {
        self.type_id
    }

    pub fn field_code(&self) -> u32 {
        self.field_code
    }
}

pub mod sf {
    use super::*;

    macro_rules! define_fields {
        ($($name:ident = ($type:expr, $num:expr, $signing:expr, $serialized:expr)),* $(,)?) => {
            $(pub const $name: SField = SField::new($type, $num, stringify!($name), $signing, $serialized);)*
        };
    }

    define_fields! {
        LEDGER_SEQUENCE = (SerializedTypeID::UInt32, 6, true, true),
        CLOSE_TIME = (SerializedTypeID::UInt32, 7, true, true),
        PARENT_CLOSE_TIME = (SerializedTypeID::UInt32, 8, true, true),
        SIGNING_TIME = (SerializedTypeID::UInt32, 9, true, true),
        EXPIRATION = (SerializedTypeID::UInt32, 10, true, true),
        TRANSFER_RATE = (SerializedTypeID::UInt32, 11, true, true),
        WALLET_SIZE = (SerializedTypeID::UInt32, 12, true, true),
        OWNER_COUNT = (SerializedTypeID::UInt32, 13, true, true),
        DESTINATION_TAG = (SerializedTypeID::UInt32, 14, true, true),
        HIGH_QUALITY_IN = (SerializedTypeID::UInt32, 16, true, true),
        HIGH_QUALITY_OUT = (SerializedTypeID::UInt32, 17, true, true),
        LOW_QUALITY_IN = (SerializedTypeID::UInt32, 18, true, true),
        LOW_QUALITY_OUT = (SerializedTypeID::UInt32, 19, true, true),
        QUALITY_IN = (SerializedTypeID::UInt32, 20, true, true),
        QUALITY_OUT = (SerializedTypeID::UInt32, 21, true, true),
        FLAGS = (SerializedTypeID::UInt32, 22, true, true),
        SOURCE_TAG = (SerializedTypeID::UInt32, 3, true, true),
        SEQUENCE = (SerializedTypeID::UInt32, 4, true, true),
        PREVIOUS_TXN_LGR_SEQ = (SerializedTypeID::UInt32, 5, true, true),
        LEDGER_ENTRY_TYPE = (SerializedTypeID::UInt16, 1, true, true),
        TRANSACTION_TYPE = (SerializedTypeID::UInt16, 2, true, true),
        SIGNER_WEIGHT = (SerializedTypeID::UInt16, 3, true, true),
        TICK_SIZE = (SerializedTypeID::UInt8, 16, true, true),
        VERSION = (SerializedTypeID::UInt16, 16, true, true),
        FEE = (SerializedTypeID::Amount, 8, true, true),
        SEND_MAX = (SerializedTypeID::Amount, 9, true, true),
        DELIVER_MIN = (SerializedTypeID::Amount, 10, true, true),
        MINIMUM_OFFER = (SerializedTypeID::Amount, 16, true, true),
        CALL_BALANCE = (SerializedTypeID::Amount, 2, true, true),
        LOW_LIMIT = (SerializedTypeID::Amount, 3, true, true),
        HIGH_LIMIT = (SerializedTypeID::Amount, 4, true, true),
        TAKER_PAYS = (SerializedTypeID::Amount, 5, true, true),
        TAKER_GETS = (SerializedTypeID::Amount, 6, true, true),
        BALANCE = (SerializedTypeID::Amount, 1, true, true),
        AMOUNT = (SerializedTypeID::Amount, 1, true, true),
        LIMIT_AMOUNT = (SerializedTypeID::Amount, 17, true, true),
        TAKER_GET_PAID = (SerializedTypeID::Amount, 19, true, true),
        TAKER_PAY = (SerializedTypeID::Amount, 20, true, true),
        INDEX_NEXT = (SerializedTypeID::UInt64, 1, true, true),
        INDEX_PREVIOUS = (SerializedTypeID::UInt64, 2, true, true),
        BOOK_NODE = (SerializedTypeID::UInt64, 3, true, true),
        OWNER_NODE = (SerializedTypeID::UInt64, 4, true, true),
        BASE_FEE = (SerializedTypeID::UInt64, 5, true, true),
        EXCHANGE_RATE = (SerializedTypeID::UInt64, 6, true, true),
        LOW_NODE = (SerializedTypeID::UInt64, 7, true, true),
        HIGH_NODE = (SerializedTypeID::UInt64, 8, true, true),
        DESTINATION_NODE = (SerializedTypeID::UInt64, 9, true, true),
        EMAIL_HASH = (SerializedTypeID::Hash128, 2, true, true),
        LEDGER_HASH = (SerializedTypeID::Hash256, 1, true, true),
        PARENT_HASH = (SerializedTypeID::Hash256, 2, true, true),
        TRANSACTION_HASH = (SerializedTypeID::Hash256, 3, true, true),
        ACCOUNT_HASH = (SerializedTypeID::Hash256, 4, true, true),
        PREVIOUS_TXN_ID = (SerializedTypeID::Hash256, 5, true, true),
        LEDGER_INDEX = (SerializedTypeID::Hash256, 6, true, true),
        WALLET_LOCATOR = (SerializedTypeID::Hash256, 7, true, true),
        ROOT_INDEX = (SerializedTypeID::Hash256, 8, true, true),
        ACCOUNT_TXN_ID = (SerializedTypeID::Hash256, 9, true, true),
        BOOK_DIRECTORY = (SerializedTypeID::Hash256, 16, true, true),
        INVOICE_ID = (SerializedTypeID::Hash256, 17, true, true),
        NICKNAME = (SerializedTypeID::Hash256, 18, true, true),
        FEATURE = (SerializedTypeID::Hash256, 19, true, true),
        AMENDMENT = (SerializedTypeID::Hash256, 20, true, true),
        DIGEST = (SerializedTypeID::Hash256, 22, true, true),
        CONSENSUS_HASH = (SerializedTypeID::Hash256, 24, true, true),
        CHECK_ID = (SerializedTypeID::Hash256, 25, true, true),
        VALIDATED_HASH = (SerializedTypeID::Hash256, 26, true, true),
        CHALLENGE_NODE = (SerializedTypeID::Hash256, 27, true, true),
        ADDRESS = (SerializedTypeID::Account, 1, true, true),
        BALANCE_OWNER = (SerializedTypeID::Account, 2, true, true),
        REGULAR_KEY = (SerializedTypeID::Account, 8, true, true),
        AUTHORIZE = (SerializedTypeID::Account, 9, true, true),
        UNAUTHORIZE = (SerializedTypeID::Account, 10, true, true),
        DESTINATION = (SerializedTypeID::Account, 3, true, true),
        ISSUER = (SerializedTypeID::Account, 4, true, true),
        OWNER = (SerializedTypeID::Account, 5, true, true),
        TARGET = (SerializedTypeID::Account, 7, true, true),
        ACCOUNT = (SerializedTypeID::Account, 1, true, true),
        OBJECT_END_MARKER = (SerializedTypeID::Object, 1, true, true),
        TRANSACTION_META_DATA = (SerializedTypeID::Object, 2, true, true),
        CREATED_NODE = (SerializedTypeID::Object, 3, true, true),
        DELETED_NODE = (SerializedTypeID::Object, 4, true, true),
        MODIFIED_NODE = (SerializedTypeID::Object, 5, true, true),
        PREVIOUS_FIELDS = (SerializedTypeID::Object, 6, true, true),
        FINAL_FIELDS = (SerializedTypeID::Object, 7, true, true),
        NEW_FIELDS = (SerializedTypeID::Object, 8, true, true),
        TEMPLATE_ENTRY = (SerializedTypeID::Object, 9, true, true),
        SIGNER_ENTRY = (SerializedTypeID::Object, 10, true, true),
        SIGNER = (SerializedTypeID::Object, 11, true, true),
        MAJORITY = (SerializedTypeID::Object, 16, true, true),
        DISABLED_VALIDATOR = (SerializedTypeID::Object, 17, true, true),
        EMITTED_DETAILS = (SerializedTypeID::Object, 18, true, true),
        ARRAY_END_MARKER = (SerializedTypeID::Array, 1, true, true),
        SIGNING_ACCOUNTS = (SerializedTypeID::Array, 2, true, true),
        TXN_SIGNATURES = (SerializedTypeID::Array, 3, true, true),
        SIGNATURES = (SerializedTypeID::Array, 4, true, true),
        TEMPLATE = (SerializedTypeID::Array, 5, true, true),
        NECESSARY = (SerializedTypeID::Array, 6, true, true),
        SUFFICIENT = (SerializedTypeID::Array, 7, true, true),
        AFFECTED_NODES = (SerializedTypeID::Array, 8, true, true),
        MEMOS = (SerializedTypeID::Array, 9, true, true),
        SIGNER_ENTRIES = (SerializedTypeID::Array, 10, true, true),
        SIGNERS = (SerializedTypeID::Array, 11, true, true),
        MAJORITIES = (SerializedTypeID::Array, 16, true, true),
        DISABLED_VALIDATORS = (SerializedTypeID::Array, 17, true, true),
        EMITTED_TXN = (SerializedTypeID::Array, 18, true, true),
        HOOK_EXECUTION = (SerializedTypeID::Array, 19, true, true),
        HOOK_EXECUTIONS = (SerializedTypeID::Array, 20, true, true),
        HOOK_PARAMETER = (SerializedTypeID::Array, 21, true, true),
        HOOK_PARAMETERS = (SerializedTypeID::Array, 22, true, true),
        HOOK_GRANT = (SerializedTypeID::Array, 23, true, true),
        HOOK_GRANTS = (SerializedTypeID::Array, 24, true, true),
        HOOKS = (SerializedTypeID::Array, 25, true, true),
        PATHS = (SerializedTypeID::PathSet, 1, true, true),
        CLOSE_RESOLUTION = (SerializedTypeID::UInt8, 1, true, true),
        METHOD = (SerializedTypeID::UInt8, 2, true, true),
        TRANSACTION_RESULT = (SerializedTypeID::UInt8, 3, true, true),
        CODE_GARAGE = (SerializedTypeID::UInt32, 47, true, true),
        TAKER_PAYS_CURRENCY = (SerializedTypeID::Hash160, 1, true, true),
        TAKER_PAYS_ISSUER = (SerializedTypeID::Hash160, 2, true, true),
        TAKER_GETS_CURRENCY = (SerializedTypeID::Hash160, 3, true, true),
        TAKER_GETS_ISSUER = (SerializedTypeID::Hash160, 4, true, true),
        PATHS_CANONICAL = (SerializedTypeID::PathSet, 2, true, true),
        PATHS_SET = (SerializedTypeID::PathSet, 1, true, true),
        INDEXES = (SerializedTypeID::Vector256, 1, true, true),
        HASHES = (SerializedTypeID::Vector256, 2, true, true),
        FEATURES = (SerializedTypeID::Vector256, 3, true, true),
        TRANSACTIONS = (SerializedTypeID::Vector256, 4, true, true),
        AMENDMENTS = (SerializedTypeID::Vector256, 5, true, true),
        CURRENCY = (SerializedTypeID::VL, 1, true, true),
        SIGNER_LIST_ID = (SerializedTypeID::UInt32, 41, true, true),
        SET_FLAG = (SerializedTypeID::UInt32, 33, true, true),
        CLEAR_FLAG = (SerializedTypeID::UInt32, 34, true, true),
        SIGNER_QUORUM = (SerializedTypeID::UInt32, 35, true, true),
        SIGNER_LIST_SEQUENCE = (SerializedTypeID::UInt32, 38, true, true),
        BURNED_NF_TOKENS = (SerializedTypeID::UInt32, 42, true, true),
        MINTED_TOKENS = (SerializedTypeID::UInt32, 43, true, true),
        HOOK_STATE_COUNT = (SerializedTypeID::UInt32, 45, true, true),
        EMIT_GENERATION = (SerializedTypeID::UInt32, 46, true, true),
        HOOK_EXECUTION_INDEX = (SerializedTypeID::UInt16, 16, true, true),
        HOOK_API_VERSION = (SerializedTypeID::UInt16, 17, true, true),
        OPERATION_LIMIT = (SerializedTypeID::UInt16, 44, true, true),
        REFERENCE_FEE_UNITS = (SerializedTypeID::UInt16, 8, true, true),
        RESERVE_BASE = (SerializedTypeID::UInt32, 31, true, true),
        RESERVE_INCREMENT = (SerializedTypeID::UInt32, 32, true, true),
        HOOK_ON = (SerializedTypeID::UInt64, 16, true, true),
        HOOK_INSTRUCTION_COUNT = (SerializedTypeID::UInt64, 17, true, true),
        EMIT_BURDEN = (SerializedTypeID::UInt64, 18, true, true),
        HOOK_RETURN_CODE = (SerializedTypeID::UInt64, 19, true, true),
        HOOK_RETURN_STRING = (SerializedTypeID::VL, 17, true, true),
        HOOK_NAMESPACE = (SerializedTypeID::Hash256, 16, true, true),
        HOOK_SET_TXN_ID = (SerializedTypeID::Hash256, 18, true, true),
        HOOK_PARAMETER_NAME = (SerializedTypeID::VL, 18, true, true),
        HOOK_PARAMETER_VALUE = (SerializedTypeID::VL, 19, true, true),
        HOOK_HASH = (SerializedTypeID::Hash256, 19, true, true),
        HOOK_GRANT_AUTHORIZATION = (SerializedTypeID::Account, 16, true, true),
        HOOK_GRANT_AUTHORIZE = (SerializedTypeID::Account, 17, true, true),
        HOOK_STATE_KEY = (SerializedTypeID::VL, 20, true, true),
        HOOK_STATE_DATA = (SerializedTypeID::VL, 21, true, true),
        PUBLIC_KEY = (SerializedTypeID::VL, 1, true, true),
        MESSAGE_KEY = (SerializedTypeID::VL, 2, true, true),
        SIGNING_PUB_KEY = (SerializedTypeID::VL, 3, true, true),
        TXN_SIGNATURE = (SerializedTypeID::VL, 4, false, true),
        SIGNATURE = (SerializedTypeID::VL, 6, true, true),
        DOMAIN = (SerializedTypeID::VL, 7, true, true),
        FUND_CODE = (SerializedTypeID::VL, 8, true, true),
        REMOVE_CODE = (SerializedTypeID::VL, 9, true, true),
        EXPIRE_CODE = (SerializedTypeID::VL, 10, true, true),
        CREATE_CODE = (SerializedTypeID::VL, 11, true, true),
        MEMO_TYPE = (SerializedTypeID::VL, 12, true, true),
        MEMO_DATA = (SerializedTypeID::VL, 13, true, true),
        MEMO_FORMAT = (SerializedTypeID::VL, 14, true, true),
        FULFILLMENT = (SerializedTypeID::VL, 16, true, true),
        CONDITION = (SerializedTypeID::VL, 17, true, true),
        CLOSE_FLAGS = (SerializedTypeID::UInt8, 8, true, true),
        INVOICE = (SerializedTypeID::VL, 22, true, true),
        TOTAL = (SerializedTypeID::Amount, 23, true, true),
        ISSUED = (SerializedTypeID::Amount, 24, true, true),
        FANS = (SerializedTypeID::UInt32, 25, true, true),
        DECIMAL = (SerializedTypeID::UInt8, 26, true, true),
        INFO = (SerializedTypeID::VL, 27, true, true),
    }
}

/// PathStep represents a single step in a payment path
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PathStep {
    pub account: Option<AccountID>,
    pub currency: Option<Currency>,
    pub issuer: Option<AccountID>,
}

impl PathStep {
    pub fn new() -> Self {
        Self {
            account: None,
            currency: None,
            issuer: None,
        }
    }

    pub fn with_account(mut self, account: AccountID) -> Self {
        self.account = Some(account);
        self
    }

    pub fn with_currency(mut self, currency: Currency) -> Self {
        self.currency = Some(currency);
        self
    }

    pub fn with_issuer(mut self, issuer: AccountID) -> Self {
        self.issuer = Some(issuer);
        self
    }
}

impl Default for PathStep {
    fn default() -> Self {
        Self::new()
    }
}

/// Path represents a sequence of steps for a payment
pub type Path = Vec<PathStep>;

/// PathSet represents multiple possible paths
pub type PathSet = Vec<Path>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum STValue {
    Object(STObject),
    Array(Vec<STValue>),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Hash128(UInt128),
    Hash160(UInt160),
    Hash256(UInt256),
    Amount(Amount),
    VL(Vec<u8>),
    Account(AccountID),
    PathSet(PathSet),
    Vector256(Vec<UInt256>),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Amount {
    pub mantissa: i64,
    pub exponent: i32,
    pub currency: Currency,
    pub issuer: AccountID,
    pub is_native: bool,
    pub is_negative: bool,
}

impl Amount {
    pub const CALL_EXPONENT: i32 = -6;
    pub const MAX_MANTISSA: i64 = 9999999999999999;
    pub const MIN_MANTISSA: i64 = -9999999999999999;

    pub fn call(drops: u64) -> Self {
        Self {
            mantissa: drops as i64,
            exponent: Self::CALL_EXPONENT,
            currency: Currency::CALL,
            issuer: AccountID::new([0u8; 20]),
            is_native: true,
            is_negative: false,
        }
    }

    pub fn issued(
        mantissa: i64,
        exponent: i32,
        currency: Currency,
        issuer: AccountID,
    ) -> Option<Self> {
        if mantissa == 0 {
            return Some(Self {
                mantissa: 0,
                exponent: 0,
                currency,
                issuer,
                is_native: false,
                is_negative: false,
            });
        }

        if mantissa > Self::MAX_MANTISSA || mantissa < Self::MIN_MANTISSA {
            return None;
        }

        Some(Self {
            mantissa,
            exponent,
            currency,
            issuer,
            is_native: false,
            is_negative: mantissa < 0,
        })
    }

    pub fn is_native(&self) -> bool {
        self.is_native
    }

    pub fn is_zero(&self) -> bool {
        self.mantissa == 0
    }

    pub fn get_currency(&self) -> Currency {
        self.currency
    }

    pub fn get_issuer(&self) -> AccountID {
        self.issuer
    }

    pub fn negate(&self) -> Self {
        let mut result = self.clone();
        result.is_negative = !self.is_negative;
        result.mantissa = -result.mantissa;
        result
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct STObject {
    fields: BTreeMap<u32, STValue>,
}

impl STObject {
    pub fn new() -> Self {
        Self {
            fields: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, field: SField, value: STValue) {
        self.fields.insert(field.field_code, value);
    }

    pub fn get(&self, field: SField) -> Option<&STValue> {
        self.fields.get(&field.field_code)
    }

    pub fn get_uint32(&self, field: SField) -> Option<u32> {
        match self.fields.get(&field.field_code) {
            Some(STValue::UInt32(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_uint64(&self, field: SField) -> Option<u64> {
        match self.fields.get(&field.field_code) {
            Some(STValue::UInt64(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_uint8(&self, field: SField) -> Option<u8> {
        match self.fields.get(&field.field_code) {
            Some(STValue::UInt8(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_uint16(&self, field: SField) -> Option<u16> {
        match self.fields.get(&field.field_code) {
            Some(STValue::UInt16(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_hash128(&self, field: SField) -> Option<UInt128> {
        match self.fields.get(&field.field_code) {
            Some(STValue::Hash128(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_hash256(&self, field: SField) -> Option<UInt256> {
        match self.fields.get(&field.field_code) {
            Some(STValue::Hash256(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_account(&self, field: SField) -> Option<AccountID> {
        match self.fields.get(&field.field_code) {
            Some(STValue::Account(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_amount(&self, field: SField) -> Option<Amount> {
        match self.fields.get(&field.field_code) {
            Some(STValue::Amount(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_vl(&self, field: SField) -> Option<&[u8]> {
        match self.fields.get(&field.field_code) {
            Some(STValue::VL(v)) => Some(v),
            _ => None,
        }
    }

    pub fn get_object(&self, field: SField) -> Option<&STObject> {
        match self.fields.get(&field.field_code) {
            Some(STValue::Object(v)) => Some(v),
            _ => None,
        }
    }

    pub fn get_array(&self, field: SField) -> Option<&[STValue]> {
        match self.fields.get(&field.field_code) {
            Some(STValue::Array(v)) => Some(v),
            _ => None,
        }
    }

    pub fn remove(&mut self, field: SField) -> Option<STValue> {
        self.fields.remove(&field.field_code)
    }

    pub fn contains(&self, field: SField) -> bool {
        self.fields.contains_key(&field.field_code)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u32, &STValue)> {
        self.fields.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxType {
    Invalid,
    Payment,
    // 1,2,4 reserved for Escrow (excluded)
    AccountSet,
    SetRegularKey,
    NicknameSet,
    OfferCreate,
    OfferCancel,
    // 10,11 reserved for Ticket (excluded)
    SignerListSet,
    // 13,14,15 reserved for PayChannel (excluded)
    IssueSet,
    TrustSet,
    EnableAmendment,
    SetFee,
}

impl TxType {
    pub const fn as_i16(&self) -> i16 {
        match self {
            Self::Invalid => -1,
            Self::Payment => 0,
            Self::AccountSet => 3,
            Self::SetRegularKey => 5,
            Self::NicknameSet => 6,
            Self::OfferCreate => 7,
            Self::OfferCancel => 8,
            Self::SignerListSet => 12,
            Self::IssueSet => 16,
            Self::TrustSet => 20,
            Self::EnableAmendment => 100,
            Self::SetFee => 101,
        }
    }

    pub fn from_i16(v: i16) -> Option<Self> {
        match v {
            -1 => Some(Self::Invalid),
            0 => Some(Self::Payment),
            3 => Some(Self::AccountSet),
            5 => Some(Self::SetRegularKey),
            6 => Some(Self::NicknameSet),
            7 => Some(Self::OfferCreate),
            8 => Some(Self::OfferCancel),
            12 => Some(Self::SignerListSet),
            16 => Some(Self::IssueSet),
            20 => Some(Self::TrustSet),
            100 => Some(Self::EnableAmendment),
            101 => Some(Self::SetFee),
            // Escrow (1,2,4), Ticket (10,11), PayChannel (13,14,15) excluded
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerEntryType {
    Any,
    Child,
    Invalid,
    AccountRoot,
    DirNode,
    CallState,
    SignerList,
    Offer,
    LedgerHashes,
    Amendments,
    FeeSettings,
    Nickname,
    NotUsed01,
    IssueRoot,
    FeeRoot,
    InvoiceRoot,
}

impl LedgerEntryType {
    pub const fn as_i16(&self) -> i16 {
        match self {
            Self::Any => -3,
            Self::Child => -2,
            Self::Invalid => -1,
            Self::AccountRoot => b'a' as i16,
            Self::DirNode => b'd' as i16,
            Self::CallState => b'r' as i16,
            Self::SignerList => b'S' as i16,
            Self::Offer => b'o' as i16,
            Self::LedgerHashes => b'h' as i16,
            Self::Amendments => b'f' as i16,
            Self::FeeSettings => b's' as i16,
            Self::Nickname => b'n' as i16,
            Self::NotUsed01 => b'c' as i16,
            Self::IssueRoot => b'i' as i16,
            Self::FeeRoot => b'F' as i16,
            Self::InvoiceRoot => b'v' as i16,
        }
    }

    pub fn from_i16(v: i16) -> Option<Self> {
        match v {
            -3 => Some(Self::Any),
            -2 => Some(Self::Child),
            -1 => Some(Self::Invalid),
            97 => Some(Self::AccountRoot),
            100 => Some(Self::DirNode),
            114 => Some(Self::CallState),
            83 => Some(Self::SignerList),
            111 => Some(Self::Offer),
            104 => Some(Self::LedgerHashes),
            102 => Some(Self::Amendments),
            115 => Some(Self::FeeSettings),
            110 => Some(Self::Nickname),
            99 => Some(Self::NotUsed01),
            105 => Some(Self::IssueRoot),
            70 => Some(Self::FeeRoot),
            118 => Some(Self::InvoiceRoot),
            _ => None,
        }
    }
}
