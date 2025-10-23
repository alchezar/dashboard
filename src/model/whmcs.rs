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
    Templates,
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
            DashboardTable::Templates => "templates",
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

/// Intermediate structure used before converting to the Dashboard's `Server`
/// type.
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
/// into the PostgreSQL `servers` table.
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

/// Represents a custom field record from WHMCS's `tblcustomfields` table
/// that defines a template.
///
#[derive(Debug, sqlx::FromRow)]
pub struct TemplateField {
    pub relid: i32,
    pub fieldoptions: String,
}

/// Represents a template record for the Dashboard's `templates` table.
///
#[derive(Debug)]
pub struct Template {
    pub os_name: String,
    pub template_vmid: i32,
    pub template_node: String,
    pub virtual_type: String,
}

impl TemplateField {
    pub fn extract(self) -> Vec<Template> {
        self.fieldoptions
            .split(',')
            .filter_map(|pairs| {
                let mut split = pairs.split('|');
                let name = split.next()?;
                let vmid = split.next()?.parse::<i32>().ok()?;
                Some((name, vmid))
            })
            .map(|(name, vmid)| Template {
                os_name: name.to_owned(),
                template_vmid: vmid.to_owned(),
                template_node: "pve".to_owned(),
                virtual_type: "qemu".to_owned(),
            })
            .collect()
    }
}

/// Represents a service record fetched from WHMCS's `tblhosting` table.
///
#[derive(Debug, sqlx::FromRow)]
pub struct Service {
    pub id: i32,
    pub domainstatus: String,
    pub userid: i32,
    pub packageid: i32,
}
