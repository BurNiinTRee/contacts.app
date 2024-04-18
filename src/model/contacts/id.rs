use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

#[derive(Default, Serialize, Deserialize, PartialEq, sqlx::Type, Clone, Copy, Debug)]
#[serde(try_from = "ContactStringId", into = "ContactStringId")]
#[sqlx(transparent)]
pub struct ContactId(pub(super) Uuid);

impl fmt::Display for ContactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Uuid> for ContactId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl FromStr for ContactId {
    type Err = <Uuid as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::from_str(s)?))
    }
}

#[derive(Clone, Deserialize, Serialize, Debug)]
#[serde(transparent)]
pub struct ContactStringId {
    id: String,
}

impl From<ContactId> for ContactStringId {
    fn from(value: ContactId) -> Self {
        Self {
            id: value.0.to_string(),
        }
    }
}
impl TryFrom<ContactStringId> for ContactId {
    type Error = sqlx::types::uuid::Error;

    fn try_from(value: ContactStringId) -> Result<Self, Self::Error> {
        Ok(Self(value.id.parse()?))
    }
}
