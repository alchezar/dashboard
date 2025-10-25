//! This module is responsible for the "Transform" phase of the migration
//! pipeline.
//!
//! The structs serve as containers for raw, deserialized data from WHMCS, while
//! the associated `impl` blocks contain the core transformation logic.
//! This includes parsing complex fields, converting types, and reshaping the
//! data to fit the target schema before it is passed to the "Load" phase.

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

impl ProductGroup {
    pub fn new(id: i32, name: &str) -> Self {
        Self {
            id,
            name: name.to_owned(),
        }
    }
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

impl Product {
    pub fn new(id: i32, gid: i32, name: &str) -> Self {
        Self {
            id,
            gid,
            name: name.to_owned(),
        }
    }
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

impl CustomField {
    pub fn new(id: i32, fieldname: &str, relid: i32) -> Self {
        Self {
            id,
            fieldname: fieldname.to_owned(),
            relid,
        }
    }
}

/// Represents all necessary fields from the configurable option row in the
/// `MySQL` database.
///
#[derive(Debug, sqlx::FromRow)]
pub struct ConfigOption {
    pub id: i32,
    pub optionname: String,
}

impl ConfigOption {
    pub fn new(id: i32, optionname: &str) -> Self {
        Self {
            id,
            optionname: optionname.to_owned(),
        }
    }
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

impl VmRecord {
    pub fn new(id: u32, vmid: u32, node: Option<&str>, hostname: &str, status: &str) -> Self {
        Self {
            id,
            vmid,
            node: node.map(|n| n.to_owned()),
            hostname: hostname.to_owned(),
            status: status.to_owned(),
        }
    }
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

impl Network {
    pub fn new(id: i32, title: &str, gateway: &str, mask: &str) -> Self {
        Self {
            id,
            title: title.to_owned(),
            gateway: gateway.to_owned(),
            mask: mask.to_owned(),
        }
    }
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

impl IpAddress {
    pub fn new(id: i32, pool_id: i32, ipaddress: &str, server_id: Option<u32>) -> Self {
        Self {
            id,
            pool_id,
            ipaddress: ipaddress.to_owned(),
            server_id,
        }
    }
}

/// Represents a custom field record from WHMCS's `tblcustomfields` table
/// that defines a template.
///
#[derive(Debug, sqlx::FromRow)]
pub struct TemplateField {
    pub relid: i32,
    pub fieldoptions: String,
}

impl TemplateField {
    pub fn new(relid: i32, fieldoptions: &str) -> Self {
        Self {
            relid,
            fieldoptions: fieldoptions.to_owned(),
        }
    }
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
                let name = split.next()?.trim();
                let vmid = split.next()?.trim().parse::<i32>().ok()?;
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

impl Service {
    pub fn new(id: i32, domainstatus: &str, userid: i32, packageid: i32) -> Self {
        Self {
            id,
            domainstatus: domainstatus.to_owned(),
            userid,
            packageid,
        }
    }
}

/// Represents a configurable option value record from WHMCS's
/// `tblhostingconfigoptions` table.
///
#[derive(Debug, sqlx::FromRow)]
pub struct ConfigValue {
    pub id: i32,
    pub relid: i32,
    pub configid: i32,
    pub optionname: String,
}

impl ConfigValue {
    pub fn new(id: i32, relid: i32, configid: i32, optionname: &str) -> Self {
        Self {
            id,
            relid,
            configid,
            optionname: optionname.to_owned(),
        }
    }
}

/// Represents a custom field value record from WHMCS's `tblcustomfieldsvalues`
/// table.
///
#[derive(Debug, sqlx::FromRow)]
pub struct CustomValue {
    pub id: u32,
    pub fieldid: i32,
    pub relid: i32,
    pub value: String,
}

impl CustomValue {
    pub fn new(id: u32, fieldid: i32, relid: i32, value: &str) -> Self {
        Self {
            id,
            fieldid,
            relid,
            value: value.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_field_extract_works() {
        // Arrange
        let template_field = TemplateField {
            relid: 1,
            fieldoptions: "Ubuntu 22.04|9000,CentOS 9|9002,Debian 11|9001".to_string(),
        };
        // Act
        let templates = template_field.extract();
        // Assert
        assert_eq!(templates.len(), 3);
        assert_eq!(templates[0].template_node, "pve");
        assert_eq!(templates[0].virtual_type, "qemu");
        assert_eq!(templates[0].os_name, "Ubuntu 22.04");
        assert_eq!(templates[0].template_vmid, 9000);
        assert_eq!(templates[1].os_name, "CentOS 9");
        assert_eq!(templates[1].template_vmid, 9002);
        assert_eq!(templates[2].os_name, "Debian 11");
        assert_eq!(templates[2].template_vmid, 9001);
    }

    #[test]
    fn vm_record_into_server_with_node_works() {
        // Arrange
        let vm_record = VmRecord {
            id: 1,
            vmid: 100,
            node: Some("pve_node".to_string()),
            hostname: "server1.test.com".to_string(),
            status: "Active".to_string(),
        };
        // Act
        let server: Server = vm_record.into();
        // Assert
        assert_eq!(server.id, 1);
        assert_eq!(server.vmid, 100);
        assert_eq!(server.node, "pve_node");
        assert_eq!(server.hostname, "server1.test.com");
        assert_eq!(server.status, "active");
    }

    #[test]
    fn vm_record_into_server_without_node_works() {
        let vm_record = VmRecord {
            id: 2,
            vmid: 102,
            node: None,
            hostname: "server2.test.com".to_string(),
            status: "Suspended".to_string(),
        };
        let server: Server = vm_record.into();

        assert_eq!(server.id, 2);
        assert_eq!(server.vmid, 102);
        assert_eq!(server.node, "pve");
        assert_eq!(server.hostname, "server2.test.com");
        assert_eq!(server.status, "suspended");
    }
}
