//! Bounded AMF0 values needed by RTMP connect/publish/play commands.
use crate::{Error, Message, Result, COMMAND_AMF0, COMMAND_AMF3};

const MAX_DEPTH: usize = 32;
const MAX_VALUES: usize = 100_000;
const MAX_BYTES: usize = 16 * 1024 * 1024 - 1;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Boolean(bool),
    String(String),
    Object(Vec<(String, Value)>),
    Null,
    Undefined,
    EcmaArray(Vec<(String, Value)>),
    StrictArray(Vec<Value>),
    Date { milliseconds: f64, timezone: i16 },
}

impl Value {
    pub fn object<const N: usize>(fields: [(&str, Value); N]) -> Self {
        Self::Object(
            fields
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }
    pub fn as_str(&self) -> Option<&str> {
        if let Self::String(value) = self {
            Some(value)
        } else {
            None
        }
    }
    pub fn as_number(&self) -> Option<f64> {
        if let Self::Number(value) = self {
            Some(*value)
        } else {
            None
        }
    }
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Self::Object(fields) | Self::EcmaArray(fields) => fields
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    pub name: String,
    pub transaction_id: f64,
    pub arguments: Vec<Value>,
}

/// Accepts AMF0 commands and AMF3 RTMP commands with the AMF0 switch byte.
/// Native AMF3 object encoding is explicitly unsupported rather than misparsed.
pub fn command(message: &Message) -> Result<Command> {
    let payload = match message.type_id {
        COMMAND_AMF0 => &message.payload[..],
        COMMAND_AMF3 if message.payload.first() == Some(&0) => &message.payload[1..],
        _ => return Err(Error::Amf("message is not an AMF0 command")),
    };
    let mut values = decode(payload)?.into_iter();
    let name = match values.next() {
        Some(Value::String(value)) => value,
        _ => return Err(Error::Amf("command name")),
    };
    let transaction_id = match values.next() {
        Some(Value::Number(value)) if value.is_finite() => value,
        _ => return Err(Error::Amf("transaction id")),
    };
    Ok(Command {
        name,
        transaction_id,
        arguments: values.collect(),
    })
}

pub fn decode(bytes: &[u8]) -> Result<Vec<Value>> {
    if bytes.len() > MAX_BYTES {
        return Err(Error::Limit("AMF0 input"));
    }
    let mut reader = Reader {
        bytes,
        pos: 0,
        values: 0,
    };
    let mut result = Vec::new();
    while reader.pos < bytes.len() {
        result.push(reader.value(0)?);
    }
    Ok(result)
}

pub fn encode(values: &[Value]) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut count = 0;
    for value in values {
        write_value(value, &mut bytes, 0, &mut count)?;
    }
    Ok(bytes)
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    values: usize,
}
impl Reader<'_> {
    fn take(&mut self, count: usize) -> Result<&[u8]> {
        let end = self
            .pos
            .checked_add(count)
            .ok_or(Error::Amf("length overflow"))?;
        if end > self.bytes.len() {
            return Err(Error::Amf("truncated value"));
        }
        let result = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(result)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn string(&mut self, length: usize) -> Result<String> {
        String::from_utf8(self.take(length)?.to_vec()).map_err(|_| Error::Amf("invalid UTF-8"))
    }
    fn fields(&mut self, depth: usize) -> Result<Vec<(String, Value)>> {
        let mut fields = Vec::new();
        loop {
            let length = self.u16()? as usize;
            if length == 0 && self.bytes.get(self.pos) == Some(&9) {
                self.pos += 1;
                break;
            }
            let name = self.string(length)?;
            let value = self.value(depth + 1)?;
            fields.push((name, value));
        }
        Ok(fields)
    }
    fn value(&mut self, depth: usize) -> Result<Value> {
        self.values += 1;
        if depth > MAX_DEPTH || self.values > MAX_VALUES {
            return Err(Error::Limit("AMF0 nesting/value count"));
        }
        Ok(match self.byte()? {
            0 => Value::Number(f64::from_be_bytes(self.take(8)?.try_into().unwrap())),
            1 => match self.byte()? {
                0 => Value::Boolean(false),
                1 => Value::Boolean(true),
                _ => return Err(Error::Amf("boolean")),
            },
            2 => {
                let len = self.u16()? as usize;
                Value::String(self.string(len)?)
            }
            3 => Value::Object(self.fields(depth)?),
            5 => Value::Null,
            6 => Value::Undefined,
            8 => {
                self.u32()?;
                Value::EcmaArray(self.fields(depth)?)
            }
            10 => {
                let count = self.u32()? as usize;
                if count > MAX_VALUES - self.values {
                    return Err(Error::Limit("AMF0 array"));
                }
                let mut values = Vec::with_capacity(count.min(1024));
                for _ in 0..count {
                    values.push(self.value(depth + 1)?);
                }
                Value::StrictArray(values)
            }
            11 => {
                let milliseconds = f64::from_be_bytes(self.take(8)?.try_into().unwrap());
                let timezone = i16::from_be_bytes(self.take(2)?.try_into().unwrap());
                Value::Date {
                    milliseconds,
                    timezone,
                }
            }
            12 => {
                let len = self.u32()? as usize;
                Value::String(self.string(len)?)
            }
            _ => return Err(Error::Amf("unsupported type marker")),
        })
    }
}

fn write_string(bytes: &mut Vec<u8>, text: &str, short: bool) -> Result<()> {
    if short {
        let len = u16::try_from(text.len()).map_err(|_| Error::Limit("AMF0 property name"))?;
        bytes.extend_from_slice(&len.to_be_bytes());
    } else {
        if text.len() > MAX_BYTES {
            return Err(Error::Limit("AMF0 string"));
        }
        bytes.extend_from_slice(&(text.len() as u32).to_be_bytes());
    }
    bytes.extend_from_slice(text.as_bytes());
    Ok(())
}
fn write_fields(
    fields: &[(String, Value)],
    bytes: &mut Vec<u8>,
    depth: usize,
    count: &mut usize,
) -> Result<()> {
    for (key, value) in fields {
        write_string(bytes, key, true)?;
        write_value(value, bytes, depth + 1, count)?;
    }
    bytes.extend_from_slice(&[0, 0, 9]);
    Ok(())
}
fn write_value(value: &Value, bytes: &mut Vec<u8>, depth: usize, count: &mut usize) -> Result<()> {
    *count += 1;
    if depth > MAX_DEPTH || *count > MAX_VALUES {
        return Err(Error::Limit("AMF0 nesting/value count"));
    }
    match value {
        Value::Number(value) => {
            bytes.push(0);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        Value::Boolean(value) => {
            bytes.extend_from_slice(&[1, u8::from(*value)]);
        }
        Value::String(value) => {
            let short = value.len() <= u16::MAX as usize;
            bytes.push(if short { 2 } else { 12 });
            write_string(bytes, value, short)?;
        }
        Value::Object(fields) => {
            bytes.push(3);
            write_fields(fields, bytes, depth, count)?;
        }
        Value::EcmaArray(fields) => {
            bytes.push(8);
            bytes.extend_from_slice(&(fields.len() as u32).to_be_bytes());
            write_fields(fields, bytes, depth, count)?;
        }
        Value::Null => bytes.push(5),
        Value::Undefined => bytes.push(6),
        Value::StrictArray(values) => {
            bytes.push(10);
            bytes.extend_from_slice(&(values.len() as u32).to_be_bytes());
            for value in values {
                write_value(value, bytes, depth + 1, count)?;
            }
        }
        Value::Date {
            milliseconds,
            timezone,
        } => {
            bytes.push(11);
            bytes.extend_from_slice(&milliseconds.to_be_bytes());
            bytes.extend_from_slice(&timezone.to_be_bytes());
        }
    }
    if bytes.len() > MAX_BYTES {
        return Err(Error::Limit("AMF0 output"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_connect_command_round_trip_over_chunk_boundary() {
        let value = crate::command_message(
            "connect",
            1.0,
            0,
            vec![Value::object([
                ("app", Value::String("live".into())),
                ("tcUrl", Value::String("rtmp://127.0.0.1:1935/live".into())),
                (
                    "flashVer",
                    Value::String("FMLE/3.0 (compatible; Open-Delay test)".into()),
                ),
                ("fpad", Value::Boolean(false)),
                ("objectEncoding", Value::Number(0.0)),
            ])],
        )
        .unwrap();
        assert!(value.payload.len() > 128);
        let mut decoder = crate::ChunkDecoder::new();
        decoder
            .feed(&crate::encode_message(&value, 128).unwrap())
            .unwrap();
        let result = command(&decoder.next_message().unwrap().unwrap()).unwrap();
        assert_eq!(result.name, "connect");
        assert_eq!(result.transaction_id, 1.0);
        assert_eq!(
            result.arguments[0].get("app").and_then(Value::as_str),
            Some("live")
        );
    }
    #[test]
    fn nested_arrays_empty_property_and_long_strings_round_trip() {
        let values = vec![
            Value::Object(vec![("".into(), Value::Boolean(true))]),
            Value::EcmaArray(vec![(
                "a".into(),
                Value::StrictArray(vec![Value::Null, Value::Number(4.0)]),
            )]),
            Value::Date {
                milliseconds: 25.0,
                timezone: -60,
            },
            Value::String("x".repeat(70000)),
        ];
        assert_eq!(decode(&encode(&values).unwrap()).unwrap(), values);
    }
    #[test]
    fn truncation_and_excessive_nesting_are_rejected() {
        assert!(decode(&[2, 0, 5, b'a']).is_err());
        let bytes = vec![3; 100];
        assert!(decode(&bytes).is_err());
        let mut value = Value::Null;
        for _ in 0..40 {
            value = Value::StrictArray(vec![value]);
        }
        assert!(encode(&[value]).is_err());
        assert!(decode(&[10, 0xff, 0xff, 0xff, 0xff]).is_err());
    }
}
