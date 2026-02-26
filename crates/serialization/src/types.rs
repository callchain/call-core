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
        LedgerSequence = (SerializedTypeID::UInt32, 6, true, true),
        CloseTime = (SerializedTypeID::UInt32, 7, true, true),
        ParentCloseTime = (SerializedTypeID::UInt32, 8, true, true),
        SigningTime = (SerializedTypeID::UInt32, 9, true, true),
        Expiration = (SerializedTypeID::UInt32, 10, true, true),
        TransferRate = (SerializedTypeID::UInt32, 11, true, true),
        WalletSize = (SerializedTypeID::UInt32, 12, true, true),
        OwnerCount = (SerializedTypeID::UInt32, 13, true, true),
        DestinationTag = (SerializedTypeID::UInt32, 14, true, true),
        HighQualityIn = (SerializedTypeID::UInt32, 16, true, true),
        HighQualityOut = (SerializedTypeID::UInt32, 17, true, true),
        LowQualityIn = (SerializedTypeID::UInt32, 18, true, true),
        LowQualityOut = (SerializedTypeID::UInt32, 19, true, true),
        QualityIn = (SerializedTypeID::UInt32, 20, true, true),
        QualityOut = (SerializedTypeID::UInt32, 21, true, true),
        Flags = (SerializedTypeID::UInt32, 22, true, true),
        SourceTag = (SerializedTypeID::UInt32, 3, true, true),
        Sequence = (SerializedTypeID::UInt32, 4, true, true),
        PreviousTxnLgrSeq = (SerializedTypeID::UInt32, 5, true, true),
        LedgerEntryType = (SerializedTypeID::UInt16, 1, true, true),
        TransactionType = (SerializedTypeID::UInt16, 2, true, true),
        SignerWeight = (SerializedTypeID::UInt16, 3, true, true),
        Version = (SerializedTypeID::UInt16, 16, true, true),
        Fee = (SerializedTypeID::Amount, 8, true, true),
        SendMax = (SerializedTypeID::Amount, 9, true, true),
        DeliverMin = (SerializedTypeID::Amount, 10, true, true),
        MinimumOffer = (SerializedTypeID::Amount, 16, true, true),
        CALLBalance = (SerializedTypeID::Amount, 2, true, true),
        LowLimit = (SerializedTypeID::Amount, 3, true, true),
        HighLimit = (SerializedTypeID::Amount, 4, true, true),
        TakerPays = (SerializedTypeID::Amount, 5, true, true),
        TakerGets = (SerializedTypeID::Amount, 6, true, true),
        Balance = (SerializedTypeID::Amount, 1, true, true),
        Amount = (SerializedTypeID::Amount, 1, true, true),
        LimitAmount = (SerializedTypeID::Amount, 17, true, true),
        TakerGetPaid = (SerializedTypeID::Amount, 19, true, true),
        TakerPay = (SerializedTypeID::Amount, 20, true, true),
        IndexNext = (SerializedTypeID::UInt64, 1, true, true),
        IndexPrevious = (SerializedTypeID::UInt64, 2, true, true),
        BookNode = (SerializedTypeID::UInt64, 3, true, true),
        OwnerNode = (SerializedTypeID::UInt64, 4, true, true),
        BaseFee = (SerializedTypeID::UInt64, 5, true, true),
        ExchangeRate = (SerializedTypeID::UInt64, 6, true, true),
        LowNode = (SerializedTypeID::UInt64, 7, true, true),
        HighNode = (SerializedTypeID::UInt64, 8, true, true),
        DestinationNode = (SerializedTypeID::UInt64, 9, true, true),
        EmailHash = (SerializedTypeID::Hash128, 2, true, true),
        LedgerHash = (SerializedTypeID::Hash256, 1, true, true),
        ParentHash = (SerializedTypeID::Hash256, 2, true, true),
        TransactionHash = (SerializedTypeID::Hash256, 3, true, true),
        AccountHash = (SerializedTypeID::Hash256, 4, true, true),
        PreviousTxnID = (SerializedTypeID::Hash256, 5, true, true),
        LedgerIndex = (SerializedTypeID::Hash256, 6, true, true),
        WalletLocator = (SerializedTypeID::Hash256, 7, true, true),
        RootIndex = (SerializedTypeID::Hash256, 8, true, true),
        AccountTxnID = (SerializedTypeID::Hash256, 9, true, true),
        BookDirectory = (SerializedTypeID::Hash256, 16, true, true),
        InvoiceID = (SerializedTypeID::Hash256, 17, true, true),
        Nickname = (SerializedTypeID::Hash256, 18, true, true),
        Feature = (SerializedTypeID::Hash256, 19, true, true),
        Amendment = (SerializedTypeID::Hash256, 20, true, true),
        TicketID = (SerializedTypeID::Hash256, 21, true, true),
        Digest = (SerializedTypeID::Hash256, 22, true, true),
        Channel = (SerializedTypeID::Hash256, 23, true, true),
        ConsensusHash = (SerializedTypeID::Hash256, 24, true, true),
        CheckID = (SerializedTypeID::Hash256, 25, true, true),
        ValidatedHash = (SerializedTypeID::Hash256, 26, true, true),
        ChallengeNode = (SerializedTypeID::Hash256, 27, true, true),
        Address = (SerializedTypeID::Account, 1, true, true),
        BalanceOwner = (SerializedTypeID::Account, 2, true, true),
        RegularKey = (SerializedTypeID::Account, 8, true, true),
        Authorize = (SerializedTypeID::Account, 9, true, true),
        Unauthorize = (SerializedTypeID::Account, 10, true, true),
        Destination = (SerializedTypeID::Account, 3, true, true),
        Issuer = (SerializedTypeID::Account, 4, true, true),
        Target = (SerializedTypeID::Account, 7, true, true),
        Account = (SerializedTypeID::Account, 1, true, true),
        ObjectEndMarker = (SerializedTypeID::Object, 1, true, true),
        TransactionMetaData = (SerializedTypeID::Object, 2, true, true),
        CreatedNode = (SerializedTypeID::Object, 3, true, true),
        DeletedNode = (SerializedTypeID::Object, 4, true, true),
        ModifiedNode = (SerializedTypeID::Object, 5, true, true),
        PreviousFields = (SerializedTypeID::Object, 6, true, true),
        FinalFields = (SerializedTypeID::Object, 7, true, true),
        NewFields = (SerializedTypeID::Object, 8, true, true),
        TemplateEntry = (SerializedTypeID::Object, 9, true, true),
        SignerEntry = (SerializedTypeID::Object, 10, true, true),
        Signer = (SerializedTypeID::Object, 11, true, true),
        Majority = (SerializedTypeID::Object, 16, true, true),
        DisabledValidator = (SerializedTypeID::Object, 17, true, true),
        EmittedDetails = (SerializedTypeID::Object, 18, true, true),
        ArrayEndMarker = (SerializedTypeID::Array, 1, true, true),
        SigningAccounts = (SerializedTypeID::Array, 2, true, true),
        TxnSignatures = (SerializedTypeID::Array, 3, true, true),
        Signatures = (SerializedTypeID::Array, 4, true, true),
        Template = (SerializedTypeID::Array, 5, true, true),
        Necessary = (SerializedTypeID::Array, 6, true, true),
        Sufficient = (SerializedTypeID::Array, 7, true, true),
        AffectedNodes = (SerializedTypeID::Array, 8, true, true),
        Memos = (SerializedTypeID::Array, 9, true, true),
        SignerEntries = (SerializedTypeID::Array, 10, true, true),
        Signers = (SerializedTypeID::Array, 11, true, true),
        Majorities = (SerializedTypeID::Array, 16, true, true),
        DisabledValidators = (SerializedTypeID::Array, 17, true, true),
        EmittedTxn = (SerializedTypeID::Array, 18, true, true),
        HookExecution = (SerializedTypeID::Array, 19, true, true),
        HookExecutions = (SerializedTypeID::Array, 20, true, true),
        HookParameter = (SerializedTypeID::Array, 21, true, true),
        HookParameters = (SerializedTypeID::Array, 22, true, true),
        HookGrant = (SerializedTypeID::Array, 23, true, true),
        HookGrants = (SerializedTypeID::Array, 24, true, true),
        Hooks = (SerializedTypeID::Array, 25, true, true),
        Paths = (SerializedTypeID::PathSet, 1, true, true),
        CloseResolution = (SerializedTypeID::UInt8, 1, true, true),
        Method = (SerializedTypeID::UInt8, 2, true, true),
        TransactionResult = (SerializedTypeID::UInt8, 3, true, true),
        TakerPaysCurrency = (SerializedTypeID::Hash160, 1, true, true),
        TakerPaysIssuer = (SerializedTypeID::Hash160, 2, true, true),
        TakerGetsCurrency = (SerializedTypeID::Hash160, 3, true, true),
        TakerGetsIssuer = (SerializedTypeID::Hash160, 4, true, true),
        PathsCanonical = (SerializedTypeID::PathSet, 2, true, true),
        PathsSet = (SerializedTypeID::PathSet, 1, true, true),
        Indexes = (SerializedTypeID::Vector256, 1, true, true),
        Hashes = (SerializedTypeID::Vector256, 2, true, true),
        Features = (SerializedTypeID::Vector256, 3, true, true),
        Transactions = (SerializedTypeID::Vector256, 4, true, true),
        Amendments = (SerializedTypeID::Vector256, 5, true, true),
        TicketSequence = (SerializedTypeID::UInt32, 40, true, true),
        TicketCount = (SerializedTypeID::UInt16, 40, true, true),
        SignerListID = (SerializedTypeID::UInt32, 41, true, true),
        SetFlag = (SerializedTypeID::UInt32, 33, true, true),
        ClearFlag = (SerializedTypeID::UInt32, 34, true, true),
        SignerQuorum = (SerializedTypeID::UInt32, 35, true, true),
        CancelAfter = (SerializedTypeID::UInt32, 36, true, true),
        FinishAfter = (SerializedTypeID::UInt32, 37, true, true),
        SignerListSequence = (SerializedTypeID::UInt32, 38, true, true),
        BurnedNFTokens = (SerializedTypeID::UInt32, 42, true, true),
        MintedTokens = (SerializedTypeID::UInt32, 43, true, true),
        HookStateCount = (SerializedTypeID::UInt32, 45, true, true),
        EmitGeneration = (SerializedTypeID::UInt32, 46, true, true),
        HookExecutionIndex = (SerializedTypeID::UInt16, 16, true, true),
        HookApiVersion = (SerializedTypeID::UInt16, 17, true, true),
        OperationLimit = (SerializedTypeID::UInt16, 44, true, true),
        ReferenceFeeUnits = (SerializedTypeID::UInt16, 8, true, true),
        ReserveBase = (SerializedTypeID::UInt32, 31, true, true),
        ReserveIncrement = (SerializedTypeID::UInt32, 32, true, true),
        HookOn = (SerializedTypeID::UInt64, 16, true, true),
        HookInstructionCount = (SerializedTypeID::UInt64, 17, true, true),
        EmitBurden = (SerializedTypeID::UInt64, 18, true, true),
        HookReturnCode = (SerializedTypeID::UInt64, 19, true, true),
        HookReturnString = (SerializedTypeID::VL, 17, true, true),
        HookNamespace = (SerializedTypeID::Hash256, 16, true, true),
        HookSetTxnID = (SerializedTypeID::Hash256, 18, true, true),
        HookParameterName = (SerializedTypeID::VL, 18, true, true),
        HookParameterValue = (SerializedTypeID::VL, 19, true, true),
        HookHash = (SerializedTypeID::Hash256, 19, true, true),
        HookGrantAuthorization = (SerializedTypeID::Account, 16, true, true),
        HookGrantAuthorize = (SerializedTypeID::Account, 17, true, true),
        HookStateKey = (SerializedTypeID::VL, 20, true, true),
        HookStateData = (SerializedTypeID::VL, 21, true, true),
        PublicKey = (SerializedTypeID::VL, 1, true, true),
        MessageKey = (SerializedTypeID::VL, 2, true, true),
        SigningPubKey = (SerializedTypeID::VL, 3, true, true),
        TxnSignature = (SerializedTypeID::VL, 4, false, true),
        Signature = (SerializedTypeID::VL, 6, true, true),
        Domain = (SerializedTypeID::VL, 7, true, true),
        FundCode = (SerializedTypeID::VL, 8, true, true),
        RemoveCode = (SerializedTypeID::VL, 9, true, true),
        ExpireCode = (SerializedTypeID::VL, 10, true, true),
        CreateCode = (SerializedTypeID::VL, 11, true, true),
        MemoType = (SerializedTypeID::VL, 12, true, true),
        MemoData = (SerializedTypeID::VL, 13, true, true),
        MemoFormat = (SerializedTypeID::VL, 14, true, true),
        Fulfillment = (SerializedTypeID::VL, 16, true, true),
        Condition = (SerializedTypeID::VL, 17, true, true),
        CloseFlags = (SerializedTypeID::UInt8, 8, true, true),
        Invoice = (SerializedTypeID::VL, 22, true, true),
        Total = (SerializedTypeID::Amount, 23, true, true),
        Issued = (SerializedTypeID::Amount, 24, true, true),
        Fans = (SerializedTypeID::UInt32, 25, true, true),
        Decimal = (SerializedTypeID::UInt8, 26, true, true),
        Info = (SerializedTypeID::VL, 27, true, true),
    }
}

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
    PathSet(Vec<Vec<PathStep>>),
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PathStep {
    pub account: Option<AccountID>,
    pub currency: Option<Currency>,
    pub issuer: Option<AccountID>,
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
    EscrowCreate,
    EscrowFinish,
    AccountSet,
    EscrowCancel,
    RegularKeySet,
    NicknameSet,
    OfferCreate,
    OfferCancel,
    TicketCreate,
    TicketCancel,
    SignerListSet,
    PaychanCreate,
    PaychanFund,
    PaychanClaim,
    IssueSet,
    TrustSet,
    Amendment,
    Fee,
}

impl TxType {
    pub const fn as_i16(&self) -> i16 {
        match self {
            Self::Invalid => -1,
            Self::Payment => 0,
            Self::EscrowCreate => 1,
            Self::EscrowFinish => 2,
            Self::AccountSet => 3,
            Self::EscrowCancel => 4,
            Self::RegularKeySet => 5,
            Self::NicknameSet => 6,
            Self::OfferCreate => 7,
            Self::OfferCancel => 8,
            Self::TicketCreate => 10,
            Self::TicketCancel => 11,
            Self::SignerListSet => 12,
            Self::PaychanCreate => 13,
            Self::PaychanFund => 14,
            Self::PaychanClaim => 15,
            Self::IssueSet => 16,
            Self::TrustSet => 20,
            Self::Amendment => 100,
            Self::Fee => 101,
        }
    }

    pub fn from_i16(v: i16) -> Option<Self> {
        match v {
            -1 => Some(Self::Invalid),
            0 => Some(Self::Payment),
            1 => Some(Self::EscrowCreate),
            2 => Some(Self::EscrowFinish),
            3 => Some(Self::AccountSet),
            4 => Some(Self::EscrowCancel),
            5 => Some(Self::RegularKeySet),
            6 => Some(Self::NicknameSet),
            7 => Some(Self::OfferCreate),
            8 => Some(Self::OfferCancel),
            10 => Some(Self::TicketCreate),
            11 => Some(Self::TicketCancel),
            12 => Some(Self::SignerListSet),
            13 => Some(Self::PaychanCreate),
            14 => Some(Self::PaychanFund),
            15 => Some(Self::PaychanClaim),
            16 => Some(Self::IssueSet),
            20 => Some(Self::TrustSet),
            100 => Some(Self::Amendment),
            101 => Some(Self::Fee),
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
    Ticket,
    SignerList,
    Offer,
    LedgerHashes,
    Amendments,
    FeeSettings,
    Escrow,
    PayChannel,
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
            Self::Ticket => b'T' as i16,
            Self::SignerList => b'S' as i16,
            Self::Offer => b'o' as i16,
            Self::LedgerHashes => b'h' as i16,
            Self::Amendments => b'f' as i16,
            Self::FeeSettings => b's' as i16,
            Self::Escrow => b'u' as i16,
            Self::PayChannel => b'x' as i16,
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
            84 => Some(Self::Ticket),
            83 => Some(Self::SignerList),
            111 => Some(Self::Offer),
            104 => Some(Self::LedgerHashes),
            102 => Some(Self::Amendments),
            115 => Some(Self::FeeSettings),
            117 => Some(Self::Escrow),
            120 => Some(Self::PayChannel),
            110 => Some(Self::Nickname),
            99 => Some(Self::NotUsed01),
            105 => Some(Self::IssueRoot),
            70 => Some(Self::FeeRoot),
            118 => Some(Self::InvoiceRoot),
            _ => None,
        }
    }
}
