#[allow(unused)]
#[derive(Hash, PartialEq, Eq)]
pub enum DashboardTable {
    Users,
    ProductGroups,
    Products,
    CustomFields,
    ConfigOptions,
    Servers,
    Networks,
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
            DashboardTable::Networks => "networks",
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

/// An intermediate structure used before converting to the Dashboard's `Server`
/// type. Represents a virtual machine record fetched directly from the WHMCS
/// database, specifically combining data from `tblhosting` and
/// `mod_pvewhmcs_vms`.
///
#[derive(Debug, sqlx::FromRow)]
pub struct VmRecord {
    pub id: u32,
    pub vmid: u32,
    pub node: Option<String>,
    pub hostname: String,
    pub status: String,
}

/// Represents a server entity in the Dashboard application, ready for insertion
/// into the PostgreSQL `servers` table. Typically created by converting a
/// `VmRecord` fetched from the WHMCS database.
///
#[derive(Debug, sqlx::FromRow)]
pub struct Server {
    pub id: i32,
    pub vmid: i32,
    pub node: String,
    pub hostname: String,
    pub status: String,
}

impl From<VmRecord> for Server {
    fn from(vm: VmRecord) -> Self {
        Self {
            id: vm.id as i32,
            vmid: vm.vmid as i32,
            node: vm.node.unwrap_or("pve".to_owned()),
            hostname: vm.hostname,
            status: vm.status.to_lowercase(),
        }
    }
}

/// Represents all necessary fields from the network row in the `MySQL`
/// database.
///
#[derive(Debug, sqlx::FromRow)]
pub struct Network {
    pub id: i32,
    pub title: String,
    pub gateway: String,
    pub mask: String,
}

/// Represents all necessary fields from the ip address row in the `MySQL`
/// database.
///
#[derive(Debug, sqlx::FromRow)]
pub struct IpAddress {
    pub id: i32,
    pub pool_id: i32,
    pub ipaddress: String,
    pub server_id: Option<u32>,
}
