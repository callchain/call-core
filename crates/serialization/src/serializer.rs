use crate::types::{Amount, SerializedTypeID, SField, STObject, STValue};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use primitives::{AccountID, Currency, UInt128, UInt160, UInt256};
use std::io::{self, Cursor, Read};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SerializeError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Invalid field ID")]
    InvalidFieldId,
    #[error("Invalid type")]
    InvalidType,
    #[error("Invalid length")]
    InvalidLength,
}

pub struct Serializer {
    buffer: Vec<u8>,
}

impl Default for Serializer {
    fn default() -> Self {
        Self::new()
    }
}

impl Serializer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(1024),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    pub fn add8(&mut self, value: u8) -> &mut Self {
        self.buffer.write_u8(value).unwrap();
        self
    }

    pub fn add16(&mut self, value: u16) -> &mut Self {
        self.buffer.write_u16::<BigEndian>(value).unwrap();
        self
    }

    pub fn add32(&mut self, value: u32) -> &mut Self {
        self.buffer.write_u32::<BigEndian>(value).unwrap();
        self
    }

    pub fn add64(&mut self, value: u64) -> &mut Self {
        self.buffer.write_u64::<BigEndian>(value).unwrap();
        self
    }

    pub fn add128(&mut self, value: UInt128) -> &mut Self {
        self.buffer.extend_from_slice(value.as_bytes());
        self
    }

    pub fn add160(&mut self, value: UInt160) -> &mut Self {
        self.buffer.extend_from_slice(value.as_bytes());
        self
    }

    pub fn add256(&mut self, value: UInt256) -> &mut Self {
        self.buffer.extend_from_slice(value.as_bytes());
        self
    }

    pub fn add_field_id(&mut self, type_id: SerializedTypeID, field_num: u16) -> &mut Self {
        let type_val = type_id as i16;
        self.encode_field_id(type_val, field_num);
        self
    }

    fn encode_field_id(&mut self, type_val: i16, field_num: u16) {
        let field_val = field_num;

        if (type_val < 16) && (field_val < 16) {
            self.add8(((type_val as u8) << 4) | (field_val as u8));
        } else if type_val < 16 {
            self.add8((type_val as u8) << 4);
            self.add8(field_val as u8);
        } else if field_val < 16 {
            self.add8(field_val as u8);
            self.add8(type_val as u8);
        } else {
            self.add8(0);
            self.add8(type_val as u8);
            self.add8(field_val as u8);
        }
    }

    pub fn add_vl(&mut self, data: &[u8]) -> &mut Self {
        self.encode_vl_length(data.len());
        self.buffer.extend_from_slice(data);
        self
    }

    fn encode_vl_length(&mut self, len: usize) {
        if len <= 192 {
            self.add8(len as u8);
        } else if len <= 12480 {
            let len_minus_193 = len - 193;
            self.add8(193 + (len_minus_193 / 256) as u8);
            self.add8((len_minus_193 % 256) as u8);
        } else if len <= 918744 {
            let len_minus_12481 = len - 12481;
            self.add8(241 + (len_minus_12481 / 65536) as u8);
            self.add8(((len_minus_12481 / 256) % 256) as u8);
            self.add8((len_minus_12481 % 256) as u8);
        } else {
            panic!("VL length too large: {}", len);
        }
    }

    pub fn add_amount(&mut self, amount: Amount) -> &mut Self {
        if amount.is_native {
            self.add_native_amount(amount);
        } else {
            self.add_issued_amount(amount);
        }
        self
    }

    fn add_native_amount(&mut self, amount: Amount) {
        let mantissa = amount.mantissa;
        let is_negative = mantissa < 0 || amount.is_negative;

        let mut value = mantissa.abs() as u64;
        if is_negative {
            value |= 1u64 << 62;
        }

        self.add64(value);
    }

    fn add_issued_amount(&mut self, amount: Amount) {
        let is_negative = amount.mantissa < 0 || amount.is_negative;
        let mantissa_abs = amount.mantissa.abs() as u64;

        let mut encoded: u64 = 1u64 << 63;
        if is_negative {
            encoded |= 1u64 << 62;
        }

        let exponent_biased = (amount.exponent + 97) as u64;
        encoded |= (exponent_biased & 0xFF) << 54;
        encoded |= mantissa_abs & 0x3FFFFFFFFFFFFF;

        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&encoded.to_be_bytes());

        self.buffer.extend_from_slice(&bytes);
        self.buffer.extend_from_slice(amount.currency.as_bytes());
        self.buffer.extend_from_slice(amount.issuer.as_bytes());
    }

    pub fn add_account(&mut self, account: AccountID) -> &mut Self {
        let bytes = account.as_bytes();
        let leading_zeros = bytes.iter().take_while(|&&b| b == 0).count();
        self.add_vl(&bytes[leading_zeros..]);
        self
    }

    pub fn add_currency(&mut self, currency: Currency) -> &mut Self {
        if currency.is_call() {
            self.buffer.extend_from_slice(&[0u8; 20]);
        } else {
            self.buffer.extend_from_slice(currency.as_bytes());
        }
        self
    }

    pub fn add_value(&mut self, field: SField, value: &STValue) -> Result<(), SerializeError> {
        self.encode_field_id(field.type_id as i16, (field.field_code & 0xFFFF) as u16);

        match (field.type_id, value) {
            (SerializedTypeID::UInt8, STValue::UInt8(v)) => self.add8(*v),
            (SerializedTypeID::UInt16, STValue::UInt16(v)) => self.add16(*v),
            (SerializedTypeID::UInt32, STValue::UInt32(v)) => self.add32(*v),
            (SerializedTypeID::UInt64, STValue::UInt64(v)) => self.add64(*v),
            (SerializedTypeID::Hash128, STValue::Hash128(v)) => self.add128(*v),
            (SerializedTypeID::Hash160, STValue::Hash160(v)) => self.add160(*v),
            (SerializedTypeID::Hash256, STValue::Hash256(v)) => self.add256(*v),
            (SerializedTypeID::Amount, STValue::Amount(v)) => self.add_amount(*v),
            (SerializedTypeID::VL, STValue::VL(v)) => self.add_vl(v),
            (SerializedTypeID::Account, STValue::Account(v)) => self.add_account(*v),
            (SerializedTypeID::Object, STValue::Object(v)) => {
                for (field_code, val) in v.iter() {
                    let field = self.field_from_code(*field_code)?;
                    self.add_value(field, val)?;
                }
                self.encode_field_id(SerializedTypeID::Object as i16, 1);
                return Ok(());
            }
            (SerializedTypeID::Array, STValue::Array(v)) => {
                for val in v.iter() {
                    self.encode_field_id(SerializedTypeID::Object as i16, 1);
                    if let STValue::Object(obj) = val {
                        for (field_code, field_val) in obj.iter() {
                            let field = self.field_from_code(*field_code)?;
                            self.add_value(field, field_val)?;
                        }
                    }
                    self.encode_field_id(SerializedTypeID::Object as i16, 1);
                }
                self.encode_field_id(SerializedTypeID::Array as i16, 1);
                return Ok(());
            }
            (SerializedTypeID::Vector256, STValue::Vector256(v)) => {
                let mut data = Vec::with_capacity(v.len() * 32);
                for hash in v.iter() {
                    data.extend_from_slice(hash.as_bytes());
                }
                self.add_vl(&data);
                return Ok(());
            }
            _ => return Err(SerializeError::InvalidType),
        };

        Ok(())
    }

    fn field_from_code(&self, code: u32) -> Result<SField, SerializeError> {
        use crate::types::sf;

        static FIELDS: &[SField] = &[
            sf::LEDGER_SEQUENCE,
            sf::CLOSE_TIME,
            sf::PARENT_CLOSE_TIME,
            sf::SIGNING_TIME,
            sf::EXPIRATION,
            sf::TRANSFER_RATE,
            sf::WALLET_SIZE,
            sf::OWNER_COUNT,
            sf::DESTINATION_TAG,
            sf::HIGH_QUALITY_IN,
            sf::HIGH_QUALITY_OUT,
            sf::LOW_QUALITY_IN,
            sf::LOW_QUALITY_OUT,
            sf::QUALITY_IN,
            sf::QUALITY_OUT,
            sf::FLAGS,
            sf::SOURCE_TAG,
            sf::SEQUENCE,
            sf::PREVIOUS_TXN_LGR_SEQ,
            sf::LEDGER_ENTRY_TYPE,
            sf::TRANSACTION_TYPE,
            sf::SIGNER_WEIGHT,
            sf::VERSION,
            sf::FEE,
            sf::SEND_MAX,
            sf::DELIVER_MIN,
            sf::MINIMUM_OFFER,
            sf::CALL_BALANCE,
            sf::LOW_LIMIT,
            sf::HIGH_LIMIT,
            sf::TAKER_PAYS,
            sf::TAKER_GETS,
            sf::TOTAL_SUPPLY,
            sf::BALANCE,
            sf::AMOUNT,
            sf::LIMIT_AMOUNT,
            sf::TAKER_GET_PAID,
            sf::TAKER_PAY,
            sf::INDEX_NEXT,
            sf::INDEX_PREVIOUS,
            sf::BOOK_NODE,
            sf::OWNER_NODE,
            sf::BASE_FEE,
            sf::EXCHANGE_RATE,
            sf::LOW_NODE,
            sf::HIGH_NODE,
            sf::DESTINATION_NODE,
            sf::EMAIL_HASH,
            sf::LEDGER_HASH,
            sf::PARENT_HASH,
            sf::TRANSACTION_HASH,
            sf::ACCOUNT_HASH,
            sf::PREVIOUS_TXN_ID,
            sf::LEDGER_INDEX,
            sf::WALLET_LOCATOR,
            sf::ROOT_INDEX,
            sf::ACCOUNT_TXN_ID,
            sf::BOOK_DIRECTORY,
            sf::INVOICE_ID,
            sf::NICKNAME,
            sf::FEATURE,
            sf::AMENDMENT,
            sf::DIGEST,
            sf::CONSENSUS_HASH,
            sf::CHECK_ID,
            sf::VALIDATED_HASH,
            sf::CHALLENGE_NODE,
            sf::ADDRESS,
            sf::BALANCE_OWNER,
            sf::REGULAR_KEY,
            sf::AUTHORIZE,
            sf::UNAUTHORIZE,
            sf::DESTINATION,
            sf::ISSUER,
            sf::TARGET,
            sf::ACCOUNT,
            sf::OBJECT_END_MARKER,
            sf::TRANSACTION_META_DATA,
            sf::CREATED_NODE,
            sf::DELETED_NODE,
            sf::MODIFIED_NODE,
            sf::PREVIOUS_FIELDS,
            sf::FINAL_FIELDS,
            sf::NEW_FIELDS,
            sf::TEMPLATE_ENTRY,
            sf::SIGNER_ENTRY,
            sf::SIGNER,
            sf::MAJORITY,
            sf::DISABLED_VALIDATOR,
            sf::EMITTED_DETAILS,
            sf::ARRAY_END_MARKER,
            sf::SIGNING_ACCOUNTS,
            sf::TXN_SIGNATURES,
            sf::SIGNATURES,
            sf::TEMPLATE,
            sf::NECESSARY,
            sf::SUFFICIENT,
            sf::AFFECTED_NODES,
            sf::MEMOS,
            sf::SIGNER_ENTRIES,
            sf::SIGNERS,
            sf::MAJORITIES,
            sf::DISABLED_VALIDATORS,
            sf::EMITTED_TXN,
            sf::HOOK_EXECUTION,
            sf::HOOK_EXECUTIONS,
            sf::HOOK_PARAMETER,
            sf::HOOK_PARAMETERS,
            sf::HOOK_GRANT,
            sf::HOOK_GRANTS,
            sf::HOOKS,
            sf::PATHS,
            sf::CLOSE_RESOLUTION,
            sf::METHOD,
            sf::TRANSACTION_RESULT,
            sf::TAKER_PAYS_CURRENCY,
            sf::TAKER_PAYS_ISSUER,
            sf::TAKER_GETS_CURRENCY,
            sf::TAKER_GETS_ISSUER,
            sf::PATHS_CANONICAL,
            sf::PATHS_SET,
            sf::INDEXES,
            sf::HASHES,
            sf::FEATURES,
            sf::TRANSACTIONS,
            sf::SIGNER_LIST_ID,
            sf::SET_FLAG,
            sf::CLEAR_FLAG,
            sf::SIGNER_QUORUM,
            sf::SIGNER_LIST_SEQUENCE,
            sf::BURNED_NF_TOKENS,
            sf::MINTED_TOKENS,
            sf::HOOK_STATE_COUNT,
            sf::EMIT_GENERATION,
            sf::HOOK_EXECUTION_INDEX,
            sf::HOOK_API_VERSION,
            sf::OPERATION_LIMIT,
            sf::REFERENCE_FEE_UNITS,
            sf::RESERVE_BASE,
            sf::RESERVE_INCREMENT,
            sf::HOOK_ON,
            sf::HOOK_INSTRUCTION_COUNT,
            sf::EMIT_BURDEN,
            sf::HOOK_RETURN_CODE,
            sf::HOOK_RETURN_STRING,
            sf::HOOK_NAMESPACE,
            sf::HOOK_SET_TXN_ID,
            sf::HOOK_PARAMETER_NAME,
            sf::HOOK_PARAMETER_VALUE,
            sf::HOOK_HASH,
            sf::HOOK_GRANT_AUTHORIZATION,
            sf::HOOK_GRANT_AUTHORIZE,
            sf::HOOK_STATE_KEY,
            sf::HOOK_STATE_DATA,
            sf::PUBLIC_KEY,
            sf::MESSAGE_KEY,
            sf::SIGNING_PUB_KEY,
            sf::TXN_SIGNATURE,
            sf::SIGNATURE,
            sf::DOMAIN,
            sf::FUND_CODE,
            sf::REMOVE_CODE,
            sf::EXPIRE_CODE,
            sf::CREATE_CODE,
            sf::MEMO_TYPE,
            sf::MEMO_DATA,
            sf::MEMO_FORMAT,
            sf::FULFILLMENT,
            sf::CONDITION,
            sf::CLOSE_FLAGS,
            sf::INVOICE,
            sf::TOTAL,
            sf::ISSUED,
            sf::FANS,
            sf::DECIMAL,
            sf::INFO,
        ];

        for field in FIELDS.iter() {
            if field.field_code == code {
                return Ok(*field);
            }
        }

        Err(SerializeError::InvalidFieldId)
    }

    pub fn add_object(&mut self, obj: &STObject) -> Result<(), SerializeError> {
        for (field_code, value) in obj.iter() {
            let field = self.field_from_code(*field_code)?;
            self.add_value(field, value)?;
        }
        Ok(())
    }

    pub fn finish(self) -> Vec<u8> {
        self.buffer
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

pub struct SerialIter<'a> {
    cursor: Cursor<&'a [u8]>,
}

impl<'a> SerialIter<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            cursor: Cursor::new(data),
        }
    }

    pub fn get8(&mut self) -> Result<u8, SerializeError> {
        Ok(self.cursor.read_u8()?)
    }

    pub fn get16(&mut self) -> Result<u16, SerializeError> {
        Ok(self.cursor.read_u16::<BigEndian>()?)
    }

    pub fn get32(&mut self) -> Result<u32, SerializeError> {
        Ok(self.cursor.read_u32::<BigEndian>()?)
    }

    pub fn get64(&mut self) -> Result<u64, SerializeError> {
        Ok(self.cursor.read_u64::<BigEndian>()?)
    }

    pub fn get128(&mut self) -> Result<UInt128, SerializeError> {
        let mut bytes = [0u8; 16];
        self.cursor.read_exact(&mut bytes)?;
        Ok(UInt128::new(bytes))
    }

    pub fn get160(&mut self) -> Result<UInt160, SerializeError> {
        let mut bytes = [0u8; 20];
        self.cursor.read_exact(&mut bytes)?;
        Ok(UInt160::new(bytes))
    }

    pub fn get256(&mut self) -> Result<UInt256, SerializeError> {
        let mut bytes = [0u8; 32];
        self.cursor.read_exact(&mut bytes)?;
        Ok(UInt256::new(bytes))
    }

    pub fn get_field_id(&mut self) -> Result<(i16, u16), SerializeError> {
        let b1 = self.get8()?;

        let (type_bits, field_bits) = if b1 == 0 {
            let type_val = self.get8()? as i16;
            let field_num = self.get8()? as u16;
            return Ok((type_val, field_num));
        } else {
            ((b1 >> 4) as i16, (b1 & 0x0f) as u16)
        };

        let type_val = if type_bits < 0 || type_bits >= 16 {
            self.get8()? as i16
        } else {
            type_bits
        };

        let field_num = if field_bits > 0 && field_bits < 16 {
            field_bits
        } else {
            self.get8()? as u16
        };

        Ok((type_val, field_num))
    }

    pub fn get_vl(&mut self) -> Result<Vec<u8>, SerializeError> {
        let len = self.decode_vl_length()?;
        let mut buf = vec![0u8; len];
        self.cursor.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn decode_vl_length(&mut self) -> Result<usize, SerializeError> {
        let b1 = self.get8()? as usize;

        if b1 <= 192 {
            Ok(b1)
        } else if b1 <= 193 + (12480 - 193) / 256 {
            let b2 = self.get8()? as usize;
            Ok(193 + (b1 - 193) * 256 + b2)
        } else if b1 <= 241 + (918744 - 12481) / 65536 {
            let b2 = self.get8()? as usize;
            let b3 = self.get8()? as usize;
            Ok(12481 + (b1 - 241) * 65536 + b2 * 256 + b3)
        } else {
            Err(SerializeError::InvalidLength)
        }
    }

    pub fn get_amount(&mut self) -> Result<Amount, SerializeError> {
        let mut bytes = [0u8; 8];
        self.cursor.read_exact(&mut bytes)?;
        let encoded = u64::from_be_bytes(bytes);

        let is_issued = (encoded >> 63) != 0;

        if !is_issued {
            let is_negative = (encoded >> 62) != 0;
            let mantissa = (encoded & 0x3FFFFFFFFFFFFFFF) as i64;
            let mantissa = if is_negative { -mantissa } else { mantissa };

            Ok(Amount {
                mantissa,
                exponent: Amount::CALL_EXPONENT,
                currency: Currency::CALL,
                issuer: AccountID::new([0u8; 20]),
                is_native: true,
                is_negative,
            })
        } else {
            let is_negative = (encoded >> 62) != 0;
            let exponent = ((encoded >> 54) & 0xFF) as i32 - 97;
            let mantissa = (encoded & 0x3FFFFFFFFFFFFF) as i64;
            let mantissa = if is_negative { -mantissa } else { mantissa };

            let currency = Currency::new(*self.get160()?.as_bytes());
            let issuer = AccountID::new(*self.get160()?.as_bytes());

            Ok(Amount {
                mantissa,
                exponent,
                currency,
                issuer,
                is_native: false,
                is_negative,
            })
        }
    }

    pub fn get_account(&mut self) -> Result<AccountID, SerializeError> {
        let data = self.get_vl()?;
        let mut full = [0u8; 20];
        let start = 20 - data.len();
        full[start..].copy_from_slice(&data);
        Ok(AccountID::new(full))
    }

    pub fn get_currency(&mut self) -> Result<Currency, SerializeError> {
        let bytes = self.get160()?;
        Ok(Currency::new(*bytes.as_bytes()))
    }

    pub fn remaining(&self) -> usize {
        let pos = self.cursor.position() as usize;
        let len = self.cursor.get_ref().len();
        len - pos
    }

    pub fn eof(&self) -> bool {
        self.remaining() == 0
    }

    pub fn position(&self) -> u64 {
        self.cursor.position()
    }

    pub fn set_position(&mut self, pos: u64) {
        self.cursor.set_position(pos);
    }

    /// Skip a field of the given type without reading its value
    pub fn skip_field(&mut self, type_id: i16) -> Result<(), SerializeError> {
        use crate::types::SerializedTypeID;

        match SerializedTypeID::try_from(type_id) {
            Ok(SerializedTypeID::UInt8) => { self.get8()?; }
            Ok(SerializedTypeID::UInt16) => { self.get16()?; }
            Ok(SerializedTypeID::UInt32) => { self.get32()?; }
            Ok(SerializedTypeID::UInt64) => { self.get64()?; }
            Ok(SerializedTypeID::Hash128) => { self.get128()?; }
            Ok(SerializedTypeID::Hash160) => { self.get160()?; }
            Ok(SerializedTypeID::Hash256) => { self.get256()?; }
            Ok(SerializedTypeID::VL) => { self.get_vl()?; }
            Ok(SerializedTypeID::Amount) => { self.get_amount()?; }
            Ok(SerializedTypeID::Account) => { self.get_account()?; }
            Ok(SerializedTypeID::Object) | Ok(SerializedTypeID::Array) => {
                // For nested objects/arrays, we need to parse them to skip properly
                return Err(SerializeError::InvalidType);
            }
            _ => {
                return Err(SerializeError::InvalidType);
            }
        }
        Ok(())
    }

    /// Parse an STObject from the serialized data.
    /// This reads field ID/value pairs until the end of the object (marked by Object end).
    pub fn get_object(&mut self) -> Result<STObject, SerializeError> {
        use crate::types::{SerializedTypeID, STObject, STValue};

        let mut obj = STObject::new();

        while !self.eof() {
            // Peek at the next byte to check for object end (0xf9 for STObject with field 1)
            // In practice, we need to detect the end marker
            let pos = self.position();

            // Try to read field ID
            let (type_id, field_num) = match self.get_field_id() {
                Ok(t) => t,
                Err(_) => {
                    // Reset position and return what we have
                    self.set_position(pos);
                    break;
                }
            };

            // Check for special end-of-object marker
            // Object end is typically indicated by specific type codes
            if type_id == 0 && field_num == 0 {
                break;
            }

            // Parse the value based on type
            let value = match SerializedTypeID::try_from(type_id) {
                Ok(SerializedTypeID::UInt16) => STValue::UInt16(self.get16()?),
                Ok(SerializedTypeID::UInt32) => STValue::UInt32(self.get32()?),
                Ok(SerializedTypeID::UInt64) => STValue::UInt64(self.get64()?),
                Ok(SerializedTypeID::UInt8) => STValue::UInt8(self.get8()?),
                Ok(SerializedTypeID::Hash128) => STValue::Hash128(self.get128()?),
                Ok(SerializedTypeID::Hash160) => STValue::Hash160(self.get160()?),
                Ok(SerializedTypeID::Hash256) => STValue::Hash256(self.get256()?),
                Ok(SerializedTypeID::Amount) => STValue::Amount(self.get_amount()?),
                Ok(SerializedTypeID::VL) => STValue::VL(self.get_vl()?),
                Ok(SerializedTypeID::Account) => STValue::Account(self.get_account()?),
                Ok(SerializedTypeID::Vector256) => {
                    // Read vector of 256-bit hashes
                    let data = self.get_vl()?;
                    let mut hashes = Vec::new();
                    for chunk in data.chunks_exact(32) {
                        let mut bytes = [0u8; 32];
                        bytes.copy_from_slice(chunk);
                        hashes.push(UInt256::new(bytes));
                    }
                    STValue::Vector256(hashes)
                }
                Ok(SerializedTypeID::Object) => {
                    // Nested object - skip for now
                    continue;
                }
                Ok(SerializedTypeID::Array) => {
                    // Array - skip for now
                    continue;
                }
                _ => {
                    return Err(SerializeError::InvalidType);
                }
            };

            // Create field code and insert
            let field_code = ((type_id as u32) << 16) | (field_num as u32);
            obj.insert_raw(field_code, value);
        }

        Ok(obj)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_id_encoding() {
        let mut ser = Serializer::new();
        ser.add_field_id(SerializedTypeID::UInt32, 4);
        let data = ser.finish();
        assert_eq!(data, vec![0x24]);
    }

    #[test]
    fn test_vl_length_encoding() {
        let mut ser = Serializer::new();
        ser.add_vl(&vec![0u8; 100]);
        let data = ser.finish();
        assert_eq!(data[0], 100);
        assert_eq!(data.len(), 101);
    }

    #[test]
    fn test_native_amount() {
        let amount = Amount::call(1000000);
        let mut ser = Serializer::new();
        ser.add_amount(amount);
        let data = ser.finish();
        assert_eq!(data.len(), 8);

        let mut iter = SerialIter::new(&data);
        let decoded = iter.get_amount().unwrap();
        assert_eq!(decoded.mantissa, 1000000);
        assert!(decoded.is_native);
    }

    #[test]
    fn test_issued_amount() {
        let currency = Currency::new([0x01; 20]);
        let issuer = AccountID::new([0x02; 20]);
        let amount = Amount::issued(1000000, -2, currency, issuer).unwrap();

        let mut ser = Serializer::new();
        ser.add_amount(amount);
        let data = ser.finish();
        assert_eq!(data.len(), 48);

        let mut iter = SerialIter::new(&data);
        let decoded = iter.get_amount().unwrap();
        assert!(!decoded.is_native);
        assert_eq!(decoded.mantissa.abs(), 1000000);
    }

    #[test]
    fn test_roundtrip_u64() {
        let original: u64 = 0x123456789ABCDEF0;
        let mut ser = Serializer::new();
        ser.add64(original);
        let data = ser.finish();

        let mut iter = SerialIter::new(&data);
        let decoded = iter.get64().unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_account_encoding() {
        let account = AccountID::new([0x00, 0x00, 0x00, 0xAB, 0xCD, 0xEF, 0x12, 0x34,
                                      0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x12, 0x34,
                                      0x56, 0x78, 0x9A, 0xBC]);

        let mut ser = Serializer::new();
        ser.add_account(account);
        let data = ser.finish();

        let mut iter = SerialIter::new(&data);
        let decoded = iter.get_account().unwrap();
        assert_eq!(account.as_bytes(), decoded.as_bytes());
    }
}
