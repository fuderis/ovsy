use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use macron::{Display, Error};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha512_256};
use std::{num::ParseIntError, str::FromStr};

/// The session ID wrapper
#[derive(Default, Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[display(fmt = "{user_id}-{timezone}-{timestamp}-{salt}")]
pub struct SessionID {
    pub user_id: u128,
    pub timezone: i16,
    pub timestamp: u128,
    pub salt: u16,
}

impl SessionID {
    /// Creates a new session ID
    pub fn new(user_id: u128, timezone: i16) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis();

        let salt = rand::random();

        Self {
            timestamp,
            timezone,
            user_id,
            salt,
        }
    }

    /// Get the session creation time
    pub fn created_at(&self) -> DateTime<Utc> {
        let secs = (self.timestamp / 1_000) as i64;
        let nsecs = ((self.timestamp % 1_000) * 1_000_000) as u32;

        Utc.timestamp_opt(secs, nsecs)
            .single()
            .unwrap_or_else(Utc::now)
    }

    /// Generates a cryptographically secure SHA-256 hash
    pub fn to_hash(&self) -> String {
        let hash = Sha512_256::digest(self.to_string());
        hash.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Returns the session local date time
    pub fn now_local(&self) -> DateTime<FixedOffset> {
        let offset_seconds = (self.timezone as i32) * 60;
        let tz = FixedOffset::east_opt(offset_seconds)
            .unwrap_or_else(|| FixedOffset::east_opt(0).unwrap());

        let utc_now = Utc::now();
        utc_now.with_timezone(&tz)
    }
}

impl FromStr for SessionID {
    type Err = SessionIDError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('-');

        let user_id_str = parts.next().ok_or(SessionIDError::InvalidFormat)?;
        let timezone_str = parts.next().ok_or(SessionIDError::InvalidFormat)?;
        let timestamp_str = parts.next().ok_or(SessionIDError::InvalidFormat)?;
        let salt_str = parts.next().ok_or(SessionIDError::InvalidFormat)?;

        if parts.next().is_some() {
            return Err(SessionIDError::InvalidFormat);
        }

        let user_id = user_id_str
            .parse::<u128>()
            .map_err(SessionIDError::InvalidUserId)?;
        let timezone = timezone_str
            .parse::<i16>()
            .map_err(SessionIDError::InvalidUserId)?;
        let timestamp = timestamp_str
            .parse::<u128>()
            .map_err(SessionIDError::InvalidTimestamp)?;
        let salt = salt_str
            .parse::<u16>()
            .map_err(SessionIDError::InvalidSalt)?;

        Ok(SessionID {
            user_id,
            timezone,
            timestamp,
            salt,
        })
    }
}

impl Serialize for SessionID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SessionID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }
}

/// The session id parsing error
#[derive(Debug, Display, Error)]
pub enum SessionIDError {
    InvalidFormat,
    InvalidUserId(ParseIntError),
    InvalidTimestamp(ParseIntError),
    InvalidSalt(ParseIntError),
}
