use rusqlite::{Result as SqlResult, Row};

#[derive(Debug, Clone)]
pub struct Tenant {
    pub id: Option<i64>,
    pub name: String,
    pub contact: Option<String>,
}

impl Tenant {
    pub fn new(name: String, contact: Option<String>) -> Self {
        Tenant {
            id: None,
            name,
            contact,
        }
    }

    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(Tenant {
            id: Some(row.get("id")?),
            name: row.get("name")?,
            contact: row.get("contact")?,
        })
    }
}
