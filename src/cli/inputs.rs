pub(crate) struct IndirectExpenseArgs {
    pub(crate) category: String,
    pub(crate) amount: f64,
    pub(crate) date: String,
    pub(crate) properties: Vec<i64>,
    pub(crate) recurring: bool,
}

impl IndirectExpenseArgs {
    pub(crate) fn new(
        category: String,
        amount: f64,
        date: String,
        properties: Vec<i64>,
        recurring: bool,
    ) -> Self {
        Self {
            category,
            amount,
            date,
            properties,
            recurring,
        }
    }
}

pub(crate) struct AddPropertyInput {
    pub(crate) label: String,
    pub(crate) address: String,
    pub(crate) purchase_date: String,
    pub(crate) purchase_price: f64,
    pub(crate) notes: Option<String>,
}

impl AddPropertyInput {
    pub(crate) fn new(
        label: String,
        address: String,
        purchase_date: String,
        purchase_price: f64,
        notes: Option<String>,
    ) -> Self {
        Self {
            label,
            address,
            purchase_date,
            purchase_price,
            notes,
        }
    }
}

pub(crate) struct AddExpenseInput {
    pub(crate) property_id: i64,
    pub(crate) category: String,
    pub(crate) amount: f64,
    pub(crate) date: String,
    pub(crate) recurring: bool,
}

impl AddExpenseInput {
    pub(crate) fn new(
        property_id: i64,
        category: String,
        amount: f64,
        date: String,
        recurring: bool,
    ) -> Self {
        Self {
            property_id,
            category,
            amount,
            date,
            recurring,
        }
    }
}

pub(crate) struct UpdatePropertyInput {
    pub(crate) property_id: i64,
    pub(crate) label: String,
    pub(crate) address: String,
    pub(crate) purchase_date: String,
    pub(crate) purchase_price: f64,
    pub(crate) notes: Option<String>,
}

impl UpdatePropertyInput {
    pub(crate) fn new(
        property_id: i64,
        label: String,
        address: String,
        purchase_date: String,
        purchase_price: f64,
        notes: Option<String>,
    ) -> Self {
        Self {
            property_id,
            label,
            address,
            purchase_date,
            purchase_price,
            notes,
        }
    }
}

pub(crate) struct UpdateExpenseInput {
    pub(crate) expense_id: i64,
    pub(crate) property_id: i64,
    pub(crate) category: String,
    pub(crate) amount: f64,
    pub(crate) date: String,
    pub(crate) recurring: bool,
}

impl UpdateExpenseInput {
    pub(crate) fn new(
        expense_id: i64,
        property_id: i64,
        category: String,
        amount: f64,
        date: String,
        recurring: bool,
    ) -> Self {
        Self {
            expense_id,
            property_id,
            category,
            amount,
            date,
            recurring,
        }
    }
}
