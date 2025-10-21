#[allow(unused)]
#[derive(Hash, PartialEq, Eq)]
pub enum DashboardTable {
    Users,
    ProductGroups,
    Products,
    CustomFields,
    ConfigOptions,
    Servers,
    Network,
    IpAddresses,
    Services,
    CustomValues,
    ConfigValues,
}

impl std::fmt::Display for DashboardTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            DashboardTable::Users => "users",
            DashboardTable::ProductGroups => "product_groups",
            DashboardTable::Products => "products",
            DashboardTable::CustomFields => "custom_fields",
            DashboardTable::ConfigOptions => "config_options",
            DashboardTable::Servers => "servers",
            DashboardTable::Network => "network",
            DashboardTable::IpAddresses => "ip_addresses",
            DashboardTable::Services => "services",
            DashboardTable::CustomValues => "custom_values",
            DashboardTable::ConfigValues => "config_values",
        })
    }
}

impl std::fmt::Debug for DashboardTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

/// Represents all necessary fields from the client row in the `MySQL` database.
///
#[derive(Debug, sqlx::FromRow)]
pub struct Client {
    pub id: i32,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub address1: String,
    pub city: String,
    pub state: String,
    pub postcode: String,
    pub country: String,
    pub phonenumber: String,
    pub password: String,
}

/// Represents all necessary fields from the product group row in the `MySQL`
/// database.
///
#[derive(Debug, sqlx::FromRow)]
pub struct ProductGroup {
    pub id: i32,
    pub name: String,
}

/// Represents all necessary fields from the product row in the `MySQL`
/// database.
///
#[derive(Debug, sqlx::FromRow)]
pub struct Product {
    pub id: i32,
    pub gid: i32,
    pub name: String,
}

/// Represents all necessary fields from the custom field row in the `MySQL`
/// database.
///
#[derive(sqlx::FromRow, Debug)]
pub struct CustomField {
    pub id: i32,
    pub fieldname: String,
    pub relid: i32,
}

/// Represents all necessary fields from the configurable option row in the
/// `MySQL` database.
///
#[derive(Debug, sqlx::FromRow)]
pub struct ConfigOption {
    pub id: i32,
    pub optionname: String,
}
