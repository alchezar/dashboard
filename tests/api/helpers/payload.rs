use serde_json::{Value, json};
use uuid::Uuid;

pub fn register_user() -> Value {
    json!({
        "first_name": "John",
        "last_name": "Doe",
        "email": "john.doe.reqwest@example.com",
        "password": "secure_password_123",
        "address": "123 Main St",
        "city": "Anytown",
        "state": "Any-state",
        "post_code": "12345",
        "country": "USA",
        "phone_number": "555-1234"
    })
}

pub fn login_user() -> Value {
    json!({
        "email": "john.doe.reqwest@example.com",
        "password": "secure_password_123",
    })
}

pub fn new_server(product_id: Uuid) -> Value {
    json!({
        "product_id": product_id,
        "host_name": "test-server.example.com",
        "cpu_cores": 2,
        "ram_gb": 2,
        "os": "ubuntu-22.04",
        "data_center": "Amsterdam"
    })
}
